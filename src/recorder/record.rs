use anyhow::{Context, Error, Result};
use std::fs::File;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::storage::Store;
use crate::ui::summary;

/// A configuration bundle for the recorder
pub struct Config {
    /// The format to use for recorded traces. See `OutputFormat` for a list of available options.
    pub format: crate::core::types::OutputFormat,
    /// Where to write rbspy's raw trace output, which can be used for later processing.
    pub raw_path: PathBuf,
    /// Where to write rbspy's output. If `-` is given, output is written to standard output.
    pub out_path: PathBuf,
    /// The process ID (PID) of the process to profile. This is usually a ruby process, but rbspy
    /// will locate and profile any ruby subprocesses of the target process if `with_subprocesses`
    /// is enabled.
    pub pid: crate::core::types::Pid,
    /// Whether to profile the target process (given by `pid`) as well as its child processes, and
    /// their child processes, and so on. Default: `false`.
    pub with_subprocesses: bool,
    /// Whether rbspy should keep its output quiet. If `true`, rbspy will only produce output when
    /// there's an error and when it finishes recording. If `false`, it will print a snapshot of
    /// the most recent trace every second. Default: `false`.
    pub silent: bool,
    /// The number of traces that should be collected each second. Default: `100`.
    pub sample_rate: u32,
    /// The length of time that the recorder should run before stopping. Default: none (run until
    /// interrupted).
    pub maybe_duration: Option<std::time::Duration>,
    /// Minimum flame width. Applies to flamegraph output only. If your sample has many small
    /// functions in it and is difficult to read, then consider increasing this value.
    /// Default: 0.1.
    pub flame_min_width: f64,
    /// Locks the process when a sample is being taken.
    ///
    /// You should enable this option for the most accurate samples. However, it briefly
    /// stops the process from executing and can affect performance. The performance impact
    /// is most noticeable in CPU-bound ruby programs or when a high sampling rate is used.
    pub lock_process: bool,
    /// Signals to the recorder that it should stop.
    pub done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// Records traces according to the given configuration
pub fn record(config: Config) -> Result<(), Error> {
    let time_limit = match config.maybe_duration {
        Some(duration) => Some(std::time::Instant::now() + duration),
        None => None,
    };

    let sampler = crate::sampler::Sampler::new(
        config.pid, 
        config.sample_rate, 
        config.lock_process, 
        time_limit, 
        config.with_subprocesses, 
        config.done,
    );

    // Create the sender/receiver channels and start the child threads off collecting stack traces
    // from each target process.
    // Give the child threads a buffer in case we fall a little behind with aggregating the stack
    // traces, but not an unbounded buffer.
    let (trace_sender, trace_receiver) = std::sync::mpsc::sync_channel(100);
    let (result_sender, result_receiver) = std::sync::mpsc::channel();
    sampler.start(trace_sender, result_sender)?;

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
                    sampler.timing_error_traces(),
                    sampler.total_traces(),
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
