#!/bin/bash

# Lizard
lizard src -C 7 -V -L 40 | tail -3 > code-metrics/lizard.out

# tarpaulin
cargo tarpaulin -o html --output-dir code-metrics
cargo tarpaulin -o json --output-dir code-metrics

# rust-code-analysis
./rust-code-analysis.py
mv rust-code-analysis.tab code-metrics/rust-code-analysis.tab

# grcov
mkdir -p target/debug/coverage
rm -rf target/debug/coverage/*
mkdir -p profraw
rm profraw/*
RUSTFLAGS_TMP=$RUSTFLAGS
LLVM_PROFILE_FILE_TMP=$LLVM_PROFILE_FILE
export RUSTFLAGS="-Cinstrument-coverage"
cargo build
export LLVM_PROFILE_FILE="profraw/wikibase_rest_api-%p-%m.profraw"
cargo test
grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/coverage/

export LLVM_PROFILE_FILE="$LLVM_PROFILE_FILE_TMP"
export RUSTFLAGS="$RUSTFLAGS_TMP"

# Update README.md
cat code-metrics/rust-code-analysis.tab | grep cyclomatic | grep average | cut -f 5 > cyclomatic.out
sed "s/^AvgCCN.*/AvgCCN $(cat cyclomatic.out)/" README.md > README.md.tmp
mv README.md.tmp README.md
rm cyclomatic.out

#jq '.coverage*100|round/100' code-metrics/tarpaulin-report.json > coverage.out # tarpaulin misses some code that clearly is covered
jq --raw-output '.message' target/debug/coverage/html/coverage.json > coverage.out
sed "s/^Codecov .*/Codecov $(cat coverage.out)/" README.md > README.md.tmp
mv README.md.tmp README.md
rm coverage.out
