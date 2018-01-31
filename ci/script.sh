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
    file target/$TARGET/debug/rbspy

    if [[ "$TRAVIS_OS_NAME" == "linux" ]]
    then
        sudo apt-get install -y libjemalloc1
        sudo apt-get install -y libtcmalloc-minimal4

        # test jemalloc + tcmalloc
        target/$TARGET/debug/rbspy record env LD_PRELOAD=/usr/lib/libjemalloc.so.1 /usr/bin/ruby ci/ruby-programs/short_program.rb
        target/$TARGET/debug/rbspy record env LD_PRELOAD=/usr/lib/libtcmalloc_minimal.so.4 /usr/bin/ruby ci/ruby-programs/short_program.rb
    fi
}

run_docker_tests() {
    if [[ "$TRAVIS_OS_NAME" == "linux" ]]
    then
        bash $(dirname $0)/docker-tests.sh
    fi
}

main() {
    run_test_suite
    run_docker_tests
}

main
