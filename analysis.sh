#!/bin/bash
lizard src -C 7 -V -L 40 | tail -3 > lizard.out
./rust-code-analysis.py
cat rust-code-analysis.tab | grep cyclomatic | grep average | cut -f 5 > cyclomatic.out
sed "s/^AvgCCN.*/AvgCCN $(cat cyclomatic.out)/" README.md > README.md.tmp
mv README.md.tmp README.md
rm cyclomatic.out
