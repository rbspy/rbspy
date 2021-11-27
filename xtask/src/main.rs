mod bindgen;

use anyhow::{anyhow, Result};
use std::env;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("Failed to run task: {:?}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<()> {
    let task = env::args().nth(1);
    match task.as_ref().map(|it| it.as_str()) {
        Some("bindgen") => {
            if let Some(version_tag) = env::args().nth(2) {
                bindgen::generate_ruby_bindings(
                    std::path::PathBuf::from("ruby-source"),
                    &version_tag,
                )?;
            } else {
                return Err(anyhow!("please provide a ruby tag parameter, e.g. v3_0_3"));
            };
        }
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

bindgen <ruby version tag>      Generates Rust bindings for various Ruby VM versions
"
    )
}
