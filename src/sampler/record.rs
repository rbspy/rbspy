use anyhow::{Context, Error, Result};
use std::collections::HashSet;
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::core::initialize::initialize;
use crate::core::types::{MemoryCopyError, Pid, Process, ProcessRetry, StackTrace};
use crate::storage::Store;
use crate::ui::summary;

const BILLION: u64 = 1000 * 1000 * 1000; // for nanosleep

// This SampleTime struct helps us sample on a regular schedule ("exactly" 100 times per second, if
// the sample rate is 100).
// What we do is -- when doing the 1234th sample, we calculate the exact time the 1234th sample
// should happen at, which is (start time + nanos_between_samples * 1234) and then sleep until that
// time
struct SampleTime {
    start_time: Instant,
    nanos_between_samples: u64,
    num_samples: u64,
}

impl SampleTime {
    pub fn new(rate: u32) -> SampleTime {
        SampleTime {
            start_time: Instant::now(),
            nanos_between_samples: BILLION / u64::from(rate),
            num_samples: 0,
        }
    }

    pub fn get_sleep_time(&mut self) -> Result<u32, u32> {
        // Returns either the amount of time to sleep (Ok(x)) until next sample time or an error of
        // how far we're behind if we're behind the expected next sample time
        self.num_samples += 1;
        let elapsed = self.start_time.elapsed();
        let nanos_elapsed = elapsed.as_secs() * BILLION + u64::from(elapsed.subsec_nanos());
        let target_elapsed = self.num_samples * self.nanos_between_samples;
        if target_elapsed < nanos_elapsed {
            Err((nanos_elapsed - target_elapsed) as u32)
        } else {
            Ok((target_elapsed - nanos_elapsed) as u32)
        }
    }
}

/// Start thread(s) recording a PID and possibly its children. Tracks new processes
/// Returns a pair of Receivers from which you can consume recorded stacktraces and errors
fn spawn_recorder_children(
    root_pid: Pid,
    with_subprocesses: bool,
    sample_rate: u32,
    maybe_stop_time: Option<Instant>,
    lock_process: bool,
) -> (
    Receiver<StackTrace>,
    Receiver<Result<(), Error>>,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
) {
    let done = Arc::new(AtomicBool::new(false));
    let total_traces = Arc::new(AtomicUsize::new(0));
    let timing_error_traces = Arc::new(AtomicUsize::new(0));
    let total_traces_clone = total_traces.clone();
    let timing_error_traces_clone = timing_error_traces.clone();

    // Set up the Ctrl+C handler + the done mutex that we send to each recorder so that it knows
    // when to stop
    let done_clone = done.clone();
    ctrlc::set_handler(move || {
        if done_clone.load(Ordering::Relaxed) {
            eprintln!("Multiple interrupts received, exiting with haste!");
            std::process::exit(1);
        }
        eprintln!("Interrupted.");
        // Trigger the end of the loop
        done_clone.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    eprintln!("Press Ctrl+C to stop");

    // Create the sender/receiver channels and start the child threads off collecting stack traces
    // from each target process.
    // Give the child threads a buffer in case we fall a little behind with aggregating the stack
    // traces, but not an unbounded buffer.
    let (trace_sender, trace_receiver) = sync_channel(100);
    let (error_sender, result_receiver) = channel();

    if with_subprocesses {
        // Start a thread which watches for new descendents and starts new recorders when they
        // appear
        let done_clone = done.clone();
        std::thread::spawn(move || {
            let process = Process::new_with_retry(root_pid).unwrap();
            let mut pids: HashSet<Pid> = HashSet::new();
            let done = done.clone();
            // we need to exit this loop when the process we're monitoring exits, otherwise the
            // sender channels won't get closed and rbspy will hang. So we check the done
            // mutex.
            while !done_clone.load(Ordering::Relaxed) {
                let mut descendents: Vec<Pid> = process
                    .child_processes()
                    .expect("Error finding descendents of pid")
                    .into_iter()
                    .map(|tuple| tuple.0)
                    .collect();
                descendents.push(root_pid);

                for pid in descendents {
                    if pids.contains(&pid) {
                        // already recording it, no need to start a new recording thread
                        continue;
                    }
                    pids.insert(pid);
                    let trace_sender = trace_sender.clone();
                    let error_sender = error_sender.clone();
                    let done_root = done.clone();
                    let done_thread = done.clone();
                    let timing_error_traces = timing_error_traces.clone();
                    let total_traces = total_traces.clone();
                    std::thread::spawn(move || {
                        let result = record(
                            pid,
                            sample_rate,
                            maybe_stop_time,
                            done_thread,
                            timing_error_traces,
                            total_traces,
                            trace_sender,
                            lock_process,
                        );
                        error_sender.send(result).expect("couldn't send error");
                        drop(error_sender);

                        if pid == root_pid {
                            debug!("Root process {} ended", pid);
                            // we need to store done = true here to signal the other threads here that we
                            // should stop profiling
                            done_root.store(true, Ordering::Relaxed);
                        }
                    });
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        });
    } else {
        // Start a single recorder thread
        std::thread::spawn(move || {
            let result = record(
                root_pid,
                sample_rate,
                maybe_stop_time,
                done,
                timing_error_traces,
                total_traces,
                trace_sender,
                lock_process,
            );
            error_sender.send(result).unwrap();
            drop(error_sender);
        });
    }
    (
        trace_receiver,
        result_receiver,
        total_traces_clone,
        timing_error_traces_clone,
    )
}

// TODO: Find a more reliable way to test this on Windows hosts
#[cfg(not(target_os = "windows"))]
#[test]
fn test_spawn_record_children_subprocesses() {
    #[cfg(target_os = "macos")]
    if !nix::unistd::Uid::effective().is_root() {
        println!("Skipping test because we're not running as root");
        return;
    }

    let which = if cfg!(target_os = "windows") {
        "C:\\Windows\\System32\\WHERE.exe"
    } else {
        "/usr/bin/which"
    };

    let output = std::process::Command::new(which)
        .arg("ruby")
        .output()
        .expect("failed to execute process");

    let ruby_binary_path = String::from_utf8(output.stdout).unwrap();

    let ruby_binary_path_str = ruby_binary_path
        .lines()
        .next()
        .expect("failed to execute ruby process");

    let coordination_dir = tempdir::TempDir::new("").unwrap();
    let coordination_dir_name = coordination_dir.path().to_str().unwrap();

    let mut process = std::process::Command::new(ruby_binary_path_str)
        .arg("ci/ruby-programs/ruby_forks.rb")
        .arg(coordination_dir_name)
        .spawn()
        .unwrap();

    let pid = process.id() as Pid;

    let (trace_receiver, result_receiver, _, _) = spawn_recorder_children(pid, true, 5, None, true);

    let mut pids = HashSet::<Pid>::new();
    for trace in &trace_receiver {
        let pid = trace.pid.unwrap();
        if !pids.contains(&pid) {
            // Now that we have a stack trace for this PID, signal to the corresponding
            // ruby process that it can exit
            let coordination_filename = format!("rbspy_ack.{}", pid);
            File::create(coordination_dir.path().join(coordination_filename.clone()))
                .expect("couldn't create coordination file");
            pids.insert(pid);
        }

        if pids.len() == 4 {
            break;
        }
    }

    let results: Vec<_> = result_receiver.iter().take(4).collect();
    for r in results {
        r.expect("unexpected error");
    }

    drop(trace_receiver);

    assert_eq!(pids.len(), 4);
    process.wait().unwrap();
}

pub struct Config {
    pub format: crate::core::types::OutputFormat,
    pub raw_path: PathBuf,
    pub out_path: PathBuf,
    pub pid: Pid,
    pub with_subprocesses: bool,
    pub silent: bool,
    pub sample_rate: u32,
    pub maybe_duration: Option<std::time::Duration>,
    pub flame_min_width: f64,
    pub lock_process: bool,
}

pub fn parallel_record(config: Config) -> Result<(), Error> {
    let maybe_stop_time = match config.maybe_duration {
        Some(duration) => Some(std::time::Instant::now() + duration),
        None => None,
    };

    let (trace_receiver, result_receiver, total_traces, timing_error_traces) =
        spawn_recorder_children(
            config.pid,
            config.with_subprocesses,
            config.sample_rate,
            maybe_stop_time,
            config.lock_process,
        );

    // Aggregate stack traces as we receive them from the threads that are collecting them
    // Aggregate to 3 places: the raw output (`.raw.gz`), some summary statistics we display live,
    // and the formatted output (a flamegraph or something)
    let mut out = config.format.outputter(config.flame_min_width);
    let mut summary_out = summary::Stats::new();
    let mut raw_store = Store::new(&config.raw_path, config.sample_rate)?;
    let mut summary_time = std::time::Instant::now() + Duration::from_secs(1);
    let start_time = Instant::now();

    for trace in trace_receiver.iter() {
        out.record(&trace)?;
        summary_out.add_function_name(&trace.trace);
        raw_store.write(&trace)?;

        if !config.silent {
            // Print a summary every second
            if std::time::Instant::now() > summary_time {
                print_summary(
                    &summary_out,
                    &start_time,
                    config.sample_rate,
                    timing_error_traces.load(Ordering::Relaxed),
                    total_traces.load(Ordering::Relaxed),
                )?;
                summary_time = std::time::Instant::now() + Duration::from_secs(1);
            }
        }
    }

    // Finish writing all data to disk
    if config.out_path.display().to_string() == "-" {
        out.complete(&mut std::io::stdout())?;
    } else {
        let mut out_file = File::create(&config.out_path).context(format!(
            "Failed to create output file {}",
            &config.out_path.display()
        ))?;
        out.complete(&mut out_file)?;
    }
    raw_store.complete();

    // Check for errors from the child threads. Ignore errors unless every single thread
    // returned an error. If that happens, return the last error. This lets rbspy successfully
    // record processes even if the parent thread isn't a Ruby process.
    let mut num_ok = 0;
    let mut last_result = Ok(());
    for result in result_receiver.iter() {
        if result.is_ok() {
            num_ok += 1;
        }
        last_result = result;
    }

    match num_ok {
        0 => last_result,
        _ => Ok(()),
    }
}

/// Records stack traces and sends them to a channel in another thread where they can be aggregated
fn record(
    pid: Pid,
    sample_rate: u32,
    maybe_stop_time: Option<Instant>,
    done: Arc<AtomicBool>,
    timing_error_traces: Arc<AtomicUsize>,
    total_traces: Arc<AtomicUsize>,
    sender: SyncSender<StackTrace>,
    lock_process: bool,
) -> Result<(), Error> {
    let mut getter = initialize(pid, lock_process)?;

    let mut total = 0;
    let mut errors = 0;

    let mut sample_time = SampleTime::new(sample_rate);
    #[cfg(windows)]
    {
        // This changes a system-wide setting on Windows so that the OS wakes up every 1ms
        // instead of the default 15.6ms. This is required to have a sleep call
        // take less than 15ms, which we need since we usually profile at more than 64hz.
        // The downside is that this will increase power usage: good discussions are:
        // https://randomascii.wordpress.com/2013/07/08/windows-timer-resolution-megawatts-wasted/
        // and http://www.belshe.com/2010/06/04/chrome-cranking-up-the-clock/
        unsafe {
            timeapi::timeBeginPeriod(1);
        }
    }

    while !done.load(Ordering::Relaxed) {
        total += 1;
        let trace = getter.get_trace();
        match trace {
            Ok(ok_trace) => {
                sender.send(ok_trace)?;
            }
            Err(x) => {
                if let Some(MemoryCopyError::ProcessEnded) = x.downcast_ref() {
                    debug!("Process {} ended", pid);
                    return Ok(());
                }

                errors += 1;
                if errors > 20 && (errors as f64) / (total as f64) > 0.5 {
                    print_errors(errors, total);
                    return Err(x);
                }
            }
        }
        if let Some(stop_time) = maybe_stop_time {
            if std::time::Instant::now() > stop_time {
                // need to store done for same reason as above
                done.store(true, Ordering::Relaxed);
                break;
            }
        }
        // Sleep until the next expected sample time
        total_traces.fetch_add(1, Ordering::Relaxed);
        match sample_time.get_sleep_time() {
            Ok(sleep_time) => {
                std::thread::sleep(std::time::Duration::new(0, sleep_time));
            }
            Err(_) => {
                timing_error_traces.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    // reset time period calls
    #[cfg(windows)]
    {
        unsafe {
            timeapi::timeEndPeriod(1);
        }
    }
    Ok(())
}

fn print_summary(
    summary_out: &crate::ui::summary::Stats,
    start_time: &Instant,
    sample_rate: u32,
    timing_error_traces: usize,
    total_traces: usize,
) -> Result<(), Error> {
    let width = match term_size::dimensions() {
        Some((w, _)) => Some(w as usize),
        None => None,
    };
    println!("{}[2J", 27 as char); // clear screen
    println!("{}[0;0H", 27 as char); // go to 0,0
    eprintln!(
        "Time since start: {}s. Press Ctrl+C to stop.",
        start_time.elapsed().as_secs()
    );
    let percent_timing_error = (timing_error_traces as f64) / (total_traces as f64) * 100.0;
    eprintln!("Summary of profiling data so far:");
    summary_out.print_top_n(20, width)?;

    if total_traces > 100 && percent_timing_error > 0.5 {
        // Only print if timing errors are more than 0.5% of total traces -- it's a statistical
        // profiler so smaller differences don't really matter
        eprintln!("{:.1}% ({}/{}) of stack traces were sampled late because we couldn't sample at expected rate, results may be inaccurate. Current rate: {}. Try sampling at a lower rate with `--rate`.", percent_timing_error, timing_error_traces, total_traces, sample_rate);
    }
    Ok(())
}

fn print_errors(errors: usize, total: usize) {
    if errors > 0 {
        eprintln!(
            "Dropped {}/{} stack traces because of errors. Please consider reporting a GitHub issue -- this isn't normal.",
            errors,
            total
        );
    }
}
