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
    configure_cargo

    # TODO if you need to install extra stuff add it here
}

main
