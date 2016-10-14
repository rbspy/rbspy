# `script` phase: you usually build, test and generate docs in this phase

set -ex

. $(dirname $0)/utils.sh

# TODO modify this function as you see fit
# PROTIP Always pass `--target $TARGET` to cargo commands, this makes cargo output build artifacts
# to target/$TARGET/{debug,release} which can reduce the number of needed conditionals in the
# `before_deploy`/packaging phase
run_test_suite() {
    cargo build --target $TARGET --verbose
    cargo test --target $TARGET

    # sanity check the file type
    file target/$TARGET/debug/ruby-stacktrace
}

main() {
    run_test_suite
}

main
