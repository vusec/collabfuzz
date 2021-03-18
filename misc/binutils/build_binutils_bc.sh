#!/bin/bash
set -euo pipefail

wget https://ftp.gnu.org/gnu/binutils/binutils-2.31.1.tar.gz && tar xfv binutils-2.31.1.tar.gz
cd binutils-2.31.1/ 
CC=gclang CXX=gclang ./configure --prefix="$(pwd)/binutils-prefix/"
make -j"$(nproc)"
make install

mkdir /work/binutils-bc
cd binutils-prefix/bin/
for t in *; do (
    get-bc "$t"
    mv "$t".bc /work/binutils-bc/
    ) &
done;
wait;
