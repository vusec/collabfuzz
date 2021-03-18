#!/usr/bin/env bash

set -euo pipefail

mkdir /work/lava-bc
cd lava_corpus/LAVA-M

do_bin() {
    cd "$1/coreutils-8.24-lava-safe"
    git apply "../../../../coreutils-8.24-on-glibc-2.28.patch"
    CC=gclang ./configure --prefix="$(pwd)/lava-install" LIBS="-lacl"
    make -j$(($(nproc) / 4))
    make install
    get-bc -o "/work/lava-bc/$1.bc" "lava-install/bin/$1"
    cd ../../
}

do_bin "base64" &
do_bin "md5sum" &
do_bin "uniq" &
do_bin "who" &
wait;

cd ../../
