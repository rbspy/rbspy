extern crate rbspy;

use rbspy::recorder::snapshot;

fn main() {
    let mut process = std::process::Command::new(path_to_ruby_binary())
        .arg("ci/ruby-programs/infinite.rb")
        .spawn()
        .unwrap();
    let pid = process.id() as rbspy::Pid;

    match snapshot(pid, true) {
        Ok(s) => println!("{}", s),
        Err(e) => println!("Failed to get snapshot: {:?}", e),
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
