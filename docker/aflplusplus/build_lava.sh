#!/usr/bin/env bash

set -e

cd lava_corpus/LAVA-M

do_afl_bin() {
    cd "$1/coreutils-8.24-lava-safe"
    git apply "../../../../coreutils-8.24-on-glibc-2.28.patch"
    ./configure --prefix="/targets/aflplusplus-lava/$1" LIBS="-lacl"
    make -j`nproc`
    make install
    make clean
    cd ../../
}

do_bin() {
    cd "$1/coreutils-8.24-lava-safe"
    #git apply "~/patches/coreutils-8.24-on-glibc-2.28.patch"
    CC=gclang ./configure --prefix="`pwd`/lava-install" LIBS="-lacl"
    make -j`nproc`
    make install
    get-bc -o "../../$1.bc" "lava-install/bin/$1"
    make clean
    cd ../../
    mkdir "$1.analysis_binaries"
    collab_fuzz_wrapper "$1.analysis_binaries" "$1.bc"
}

do_afl_bin "base64"
do_afl_bin "md5sum"
do_afl_bin "uniq"
do_afl_bin "who"
cd ~
