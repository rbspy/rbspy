#!/usr/bin/env bash

source ~/.bash_profile

set -e

ruby -v
cargo --version

export CARGO_HOME="/vagrant/.cargo"
mkdir -p $CARGO_HOME

cd /vagrant

if [ -f build-artifacts.tar ]; then
  tar xf build-artifacts.tar
  rm -f build-artifacts.tar
fi

cargo build --release --workspace --all-targets
cargo test --release -- \
    --skip core::initialize::tests \
    --skip sampler::tests

tar cf build-artifacts.tar target
tar rf build-artifacts.tar .cargo/git || true
tar rf build-artifacts.tar .cargo/registry || true
