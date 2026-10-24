#!/bin/sh
# The snake rules are hardware-free, so they're tested on the host rather than on the badge.
set -e
cd "$(dirname "$0")/.."
mkdir -p target
rustc --edition 2024 --test tools/game-tests.rs -o target/game-tests
./target/game-tests
