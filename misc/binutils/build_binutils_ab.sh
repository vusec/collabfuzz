#!/bin/bash
set -euo pipefail

mkdir /work/analysis_binaries
cd /work/binutils-bc
# XXX: the following OOMs on 16GB if run in parallel
for t in ./*.bc; do
    fb=$(basename -s .bc "$t")
    mkdir "$fb".analysis_binaries
    collab_fuzz_wrapper "$fb.analysis_binaries" "$t" -ldl
    mv "$fb".analysis_binaries /work/analysis_binaries/"$fb".analysis_binaries
done
