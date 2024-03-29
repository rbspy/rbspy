#!/usr/bin/env bash

source "$HOME/.cargo/env"

set -e

ruby -v
cargo --version

cd /vagrant

if [ -f build-artifacts.tar ]; then
  echo "Unpacking cached build artifacts..."
  tar xf build-artifacts.tar
  rm -f build-artifacts.tar
fi

cargo build --release --workspace --all-targets
cargo test --release -- \
    --skip core::ruby_spy::tests \
    --skip sampler::tests

set +e
tar cf build-artifacts.tar target
tar rf build-artifacts.tar "$HOME/.cargo/git"
tar rf build-artifacts.tar "$HOME/.cargo/registry"

exit 0
