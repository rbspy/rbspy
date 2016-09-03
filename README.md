# ruby-stacktrace

Have you ever wanted to know what your Ruby program is doing?
`ruby-stacktrace` can tell you! Maybe.

**this is alpha, Linux-only software**. It will very likely crash. If it
crashes, the rest of your system should probably be fine (it shouldn't
crash your Ruby). But I wouldn't totally swear that.

## Requirements

1. Linux (It uses a Linux-only system call)
2. The most recent pre-release of `ruby-stacktrace` (download from [here](https://github.com/jvns/ruby-stacktrace/releases))
3. A Ruby version compiled with debugging symbols (check by running
   `file` on your Ruby binary)

I've tested this succesfully on Ruby versions 2.1.6 and 2.2.3. No
promises though. It works on my computer and at least 2 other computers.

## How to use it

1. Download recent release of `ruby-stacktrace` (download from [here](https://github.com/jvns/ruby-stacktrace/releases))
1. Find the PID of the Ruby process you want to investigate (like 7723)
1. run `sudo ./ruby-stacktrace top 7723`
1. It'll either work (and tell you which functions are being called the most)
   or crash
1. I would not run this on a production system today, but I don't know
   of any specific reason you shouldn't (other than that it's sketchy
   alpha software)

If it crashes, you can file an issue and attach the Ruby binary for the
process it couldn't spy on. I will read all the issues and help if I
can! Especially if it's just that something in this README is explained
poorly. I have approximately no time to fix issues, so I will probably
not fix the bug. Pull requests are very welcome!

## Generating flamegraphs

You can use this tool to generate flamegraphs for a running Ruby
process. 

1. Get the [FlameGraph repository](https://github.com/brendangregg/FlameGraph) and add it to your PATH
1. Run `sudo ./ruby-stacktrace stackcollapse $PID > stacks` until you
   get bored of collecting data
1. run `stackcollapse.pl < stacks | flamegraph.pl > output.svg`
1. Open output.svg! You should get a beautiful graph like this: (click
   to enlarge)

<a href="http://jvns.ca/images/sampling.png"><img src="http://jvns.ca/images/sampling.png" width="400px"></a>

## How it works

I wrote a blog post about the internals at [How to spy on a Ruby process](http://jvns.ca/blog/2016/06/12/a-weird-system-call-process-vm-readv/)

## Developing ruby-stacktrace

It's written in Rust.

1. Install cargo from [crates.io](https://crates.io/)
1. `cargo build` to build
1. `cargo test` to test
1. `cargo bench` for benchmarks

The build artifacts will end up in `target/release`

## Authors

(in alphabetical order)

* Julia Evans
* Kamal Marhubi
