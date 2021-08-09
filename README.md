# rbspy

[![crates.io](https://badgen.net/crates/v/rbspy)](https://crates.io/crates/rbspy)
[![ci](https://github.com/rbspy/rbspy/actions/workflows/ci.yml/badge.svg)](https://github.com/rbspy/rbspy/actions/workflows/ci.yml)

<img src="https://rbspy.github.io/rbspy.jpg" width="128px">

Have a running Ruby program that you want to profile without restarting it? Want to profile a Ruby
command line program really easily? You want `rbspy`! rbspy can profile any Ruby program just by
running 1 simple command.

`rbspy` lets you profile Ruby processes that are already running. You give it a PID, and it starts
profiling. It's a sampling profiler, which means it's **low overhead** and **safe to run in
production**.

`rbspy` lets you record profiling data, save the raw profiling data to disk, and then analyze it in
a variety of different ways later on.

## profiling options

There are 2 main ways to profile code -- you can either profile everything the
application does (including waiting), or only profile when the application is using the CPU.

By default, rbspy profiles everything the program does (including waiting).
To sample only when the program is using the CPU, use the `--on-cpu` flag.

`--on-cpu` is implemented by 2 layers of filtering using the following sources of information:
- OS level thread information (only available for linux, macos and windows)
- current thread state in the ruby interpreter.

In addition, rbspy can either profile in a blocking or nonblocking mode.
The latter is less accurate but does not require rbspy to stop the live process to take samples.

## Documentation

=> https://rbspy.github.io

## Requirements

rbspy supports Linux\*, Mac, Windows, and FreeBSD.

<small>
* kernel version 3.2+ required. For Ubuntu, this means Ubuntu 12.04 or newer.
</small>

## Add a testimonial

Did rbspy help you make your program faster? An awesome way to thank the project is to add a [success story to this GitHub issue](https://github.com/rbspy/rbspy/issues/62)
where people talk about ways rbspy has helped them! Hearing that rbspy is working for people is good
motivation :)

## Installing

On Mac, you can install with Homebrew: `brew install rbspy`.

On Linux:

1. Download recent release of `rbspy` from [the GitHub releases page](https://github.com/rbspy/rbspy/releases).
    * The binaries tagged with `musl` are statically linked against musl libc and can be used on most systems. The ones tagged with `gnu` are dynamically linked against GNU libc, so you will need it to be installed.
2. Unpack it
3. Move the `rbspy` binary to `/usr/local/bin`

Or have a look at [Installing rbspy](https://rbspy.github.io/installing/) on our documentation.

## Contributing

Pull requests that improve usability, fix bugs, or help rbspy support more operating systems are
very welcome. If you have a question, the best way to ask is to [create a GitHub issue](https://github.com/rbspy/rbspy/issues/new)!

If you're not a very experienced Rust programmer, you're very welcome to contribute. A major reason
rbspy is written in Rust is that Rust is more approachable for beginners than C/C++.
https://www.rust-lang.org/ has great resources for learning Rust.

## Building rbspy

1. Install cargo from [crates.io](https://crates.io/)
1. `cargo build` to build
1. `cargo test` to test

The built binary will end up at `target/debug/rbspy`
