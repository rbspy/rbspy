mod include;
extern crate rbspy;

use crate::include::path_to_ruby_binary;
use rbspy::recorder::snapshot;

fn main() {
    let mut process = std::process::Command::new(path_to_ruby_binary())
        .arg("ci/ruby-programs/infinite_on_cpu.rb")
        .spawn()
        .unwrap();
    let pid = process.id() as rbspy::Pid;

    match snapshot(pid, true, None, false) {
        Ok(Some(s)) => println!("{}", s),
        Ok(None) => println!("No stack trace was captured"),
        Err(e) => println!("Failed to get snapshot: {:?}", e),
    }

    process.kill().expect("couldn't clean up ruby process");
}
