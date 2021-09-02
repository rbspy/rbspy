use anyhow::{Context, Error, Result};
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::storage::Store;
use crate::ui::summary;

/// A configuration bundle for the recorder
pub struct Config {
    /// The format to use for recorded traces. See `OutputFormat` for a list of available options.
    pub format: crate::core::types::OutputFormat,
    /// Where to write rbspy's raw trace output, which can be used for later processing.
    pub raw_path: Option<PathBuf>,
    /// Where to write rbspy's output. If `-` is given, output is written to standard output.
    pub out_path: Option<PathBuf>,
    /// The process ID (PID) of the process to profile. This is usually a ruby process, but rbspy
    /// will locate and profile any ruby subprocesses of the target process if `with_subprocesses`
    /// is enabled.
    pub pid: crate::core::process::Pid,
    /// Whether to profile the target process (given by `pid`) as well as its child processes, and
    /// their child processes, and so on. Default: `false`.
    pub with_subprocesses: bool,
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
}

pub struct Recorder {
    format: crate::core::types::OutputFormat,
    flame_min_width: f64,
    out_path: Option<PathBuf>,
    raw_path: Option<PathBuf>,
    sample_rate: u32,
    sampler: crate::sampler::Sampler,
    summary: Arc<Mutex<summary::Stats>>,
}

impl Recorder {
    pub fn new(config: Config) -> Self {
        let sampler = crate::sampler::Sampler::new(
            config.pid,
            config.sample_rate,
            config.lock_process,
            config.maybe_duration,
            config.with_subprocesses,
        );

        Recorder {
            format: config.format,
            flame_min_width: config.flame_min_width,
            out_path: config.out_path,
            raw_path: config.raw_path,
            sample_rate: config.sample_rate,
            sampler,
            summary: Arc::new(Mutex::new(summary::Stats::new())),
        }
    }

    /// Records traces until the process exits or the stop function is called
    pub fn record(&self) -> Result<(), Error> {
        // Create the sender/receiver channels and start the child threads off collecting stack traces
        // from each target process.
        // Give the child threads a buffer in case we fall a little behind with aggregating the stack
        // traces, but not an unbounded buffer.
        let (trace_sender, trace_receiver) = std::sync::mpsc::sync_channel(100);
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        self.sampler.start(trace_sender, result_sender)?;

        // Aggregate stack traces as we receive them from the threads that are collecting them
        // Aggregate to 3 places: the raw output (`.raw.gz`), some summary statistics we display live,
        // and the formatted output (a flamegraph or something)
        let mut out = None;
        if self.out_path.is_some() {
            out = Some(self.format.clone().outputter(self.flame_min_width));
        }
        let mut raw_store = None;
        if let Some(raw_path) = &self.raw_path {
            raw_store = Some(Store::new(&raw_path, self.sample_rate)?);
        }

        for trace in trace_receiver {
            if let Some(out) = &mut out {
                out.record(&trace)?;
            }
            if let Some(raw_store) = &mut raw_store {
                raw_store.write(&trace)?;
            }

            let mut summary = self.summary.lock().unwrap();
            summary.add_function_name(&trace.trace);
        }

        // Finish writing all data to disk
        if let (Some(out), Some(out_path)) = (&mut out, self.out_path.as_ref()) {
            if out_path.display().to_string() == "-" {
                out.complete(&mut std::io::stdout())?;
            } else {
                let mut out_file = File::create(&out_path).context(format!(
                    "Failed to create output file {}",
                    &out_path.display()
                ))?;
                out.complete(&mut out_file)?;
            }
        }
        if let Some(raw_store) = raw_store {
            raw_store.complete();
        }

        // Check for errors from the child threads. Ignore errors unless every single thread
        // returned an error. If that happens, return the last error. This lets rbspy successfully
        // record processes even if the parent thread isn't a Ruby process.
        let mut num_ok = 0;
        let mut last_result = Ok(());
        for result in result_receiver {
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

    /// Stops the recorder
    pub fn stop(&self) {
        self.sampler.stop();
    }

    /// Writes a summary of collected traces
    pub fn write_summary(&self, w: &mut dyn std::io::Write) -> Result<(), Error> {
        let width = match term_size::dimensions() {
            Some((w, _)) => Some(w as usize),
            None => None,
        };
        let timing_error_traces = self.sampler.timing_error_traces();
        let total_traces = self.sampler.total_traces();
        let percent_timing_error = (timing_error_traces as f64) / (total_traces as f64) * 100.0;

        let summary = self.summary.lock().unwrap();
        writeln!(
            w,
            "Time since start: {}s. Press Ctrl+C to stop.",
            summary.elapsed_time().as_secs()
        )?;

        writeln!(w, "Summary of profiling data so far:")?;
        summary.write_top_n(w, 20, width)?;

        if total_traces > 100 && percent_timing_error > 0.5 {
            // Only include this warning if timing errors are more than 0.5% of total traces. rbspy
            // is a statistical profiler, so smaller differences don't really matter.
            writeln!(w, "{:.1}% ({}/{}) of stack traces were sampled late because we couldn't sample at expected rate; results may be inaccurate. Current rate: {}. Try sampling at a lower rate with `--rate`.", percent_timing_error, timing_error_traces, total_traces, self.sample_rate)?;
        }
        Ok(())
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop();
    }
}
