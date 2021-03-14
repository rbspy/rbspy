# rbspy

<a href="https://travis-ci.org/rbspy/rbspy"><img src="https://travis-ci.org/rbspy/rbspy.svg"></a>

<img src="https://rbspy.github.io/rbspy.jpg" width="128px">

Have a running Ruby program that you want to profile without restarting it? Want to profile a Ruby
command line program really easily? You want `rbspy`! rbspy can profile any Ruby program just by
running 1 simple command.

`rbspy` lets you profile Ruby processes that are already running. You give it a PID, and it starts
profiling. It's a sampling profiler, which means it's **low overhead** and **safe to run in
production**.

`rbspy` lets you record profiling data, save the raw profiling data to disk, and then analyze it in
a variety of different ways later on.

## only wall-clock profiling

There are 2 main ways to profile code -- you can either profile everything the
application does (including waiting), or only profile when the application is using the CPU.

rbspy profiles everything the program does (including waiting) -- there's no
option to just profile when the program is using the CPU.

## Documentation

=> https://rbspy.github.io

## Requirements

rbspy supports Linux\*, Mac, and Windows.

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

1. Download recent release of `rbspy` from [the GitHub releases page](https://github.com/rbspy/rbspy/releases)
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

## Tagging a release

Here are the steps for maintainers to tag a new release:

1. Update `Cargo.toml` with the new version, run `cargo build` to ensure `Cargo.lock` is updated.
1. If you have updated the ruby-structs bindings, update the version number in `ruby-structs/Cargo.toml` so that it matches the new rbspy version.
1. Open a PR for the version bump. You can generate a CHANGELOG via `git log --pretty='- %s' v0.3.10...HEAD`.
1. After the PR is merged, tag the new release, e.g. `git tag v0.3.11`, and push it: `git push --tags`.
1. Travis will publish the tarballs to GitHub.

## Contributors

* [Julia Evans](https://github.com/jvns)
* [Kamal Marhubi](https://github.com/kamalmarhubi)
* [Joel Johnson](https://github.com/liaden/)
