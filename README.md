# rbspy

<a href="https://travis-ci.org/rbspy/rbspy"><img src="https://travis-ci.org/rbspy/rbspy.svg"></a>
[![Join the chat at https://gitter.im/rbspy/rbspy](https://badges.gitter.im/rbspy/rbspy.svg)](https://gitter.im/rbspy/rbspy?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

<img src="https://github.com/rbspy/rbspy/raw/master/logo/rbspy.png" width="128px">

----

Have you ever wanted to know what functions your Ruby program is calling? `rbspy` can tell you!

`rbspy` lets you profile running Ruby processes. It's the only Ruby profiler that can profile
arbitrary Ruby processes that are already running.

It's currently alpha software, and is being actively developed. Please report bugs!

<img src="https://user-images.githubusercontent.com/817739/35197779-dfae334e-feb2-11e7-95f5-02d80a39e5bb.gif">

## Requirements

rbspy runs on Linux\* and Mac.

<small>
* kernel version 3.2+ required. For Ubuntu, this means Ubuntu 12.04 or newer.
</small>


## How to get rbspy

1. Download recent release of `rbspy` from [the github releases page](https://github.com/rbspy/rbspy/releases)
2. Unpack it
3. Move the `rbspy` binary to `/usr/local/bin`

## Using rbspy

rbspy currently has 2 features: snapshot and record.

### Snapshot

Snapshot takes a single stack trace from the specified process, prints it, and exits. This is
useful if you have a stuck Ruby program and just want to know what it's doing right now.  Must be
run as root.

```
sudo rbspy snapshot --pid $PID
```

### Record

Record records stack traces from your process and saves them to disk.

`rbspy record` will save 2 files: a gzipped raw data file, and a visualization (by default a flamegraph, you
can configure the visualization format with `--format`). The raw data file contains every stack
trace that `rbspy` recorded, so that you can generate other visualizations later if you want. By
default, rbspy saves both files to `~/.cache/rbspy/records`, but you can customize where those are
stored with `--file` and `--raw-file`.

This is useful when you want to know what functions your program is spending most of its time in.

You can record stack traces in two different ways, by PID or by executing a new ruby process.

#### Record by PID

```
# Must be run as root
sudo rbspy record --pid $PID
```

#### Record by executing the process through rbspy

```
# Must be run as root on Mac (but not Linux)
rbspy record ruby myprogram.rb
```

The reason this has to be run as root on Mac but not on Linux is that Mac and Linux systems APIs are
different. rbspy can use the `process_vm_readv` system call to read memory from a child process on
Linux without being root, but can't do the same with `vm_read` on Mac.

If run with sudo, `rbspy record` by default drops root privileges when executing a subprocess. So if
you're user `bork` and run `sudo rbspy record ruby script.rb`. You can disable this with
`--no-drop-root`.

#### Optional Arguments

These work regardless of how you started the recording. 

 * `--rate`: Specifies the frequency of that stack traces are recorded. The interval is determined by `1000/rate`. The default rate is 100hz.
 * `--duration`: Specifies how long to run before stopping rbspy. This conficts with running a subcommand (`rbspy record ruby myprogram.rb`).
 * `--format`: Specifies what format to use to report profiling data. The options are:
   * `flamegraph`: generates a flamegraph SVG that you can view in a browser
   * `callgrind`: generates a callgrind-formatted file that you can view with a tool like
     `kcachegrind`.
   * `summary`: aggregates % self and % total times by function. Useful to get a basic overview
   * `summary_by_line`: aggregates % self and % total times by line number. Especially useful when
      there's 1 line in your program which is taking up all the time.
 * `--file`: Specifies where rbspy will save formatted output. 
 * `--raw-file`: Specifies where rbspy will save formatted data. Use a gz extension because it will be gzipped.

## Reporting

If you have a raw rbspy data file that you've previously recorded, you can use `rbspy report` to
generate different kinds of visualizations from it (the flamegraph/callgrind/summary formats, as
documented above). This is useful because you can record raw data from a program and then decide how
you want to visualize it afterwards.

For example, here's what recording a simple program and then generating a summary report looks like:

```
$ sudo rbspy record --raw-file raw.gz ruby ci/ruby-programs/short_program.rb
$ rbspy report -f summary -i raw.gz -o summary.txt
$ cat summary.txt
% self  % total  name
100.00   100.00  <c function> - unknown
  0.00   100.00  ccc - ci/ruby-programs/short_program.rb
  0.00   100.00  bbb - ci/ruby-programs/short_program.rb
  0.00   100.00  aaa - ci/ruby-programs/short_program.rb
  0.00   100.00  <main> - ci/ruby-programs/short_program.rb
```

## What's a flamegraph?

rbspy uses [Brendan Gregg's flamegraph script](https://github.com/brendangregg/flamegraph) to
generate flamegraphs!

A flamegraph is a way to visualize profiling data from a process. Here's a flamegraph of
Jekyll building a blog recorded with `rbspy record jekyll build`.

You can see it spends about 50% of its time building the site (on the left, above `execute`) and
about 50% of its time loading requires (on the right, above `require`).

<a href="https://user-images.githubusercontent.com/817739/35201793-3a16071a-feec-11e7-8583-e1fa3c5e14b2.png">
<img src="https://user-images.githubusercontent.com/817739/35201793-3a16071a-feec-11e7-8583-e1fa3c5e14b2.png">
</a>

## On the "Dropped X/Y stack traces because of errors" message

rbspy does not stop your Ruby processes to collect information about what it's doing. This is for
both performance reasons and general production-safety reasons -- only **reading** from your Ruby
processes and not altering them in any way means that rbspy is safer to run on production Ruby
applications. rbspy does not use ptrace or signals.

This means that sometimes rbspy will try to read a stack trace out of a Ruby process, there will be
a race, and the memory of that process will temporarily be in an invalid state which means rbspy
can't collect its stack. `rbspy record` handles this by just dropping that stack trace and trying
again later, and reports the total number of dropped stack traces when it's done.

A typical error message here is something like "Dropped 13/3700 stack traces because of errors". If
you're seeing high error rates (more than 1/100 or so), please create an issue.

## Missing features

Contributions in any of these areas would be very welcome.

* BSD/Windows support
* Profile multiple threads
* Profile C extensions (rbspy will simply ignore any calls into C extensions)

## Contributing

A major goal for this project is to get more maintainers and contributors. Pull requests that
improve usability, fix bugs, or help rbspy support more operating systems are very welcome. If you
have questions about contributing come chat [on gitter](https://gitter.im/rbspy/rbspy).

If you're not a very experienced Rust programmer, you're very welcome to contribute. A major reason
rbspy is written in Rust is that Rust is more approachable than C/C++.
https://www.rust-lang.org/en-US/ has great resources for learning Rust.

## Building rbspy

1. Install cargo from [crates.io](https://crates.io/)
1. `cargo build` to build
1. `cargo test` to test

The build artifacts will end up in `target/debug`

## Authors

* Julia Evans
* Kamal Marhubi
