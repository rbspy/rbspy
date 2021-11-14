use anyhow::{Context, Error, Result};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};
#[cfg(windows)]
use winapi::um::timeapi;

use crate::core::initialize::initialize;
use crate::core::process::{Pid, Process, ProcessRetry};
use crate::core::types::{MemoryCopyError, StackTrace};

#[derive(Debug)]
pub struct Sampler {
    done: Arc<AtomicBool>,
    lock_process: bool,
    root_pid: Pid,
    sample_rate: u32,
    time_limit: Option<Duration>,
    timing_error_traces: Arc<AtomicUsize>,
    total_traces: Arc<AtomicUsize>,
    with_subprocesses: bool,
}

impl Sampler {
    pub fn new(
        pid: Pid,
        sample_rate: u32,
        lock_process: bool,
        time_limit: Option<Duration>,
        with_subprocesses: bool,
    ) -> Self {
        Sampler {
            done: Arc::new(AtomicBool::new(false)),
            lock_process,
            root_pid: pid,
            sample_rate,
            time_limit,
            timing_error_traces: Arc::new(AtomicUsize::new(0)),
            total_traces: Arc::new(AtomicUsize::new(0)),
            with_subprocesses,
        }
    }

    pub fn total_traces(&self) -> usize {
        self.total_traces.load(Ordering::Relaxed)
    }

    pub fn timing_error_traces(&self) -> usize {
        self.timing_error_traces.load(Ordering::Relaxed)
    }

    /// Start thread(s) recording a PID and possibly its children. Tracks new processes
    /// Returns a pair of Receivers from which you can consume recorded stacktraces and errors
    pub fn start(
        &self,
        trace_sender: SyncSender<StackTrace>,
        result_sender: Sender<Result<(), Error>>,
    ) -> Result<(), Error> {
        let done = self.done.clone();
        let root_pid = self.root_pid.clone();
        let sample_rate = self.sample_rate.clone();
        let maybe_stop_time = match self.time_limit {
            Some(duration) => Some(std::time::Instant::now() + duration),
            None => None,
        };
        let lock_process = self.lock_process.clone();
        let result_sender = result_sender.clone();
        let timing_error_traces = self.timing_error_traces.clone();
        let total_traces = self.total_traces.clone();

        if self.with_subprocesses {
            // Start a thread which watches for new descendents and starts new recorders when they
            // appear
            let done_clone = self.done.clone();
            std::thread::spawn(move || {
                let process = Process::new_with_retry(root_pid).unwrap();
                let mut pids: HashSet<Pid> = HashSet::new();
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
                        let done_root = done.clone();
                        let done_thread = done.clone();
                        let result_sender = result_sender.clone();
                        let timing_error_traces = timing_error_traces.clone();
                        let total_traces = total_traces.clone();
                        let trace_sender_clone = trace_sender.clone();
                        std::thread::spawn(move || {
                            let result = sample(
                                pid,
                                sample_rate,
                                maybe_stop_time,
                                done_thread,
                                timing_error_traces,
                                total_traces,
                                trace_sender_clone,
                                lock_process,
                            );
                            result_sender.send(result).expect("couldn't send error");
                            drop(result_sender);

                            if pid == root_pid {
                                debug!("Root process {} ended", pid);
                                // we need to store done = true here to signal the other threads here that we
                                // should stop profiling
                                done_root.store(true, Ordering::Relaxed);
                            }
                        });
                    }
                    // TODO: Parameterize subprocess check interval
                    std::thread::sleep(Duration::from_secs(1));
                }
            });
        } else {
            // Start a single recorder thread
            std::thread::spawn(move || {
                let result = sample(
                    root_pid,
                    sample_rate,
                    maybe_stop_time,
                    done,
                    timing_error_traces,
                    total_traces,
                    trace_sender,
                    lock_process,
                );
                result_sender.send(result).unwrap();
                drop(result_sender);
            });
        }

        return Ok(());
    }

    pub fn stop(&self) {
        self.done.store(true, Ordering::Relaxed);
    }
}

/// Samples stack traces and sends them to a channel in another thread where they can be aggregated
fn sample(
    pid: Pid,
    sample_rate: u32,
    maybe_stop_time: Option<Instant>,
    done: Arc<AtomicBool>,
    timing_error_traces: Arc<AtomicUsize>,
    total_traces: Arc<AtomicUsize>,
    sender: SyncSender<StackTrace>,
    lock_process: bool,
) -> Result<(), Error> {
    let mut getter = initialize(pid, lock_process).context("initialize")?;

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
                sender.send(ok_trace).context("send trace")?;
            }
            Err(e) => {
                if let Some(MemoryCopyError::ProcessEnded) = e.downcast_ref() {
                    debug!("Process {} ended", pid);
                    return Ok(());
                }

                errors += 1;
                if errors > 20 && (errors as f64) / (total as f64) > 0.5 {
                    // TODO: Return error type instead of printing here
                    print_errors(errors, total);
                    return Err(e);
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

fn print_errors(errors: usize, total: usize) {
    if errors > 0 {
        eprintln!(
            "Dropped {}/{} stack traces because of errors. Please consider reporting a GitHub issue -- this isn't normal.",
            errors,
            total
        );
    }
}

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

const BILLION: u64 = 1000 * 1000 * 1000; // for nanosleep

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

#[cfg(test)]
mod tests {
    #[cfg(not(target_os = "windows"))]
    use std::collections::HashSet;
    #[cfg(unix)]
    use std::process::Command;

    use crate::core::process::{tests::RubyScript, Pid};
    use crate::sampler::Sampler;

    #[test]
    fn test_sample_single_process() {
        #[cfg(target_os = "macos")]
        if !nix::unistd::Uid::effective().is_root() {
            println!("Skipping test because we're not running as root");
            return;
        }

        let mut process = RubyScript::new("ci/ruby-programs/infinite.rb");
        let pid = process.id() as Pid;

        let sampler = Sampler::new(pid, 100, true, None, false);
        let (trace_sender, trace_receiver) = std::sync::mpsc::sync_channel(100);
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        sampler
            .start(trace_sender, result_sender)
            .expect("sampler failed to start");

        let trace = trace_receiver.recv().expect("failed to receive trace");
        assert_eq!(trace.pid.unwrap(), pid);

        process.kill().expect("failed to kill process");

        let result = result_receiver.recv().expect("failed to receive result");
        result.expect("unexpected error");
    }

    #[test]
    fn test_sample_single_process_with_time_limit() {
        #[cfg(target_os = "macos")]
        if !nix::unistd::Uid::effective().is_root() {
            println!("Skipping test because we're not running as root");
            return;
        }

        let mut process = RubyScript::new("ci/ruby-programs/infinite.rb");
        let pid = process.id() as Pid;

        let sampler = Sampler::new(
            pid,
            100,
            true,
            Some(std::time::Duration::from_millis(500)),
            false,
        );
        let (trace_sender, trace_receiver) = std::sync::mpsc::sync_channel(100);
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        sampler
            .start(trace_sender, result_sender)
            .expect("sampler failed to start");

        for trace in trace_receiver {
            assert_eq!(trace.pid.unwrap(), pid);
        }

        // At this point the sampler has halted, so we can kill the process
        process.kill().expect("failed to kill process");

        let result = result_receiver.recv().expect("failed to receive result");
        result.expect("unexpected error");
    }

    // TODO: Find a more reliable way to test this on Windows hosts
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_sample_subprocesses() {
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

        let output = Command::new(which)
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

        let mut process = Command::new(ruby_binary_path_str)
            .arg("ci/ruby-programs/ruby_forks.rb")
            .arg(coordination_dir_name)
            .spawn()
            .unwrap();
        let pid = process.id() as Pid;

        let sampler = Sampler::new(pid, 5, true, None, true);
        let (trace_sender, trace_receiver) = std::sync::mpsc::sync_channel(100);
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        sampler
            .start(trace_sender, result_sender)
            .expect("sampler failed to start");

        let mut pids = HashSet::<Pid>::new();
        for trace in trace_receiver {
            let pid = trace.pid.unwrap();
            if !pids.contains(&pid) {
                // Now that we have a stack trace for this PID, signal to the corresponding
                // ruby process that it can exit
                let coordination_filename = format!("rbspy_ack.{}", pid);
                std::fs::File::create(coordination_dir.path().join(coordination_filename.clone()))
                    .expect("couldn't create coordination file");
                pids.insert(pid);
            }
        }

        let _ = process.wait();

        let results: Vec<_> = result_receiver.iter().take(4).collect();
        for r in results {
            r.expect("unexpected error");
        }

        assert_eq!(pids.len(), 4);
    }
}
