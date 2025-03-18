#!/bin/bash

# Lizard
lizard src -C 7 -V -L 40 | tail -3 > code-metrics/lizard.out

# tarpaulin
cargo tarpaulin -o html --output-dir code-metrics
cargo tarpaulin -o json --output-dir code-metrics

# rust-code-analysis
./rust-code-analysis.py
mv rust-code-analysis.tab code-metrics/rust-code-analysis.tab

# Update README.md
cat code-metrics/rust-code-analysis.tab | grep cyclomatic | grep average | cut -f 5 > cyclomatic.out
sed "s/^AvgCCN.*/AvgCCN $(cat cyclomatic.out)/" README.md > README.md.tmp
mv README.md.tmp README.md
rm cyclomatic.out

jq '.coverage*100|round/100' code-metrics/tarpaulin-report.json > coverage.out
sed "s/^Codecov .*/Codecov $(cat coverage.out)%/" README.md > README.md.tmp
mv README.md.tmp README.md
rm coverage.out
