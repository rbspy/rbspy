mod include;
extern crate rbspy;

use crate::include::path_to_ruby_binary;
use rbspy::recorder::snapshot;

fn main() {
    let mut process = std::process::Command::new(path_to_ruby_binary())
        .arg("ci/ruby-programs/infinite.rb")
        .spawn()
        .unwrap();
    let pid = process.id() as rbspy::Pid;

    match snapshot(pid, true, None) {
        Ok(s) => println!("{}", s),
        Err(e) => println!("Failed to get snapshot: {:?}", e),
    }

    process.kill().expect("couldn't clean up ruby process");
}
