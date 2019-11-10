# `install` phase: install stuff needed for the `script` phase

set -ex

. $(dirname $0)/utils.sh

install_rustup() {
    # uninstall the rust toolchain installed by travis, we are going to use rustup

    curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain=$TRAVIS_RUST_VERSION

    rustc -V
    cargo -V
}

install_standard_crates() {
    if [ $(host) != "$TARGET" ]; then
        rustup target add $TARGET
    fi
}

install_system_dependencies() {
    if [ $TARGET = x86_64-unknown-linux-musl ]; then
        sudo apt-get install musl-tools
        # download libunwind and build a static version w/ musl-gcc
        wget https://github.com/libunwind/libunwind/releases/download/v1.3.1/libunwind-1.3.1.tar.gz
        tar -zxvf libunwind-1.3.1.tar.gz
        cd libunwind-1.3.1/
        CC=musl-gcc ./configure --disable-minidebuginfo --enable-ptrace --disable-tests --disable-documentation
        make
        sudo make install
        cd ..
    fi
}

configure_cargo() {
    local prefix=$(gcc_prefix)

    if [ ! -z $prefix ]; then
        # information about the cross compiler
        ${prefix}gcc -v

        # tell cargo which linker to use for cross compilation
        mkdir -p .cargo
        cat >>.cargo/config <<EOF
[target.$TARGET]
linker = "${prefix}gcc"
EOF
    fi
}

main() {
    install_rustup
    install_standard_crates
    install_system_dependencies
    configure_cargo

    # TODO if you need to install extra stuff add it here
}

main
