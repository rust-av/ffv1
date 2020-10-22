#!/bin/sh

RUNS=2

hyperfine -r ${RUNS} \
    -L program c,go,rust \
    'builds/{program}-ffv1 ../data/ffv1_v3.mkv' \
    --export-csv ffv1-bench.csv \
    --export-markdown ffv1-bench.md
