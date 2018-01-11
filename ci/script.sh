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

run_docker_tests() {
    rm -rf /tmp/artifacts
    mkdir /tmp/artifacts
    cp target/$TARGET/debug/ruby-stacktrace /tmp/artifacts
    cp examples/short_program.rb /tmp/artifacts
    cp examples/infinite.rb /tmp/artifacts
    ls
    ls docker/
    for distro in ubuntu1404 ubuntu1704 fedora arch2018
    do
        docker build -t rb-stacktrace-$distro -f ./docker/Dockerfile.$distro  ./docker/ >> /tmp/output 2>&1
        echo -n "${distro}... "
        docker run -v=/tmp/artifacts:/stuff rb-stacktrace-$distro  env RUST_LOG=debug RUST_BACKTRACE=1 /stuff/ruby-stacktrace stackcollapse /usr/bin/ruby /stuff/short_program.rb >> /tmp/output 2>&1
    done
}

main() {
    run_test_suite
    run_docker_tests
}

main
