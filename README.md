<a href="https://travis-ci.org/rbspy/rbspy"><img src="https://travis-ci.org/rbspy/rbspy.svg"></a>
[![Join the chat at https://gitter.im/rbspy/rbspy](https://badges.gitter.im/rbspy/rbspy.svg)](https://gitter.im/rbspy/rbspy?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

# rbspy

Have you ever wanted to know what functions your Ruby program is calling? `rbspy` can tell you!

`rbspy` lets you profile running Ruby processes. It's the only Ruby profiler that can profile
arbitrary Ruby processes that are already running.

It's currently alpha software, and is being actively developed. Please report bugs!

## Requirements

rbspy only runs on Linux\*. Mac support is planned.

<small>
* kernel version 3.2+ required. For Ubuntu, this means Ubuntu 12.04 or newer.
</small>


## How to get rbspy

1. Download recent release of `rbspy` (download from [the github releases page](https://github.com/rbspy/rbspy/releases))
2. Unpack it
3. Move the `rbspy` binary to `/usr/local/bin`

## Using rbspy

rbspy currently has 2 features: snapshot and record.

**Snapshot**

Snapshot takes a single stack trace from the specified process, prints it, and exits. This is
useful if you have a stuck Ruby program and just want to know what it's doing right now.  Must be
run as root.

```
sudo rbspy snapshot --pid $PID
```

**Record**

Record records stack traces from your process and generates a flamegraph. You can either give it the
PID of an already-running process to record, or ask it to execute and record a new Ruby process.

This is useful when you want to know what functions your program is spending most of its time in.

```
sudo rbspy record --pid $PID
# recording a subprocess doesn't require root access
rbspy record ruby myprogram.rb
```

When recording, rbspy will by default save data to `~/.cache/rbspy/records`. You can also specify an
output file with `--file`.

**Generate a flamegraph**

Here's how to convert the output of `rbspy record` into a flamegraph.

1. Get the [FlameGraph repository](https://github.com/brendangregg/FlameGraph) and add it to your PATH
1. run `stackcollapse.pl < stacks | flamegraph.pl > output.svg`
1. Open output.svg in Firefox or Chrome! You should get a beautiful graph like this: (click
   to enlarge)

<a href="http://jvns.ca/images/sampling.png"><img src="http://jvns.ca/images/sampling.png" width="400px"></a>

## Missing features

* Mac support 
* Profile multiple threads
* Profile C extensions (rbspy will simply ignore any calls into C extensions)
* Profile processes running in containers
* Generate flamegraphs without relying on an external script

## Contributing

Contributions are very welcome! rbspy is written in Rust. If you don't know Rust but you're
interested in learning some Rust and contributing, we'd love to have you. The reason that rbspy is
written in Rust (and not C, like many other Ruby tools) is that Rust is easier to learn than C in a
lot of ways.

1. Install cargo from [crates.io](https://crates.io/)
1. `cargo build` to build
1. `cargo test` to test

The build artifacts will end up in `target/debug`

## Authors

* Julia Evans
* Kamal Marhubi
