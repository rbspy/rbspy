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

run_ruby_integration_tests_mac() {
    # Tests that rbspy works with rvm and rbenv-installed Ruby on Mac
    curl -sSL https://get.rvm.io | bash
    brew install ruby-build
    mkdir -p ~/.rbenv/versions
    export PATH=$PATH:~/.rvm/bin/
    for version in ruby-2.0.0 ruby-2.2.0 ruby-2.4.0 ruby-2.5.0
        do
        ruby_version=ruby-$version

        # rvm
        rvm install $ruby_version
        rvm use $ruby_version
        sudo target/$TARGET/debug/rbspy record --file stacks.txt ruby ci/ruby-programs/short_program.rb
        # check that the number of stacks counted is a reasonable number
        [ `wc -l stacks.txt | awk '{print $1}'` -gt "50" ]

        # rbenv
        ruby-build $version ~/.rbenv/versions/$version
        sudo target/$TARGET/debug/rbspy record --file stacks.txt ~/.rvm/rubies/$ruby_version/bin/ruby ci/ruby-programs/short_program.rb
        [ `wc -l stacks.txt | awk '{print $1}'` -gt "50" ]
        done
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
    if [[ "$TRAVIS_OS_NAME" == "osx" ]]
    then
        run_ruby_integration_tests_mac
    fi
}

main
