# New Ruby Version Checklist

When a new version of Ruby is released, rbspy needs to be updated before it can profile programs that use the new version. The maintainers try to add support for new versions soon after they're released, but the process is open to everyone. Feel free to contribute a PR if you'd like rbspy to support a new Ruby version.

## Generate bindings

We use the `bindgen` tool to generate the glue code, also known as bindings, that make it possible for Rust to interoperate with Ruby's C libraries.

1. If you're not a maintainer, fork the rbspy repository
1. Create a new git branch
1. Add the new Ruby version(s) to the [ruby-bindings workflow](https://github.com/rbspy/rbspy/blob/main/.github/workflows/ruby-bindings.yml) and [ruby-version-tests workflow](https://github.com/rbspy/rbspy/blob/main/.github/workflows/ruby-version-tests.yml). ([example](https://github.com/rbspy/rbspy/commit/ba2508841476673c670350f87878fa7604ea6de1))
1. Commit the change and push your branch
1. Browse to the [ruby-bindings workflow](https://github.com/rbspy/rbspy/actions/workflows/ruby-bindings.yml), click "Run workflow", select your branch from the list, and click "Run workflow" again. (If you're working in a fork of the rbspy repository, then you'll need to run the workflow in your fork.)
    * Under the hood, this workflow runs `cargo xtask bindgen <ruby version tag>` for every version that rbspy supports, including the ones you just added. This process usually takes 5-10 minutes.
1. When the build finishes, it creates a new branch with a name like "generate-ruby-bindings-42". Merge that branch into the branch you made earlier, or cherry-pick the commit into your branch.

## Update version-specific code paths

With new bindings in hand, we can update rbspy itself to work with the new Ruby version.

1. Open `src/core/supported_ruby_versions.rs` and add each new version to the list.
1. Open `ruby-structs/src/lib.rs` and add a `mod` line for each new version.
1. Open `src/core/ruby_version.rs` and add a line for each new version. Please also add a test for each one (see next section).
1. Commit your changes and push your branch
1. Open a PR

To understand where the lines need to be added, you can use [this commit](https://github.com/rbspy/rbspy/commit/108667e6a049fcf7523a6e318361df6f7043eaf7) as a template.

## Update tests

1. Browse to the [coredump workflow](https://github.com/rbspy/rbspy/actions/workflows/coredump.yml)
1. Click "Run workflow" and enter the new Ruby version and your branch name
1. When the workflow finishes, it will have a `ruby-coredump-x.y.z.gz` file in the list of artifacts
1. Expand the "Inspect process" section of the build output to see the memory addresses of the VM and symbols
1. Add the core dump to the rbspy-testdata repo
1. Optional but recommended: edit rbspy's Cargo.toml to refer to your local copy of rbspy-testdata. This will make it easier to iterate on the test if it doesn't work the first time
1. Add a test in `ruby_version.rs` (and a `real_stack_trace_X_Y_Z` func) that uses the new core dump. You'll need the info from the "Inspect process" section of the build output
1. Verify that the test passes
1. Publish a new version of rbspy-testdata
1. Update the version of rbspy-testdata referenced in rbspy's Cargo.toml
