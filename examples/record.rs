mod include;
extern crate rbspy;

use rbspy::recorder::{RecordConfig, Recorder};
use rbspy::OutputFormat;
use crate::include::path_to_ruby_binary;

fn main() {
    let mut process = std::process::Command::new(path_to_ruby_binary())
        .arg("ci/ruby-programs/infinite.rb")
        .spawn()
        .unwrap();
    let out_path = std::path::PathBuf::from("rbspy-out.svg");

    let config = RecordConfig {
        format: OutputFormat::flamegraph,
        raw_path: Some(std::path::PathBuf::from("rbspy-raw.txt")),
        out_path: Some(out_path.clone()),
        pid: process.id() as rbspy::Pid,
        with_subprocesses: false,
        sample_rate: 99,
        maybe_duration: Some(std::time::Duration::from_secs(1)),
        flame_min_width: 10.0,
        lock_process: true,
    };
    let recorder = Recorder::new(config);
    match recorder.record() {
        Ok(_) => println!(
            "A flamegraph was saved to {}",
            out_path.display().to_string()
        ),
        Err(e) => println!("Failed to record: {:?}", e),
    }

    process.kill().expect("couldn't clean up ruby process");
}
