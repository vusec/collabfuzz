#!/bin/bash

set -e

echo "==============================================================="
echo "==============================================================="
echo "==============================================================="
# rm -rf ./build &&
CC=clang CXX=clang++ cmake -DCMAKE_INSTALL_PREFIX=/work/install/ -DCMAKE_BUILD_TYPE=Debug -S ./external/id-assigner-pass -B ./build/id-assigner-pass && cd ./build/id-assigner-pass && make && make install && cd ../..

echo "==============================================================="

CC=clang CXX=clang++ cmake -DCMAKE_INSTALL_PREFIX=/work/install/ -DCMAKE_BUILD_TYPE=Debug -S . -B ./build/bb_reachability && cd ./build/bb_reachability && make && make install && cd ../..

echo "==============================================================="
pwd

ls -la /work/install/lib/LLVMBBReachability.so

    # | opt -load /work/install/lib/LLVMBBReachability.so -bbids \
        # | opt -load /work/install/lib/LLVMBBReachability.so -inst-bbids \
rm -r tmp/*
for EXEC_NAME in "addr2line" "ar" "as-new" "bfdtest1" "bfdtest2" "chew" "cxxfilt" "elfedit" "gdb" "gdbreplay" "gdbserver" "gprof" "ld-new" "nm-new" "objcopy" "objdump" "ranlib" "readelf" "size" "strings" "strip-new" "sysinfo"
do
    echo "graph for: ${EXEC_NAME}"
    cat /work/examples/binutils-bc/$EXEC_NAME.bc | opt -load /work/install/lib/LLVMIDAssigner.so -load /work/install/lib/LLVMBBReachability.so -bb-reach -bb-reach-output tmp/bb_graph.json -o tmp/$EXEC_NAME.bc
    mkdir tmp/${EXEC_NAME}/
    mv tmp/bb_graph.json tmp/${EXEC_NAME}
    # && clang -o tmp/$EXEC_NAME tmp/$EXEC_NAME.bc
done

# -filetype=obj
