# New Ruby Version Checklist

When a new version of Ruby is released, rbspy needs to be updated before it can profile programs that use the new version. The maintainers try to add support for new versions soon after they're released, but the process is open to everyone. Feel free to contribute a PR if you'd like rbspy to support a new Ruby version.

## Generate bindings

We use the `bindgen` tool to generate the glue code, also known as bindings, that make it possible for Rust to interoperate with Ruby's C libraries.

1. If you're not a maintainer, fork the rbspy repository
1. Create a new git branch
1. Add the new Ruby version(s) to the [ruby-bindings workflow](https://github.com/rbspy/rbspy/blob/main/.github/workflows/ruby-bindings.yml) and [ruby-version-tests workflow](https://github.com/rbspy/rbspy/blob/main/.github/workflows/ruby-version-tests.yml). ([example](https://github.com/rbspy/rbspy/commit/ba2508841476673c670350f87878fa7604ea6de1))
1. Commit the change and push your branch
1. Browse to the [ruby-bindings workflow](https://github.com/rbspy/rbspy/actions/workflows/ruby-bindings.yml), click "Run workflow", select your branch from the list, and click "Run workflow" again. (If you're working in a fork of the rbspy repository, then you'll need to run the workflow in your fork.)
    * Under the hood, this workflow runs `cargo bindgen <ruby version tag>` for every version that rbspy supports, including the ones you just added. This process usually takes 5-10 minutes.
1. When the build finishes, it creates a new branch with a name like "generate-ruby-bindings-42". Merge that branch into the branch you made earlier, or cherry-pick the commit into your branch.

## Update version-specific code paths

With new bindings in hand, we can update rbspy itself to work with the new Ruby version.

1. Open `ruby-structs/src/lib.rs` and add a `mod` line for each new version.
1. Open `src/core/initialize.rs` and add a line for each new version in the `is_maybe_thread_function` and `get_stack_trace_function` functions.
1. Open `src/core/ruby_version.rs` and add a line for each new version. Please also add a test for each one (see next section).
1. Commit your changes and push your branch
1. Open a PR

To understand where the lines need to be added and how to write the tests, you can use [this commit](https://github.com/rbspy/rbspy/commit/9d8fee1665c1b4fcdb007533307696d524964e84) as a template.

## Update tests

1. Run `infinite.rb` with the new Ruby version

    ```sh
    rbenv local X.Y.Z
    ruby ci/ruby-programs/infinite.rb
    ```
1. In another terminal, run rbspy with RUST_LOG=debug to get VM and symbols addresses:

    ```sh
    cargo build && RUST_LOG=info ./target/debug/rbspy snapshot -p $(pgrep -fn infinite.rb)
    ```
1. Run `gcore -o ruby-coredump-X.Y.Z -p $(pgrep ruby)` to get a core dump
1. Compress the core dump with `gzip -9 ruby-coredump-X.Y.Z`
1. Add the core dump to the rbspy-testdata repo
1. Optional but recommended: edit rbspy's Cargo.toml to refer to your local copy of rbspy-testdata
1. Add a test in `ruby_version.rs` (and a `real_stack_trace_X_Y_Z` func) that uses the new core dump
1. Verify that the test passes
1. Publish a new version of rbspy-testdata
1. Update the version of rbspy-testdata referenced in rbspy's Cargo.toml
