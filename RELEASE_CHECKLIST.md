# Release Checklist

1. Make sure your local copy is up to date with respect to the upstream repository (GitHub).
1. Run `cargo update` to upgrade crate dependencies. Commit the changes to `Cargo.lock`.
1. Update the version number in `ruby-structs/Cargo.toml` so that it matches the new rbspy version.
1. Update `Cargo.toml` with the new rbspy version, and then run `cargo build` to ensure `Cargo.lock` is updated. Commit the changes.
1. Push the commits (or open a PR). If you open a PR, wait for it to be reviewed and merged before continuing.
1. Wait for CI to finish and verify that all checks passed.
1. Tag the new release, e.g. `git tag v0.3.11`, and push it: `git push --tags`.
    - GitHub Actions workflows will build new binaries, create a new draft release, and attach the binaries.
    - If the release build fails, delete the tag from GitHub. Fix the issue, and then create a new release tag and push it.
1. Browse to the [draft release](https://github.com/rbspy/rbspy/releases) and edit the generated release notes so that they highlight the key changes. Add links to the PRs or commits for those changes.
1. Publish the release. GitHub Actions workflows will publish the new version to crates.io and Docker Hub.
