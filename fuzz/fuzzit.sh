#!/bin/bash

# Fuzzit: https://app.fuzzit.dev

set -xe

if [ -z ${1+x} ]; then
    echo "must call with job type as first argument e.g. 'fuzzing' or 'sanity'"
    echo "see https://github.com/fuzzitdev/example-rust/blob/master/.travis.yml"
    exit 1
fi

# Install Rust dependencies
rustup install nightly
carog install --force cargo-fuzz afl honggfuzz

#Install Fuzzit dependencies
wget -q -O fuzzit https://github.com/fuzzitdev/fuzzit/releases/download/v2.4.29/fuzzit_Linux_x86_64
chmod a+x fuzzit

# Build fuzzers
cargo +nightly fuzz run crypto -- -runs=0
cargo afl build
cargo hfuzz build

# Create Fuzzit job
if [ $1 == "fuzzing" ]; then
    ./fuzzit auth ${FUZZIT_API_KEY}
    ./fuzzit create job --branch $(git branch --show-current) --revision $GITHUB_SHA zebra/crypto ./fuzz/target/x86_64-unknown-linux-gnu/debug/crypto
else
    ./fuzzit create job --local zebra/crypto ./fuzz/target/x86_64-unknown-linux-gnu/debug/crypto
fi
