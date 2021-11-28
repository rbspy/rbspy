# New Ruby Version Checklist

When a new version of Ruby is released, rbspy needs to be updated before it can profile programs that use the new version. The maintainers try to add support for new versions soon after they're released, but the process is open to everyone. Feel free to contribute a PR if you'd like rbspy to support a new Ruby version.

## Generate bindings

We use the `bindgen` tool to generate the glue code, also known as bindings, that make it possible for Rust to interoperate with Ruby's C libraries.

1. If you're not a maintainer, fork the rbspy repository
1. Create a new git branch
1. Add the new Ruby version(s) to the [ruby-bindings workflow](https://github.com/rbspy/rbspy/blob/master/.github/workflows/ruby-bindings.yml). ([example](https://github.com/rbspy/rbspy/commit/a5871fe7e7a2cc93e57b5b3aca8c197497a7b2ae))
1. Commit the change and push your branch
1. Browse to the [ruby-bindings workflow](https://github.com/rbspy/rbspy/actions/workflows/ruby-bindings.yml), click "Run workflow", select your branch from the list, and click "Run workflow" again. (If you're working in a fork of the rbspy repository, then you'll need to run the workflow in your fork.)
    * Under the hood, this workflow runs `cargo bindgen <ruby version tag>` for every version that rbspy supports, including the ones you just added. This process usually takes 5-10 minutes.
1. When the build finishes, it creates a new branch with a name like "generate-ruby-bindings-42". Merge that branch into the branch you made earlier, or cherry-pick the commit into your branch.

## Update version-specific code paths

With new bindings in hand, we can update rbspy itself to work with the new Ruby version.

1. Open `ruby-structs/src/lib.rs` and add a `mod` line for each new version.
1. Open `src/core/initialize.rs` and add a line for each new version in the `is_maybe_thread_function` and `get_stack_trace_function` functions.
1. Open `src/core/ruby_version.rs` and add a line for each new version. Please also add a test for each one.
1. Commit your changes and push your branch
1. Open a PR

To understand where the lines need to be added and how to write the tests, you can use [this commit](https://github.com/rbspy/rbspy/commit/9d8fee1665c1b4fcdb007533307696d524964e84) as a template.
