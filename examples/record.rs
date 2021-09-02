extern crate rbspy;

use rbspy::recorder::{RecordConfig, Recorder};
use rbspy::OutputFormat;

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

fn path_to_ruby_binary() -> String {
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

    ruby_binary_path_str.to_string()
}
