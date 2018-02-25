#!/bin/bash
set -xeuo #pipefail

#RUST_BACKTRACE=1 
RUST_LOG=debug ./rbspy record $(which ruby) /short_program.rb

#$(which ruby) infinite.rb &
#sleep 2s

#RUST_BACKTRACE=1 RUST_LOG=debug  ./rbspy snapshot -p $(pgrep ruby)