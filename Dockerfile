### Base image
FROM fedora:31 AS base

### Downloader image to download sources
FROM alpine AS downloader
RUN apk add git wget

### Base image used to build stuff
FROM base AS base-builder
RUN dnf install -y --refresh \
        cmake \
        clang \
        file \
        gcc-c++ \
        git \
        llvm-devel \
        make \
        wget \
        xz && \
    sed --in-place=.orig \
        's/if (ARG_SHARED)/if (ARG_SHARED OR ARG_MODULE)/' \
        /usr/lib64/cmake/llvm/AddLLVM.cmake
WORKDIR /work
RUN useradd -ms /bin/bash coll && chown -R coll:coll /work
USER coll
RUN llvm-config --version > /work/llvm-version

### gllvm image
FROM golang:1.14 AS gllvm
RUN go get github.com/SRI-CSL/gllvm/cmd/...

### LLVM passes image
FROM base-builder AS llvm-passes
USER root
ENV HOME /root
RUN dnf install -y --refresh \
        boost \
        boost-devel \
        boost-static \
        boost-program-options \
        ninja-build
ENV PATH="${HOME}/.cargo/bin:${PATH}"
RUN curl --proto '=https' --tlsv1.2 -sSf --output /tmp/rustup-init \
        https://sh.rustup.rs && \
    chmod +x /tmp/rustup-init && \
    /tmp/rustup-init -y && \
    rustup install nightly
COPY llvm-passes llvm-passes/
WORKDIR /work/llvm-passes/build
RUN rm -f CMakeCache.txt && \
    rustup override set nightly \
        --path ../input-bytes-tracer-pass && \
    cmake \
        -GNinja \
        -DCMAKE_INSTALL_PREFIX=/usr \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_C_COMPILER=clang \
        -DCMAKE_CXX_COMPILER=clang++ \
        -DBUILD_TESTING=OFF \
        .. && \
    ninja && \
    cpack

### LAVA bitcode image
FROM base-builder AS lava-bc
USER root
RUN dnf install -y --refresh libacl-devel
USER coll
RUN wget http://panda.moyix.net/~moyix/lava_corpus.tar.xz && \
    tar xf lava_corpus.tar.xz
COPY --from=gllvm /go/bin/gclang /usr/bin/gclang
COPY --from=gllvm /go/bin/get-bc /usr/bin/get-bc
COPY misc/lava/coreutils-8.24-on-glibc-2.28.patch coreutils-8.24-on-glibc-2.28.patch
COPY misc/lava/build_lava_bc.sh build_lava_bc.sh
RUN ./build_lava_bc.sh

### LAVA analysis_binaries
FROM base-builder AS lava
USER root
RUN dnf install -y --refresh \
        boost-static \
        boost-program-options
COPY --from=llvm-passes /work/llvm-passes/build/AnalysisPasses-0.1.1-Linux.sh /work
RUN /work/AnalysisPasses-0.1.1-Linux.sh --skip-license --prefix=/usr
USER coll
COPY --from=lava-bc --chown=coll:coll /work/lava-bc /work/lava-bc/
WORKDIR /work/lava-bc
COPY misc/lava/build_lava_ab.sh build_lava_ab.sh
USER coll
RUN ./build_lava_ab.sh

### binutils bitcode
FROM base-builder AS binutils-bc
COPY --from=gllvm /go/bin/gclang /usr/bin/gclang
COPY --from=gllvm /go/bin/get-bc /usr/bin/get-bc
USER root
RUN dnf install -y --refresh texinfo
USER coll
COPY misc/binutils/build_binutils_bc.sh build_binutils_bc.sh
RUN ./build_binutils_bc.sh

### binutils analysis_binaries
FROM base-builder AS binutils
USER root
COPY --from=llvm-passes /work/llvm-passes/build/AnalysisPasses-0.1.1-Linux.sh /work
RUN dnf install -y --refresh \
        boost-static \
        boost-program-options
RUN /work/AnalysisPasses-0.1.1-Linux.sh --skip-license --prefix=/usr
USER coll
COPY --from=binutils-bc --chown=coll:coll /work/binutils-bc /work/binutils-bc/
COPY misc/binutils/build_binutils_ab.sh build_binutils_ab.sh
RUN ./build_binutils_ab.sh

### llvm-project-src
FROM downloader AS llvm-project-src
WORKDIR /work/llvm-project
COPY --from=base-builder /work/llvm-version /work/llvm-version
RUN export LLVM_VER=`cat /work/llvm-version` && \
    wget https://github.com/llvm/llvm-project/releases/download/llvmorg-$LLVM_VER/llvm-$LLVM_VER.src.tar.xz && \
    tar xf llvm-$LLVM_VER.src.tar.xz && \
    rm llvm-$LLVM_VER.src.tar.xz && \
    mv llvm-$LLVM_VER.src llvm
RUN export LLVM_VER=`cat /work/llvm-version` && \
    wget https://github.com/llvm/llvm-project/releases/download/llvmorg-$LLVM_VER/libcxx-$LLVM_VER.src.tar.xz && \
    tar xf libcxx-$LLVM_VER.src.tar.xz && \
    rm libcxx-$LLVM_VER.src.tar.xz && \
    mv libcxx-$LLVM_VER.src libcxx
RUN export LLVM_VER=`cat /work/llvm-version` && \
    wget https://github.com/llvm/llvm-project/releases/download/llvmorg-$LLVM_VER/libcxxabi-$LLVM_VER.src.tar.xz && \
    tar xf libcxxabi-$LLVM_VER.src.tar.xz && \
    rm libcxxabi-$LLVM_VER.src.tar.xz && \
    mv libcxxabi-$LLVM_VER.src libcxxabi
RUN export LLVM_VER=`cat /work/llvm-version` && \
    wget https://github.com/llvm/llvm-project/releases/download/llvmorg-$LLVM_VER/compiler-rt-$LLVM_VER.src.tar.xz && \
    tar xf compiler-rt-$LLVM_VER.src.tar.xz && \
    rm compiler-rt-$LLVM_VER.src.tar.xz && \
    mv compiler-rt-$LLVM_VER.src compiler-rt
RUN apk add patch
COPY misc/google/0001-Add-support-to-build-libcxx-and-libcxxabi-with-DFSan.patch llvm_dfsan.patch
RUN patch -p1 < llvm_dfsan.patch

### llvm-project
FROM base-builder AS llvm-project
USER root
WORKDIR /work
RUN dnf install -y --refresh ninja-build
COPY --from=llvm-project-src /work/llvm-project /work/llvm-project/

### dfsan_libcxx
FROM llvm-project AS dfsan-libcxx
RUN mkdir llvm-build && \
    cd llvm-build && \
    cmake -G Ninja ../llvm-project/llvm \
        -DLLVM_LIBDIR_SUFFIX=64 \
        -DLLVM_ENABLE_PROJECTS='libcxx;libcxxabi' \
        -DLLVM_USE_SANITIZER=DataFlow \
        -DCMAKE_C_COMPILER=clang \
        -DCMAKE_CXX_COMPILER=clang++ \
        -DCMAKE_C_FLAGS="-fsanitize-blacklist=/usr/lib64/clang/`llvm-config --version`/dfsan_abilist.txt" \
        -DCMAKE_CXX_FLAGS="-fsanitize-blacklist=/usr/lib64/clang/`llvm-config --version`/dfsan_abilist.txt" \
        -DLIBCXX_ENABLE_SHARED=OFF \
        -DLIBCXXABI_ENABLE_SHARED=OFF && \
    ninja cxx cxxabi
RUN cp llvm-build/lib64/libc++abi.a /usr/lib64/dfsan_libc++abi.a && \
    cp llvm-build/lib64/libc++.a /usr/lib64/dfsan_libc++.a && \
    rm -rf llvm-build

### icount_libcxx
FROM llvm-project AS icount-libcxx
COPY --from=llvm-passes /work/llvm-passes/build/AnalysisPasses-0.1.1-Linux.sh /work
RUN /work/AnalysisPasses-0.1.1-Linux.sh --skip-license --prefix=/usr
RUN mkdir llvm-build-icount && \
    cd llvm-build-icount && \
    ICOUNT_WRAPPER_FORWARD=1 cmake -G Ninja ../llvm-project/llvm \
        -DLLVM_LIBDIR_SUFFIX=64 \
        -DLLVM_ENABLE_PROJECTS='libcxx;libcxxabi' \
        -DCMAKE_C_COMPILER=clang_icount \
        -DCMAKE_CXX_COMPILER=clang_icount++ \
        -DLIBCXX_ENABLE_SHARED=OFF \
        -DLIBCXXABI_ENABLE_SHARED=OFF && \
    ninja cxx cxxabi
RUN cp llvm-build-icount/lib64/libc++abi.a /usr/lib64/icount_libc++abi.a && \
    cp llvm-build-icount/lib64/libc++.a /usr/lib64/icount_libc++.a && \
    rm -r llvm-build-icount

### Google fuzzer test suite source
FROM downloader AS google-src
WORKDIR /work
RUN git clone https://github.com/google/fuzzer-test-suite.git
COPY misc/google/build.sh /work/fuzzer-test-suite/build.sh
COPY misc/google/001-fedora-build.patch /work/fuzzer-test-suite/
RUN cd fuzzer-test-suite && git apply 001-fedora-build.patch

### GSS build, because apparently not in fedora repo
FROM base AS gss-build
RUN dnf install -y --refresh \
        gcc \
        make \
        wget
WORKDIR /work
RUN wget ftp://ftp.gnu.org/gnu/gss/gss-1.0.3.tar.gz && \
    tar xf gss-1.0.3.tar.gz && \
    rm gss-1.0.3.tar.gz && \
    mv gss-1.0.3 gss && \
    cd gss && \
    ./configure --prefix=/usr --libdir=/usr/lib64 && \
    make -j

### Google fuzzer test suite bitcode
FROM base-builder AS google-build
USER root
RUN dnf install -y --refresh \
        libcxx libcxx-devel \
        libtool \
        which \
        golang \
        libarchive-devel glib2-devel libxml2-devel libgcrypt-devel \
        openssl-devel zlib-devel bzip2-devel xz-devel \
        libvorbis-devel libogg-devel ragel-devel nasm \
        autoconf-archive dbus-devel readline-devel lcov
COPY --from=gss-build /work/gss /work/gss
RUN cd /work/gss && make install
COPY --from=google-src /work/fuzzer-test-suite /work/fuzzer-test-suite
COPY --from=gllvm /go/bin/gclang /usr/bin/gclang
COPY --from=gllvm /go/bin/gclang++ /usr/bin/gclang++
COPY --from=llvm-project-src /work/llvm-project /work/llvm-project/
WORKDIR /work/build-test-suite
COPY misc/google/gen_library_abilist.sh /work/build-test-suite
COPY misc/google/build_fuzzer_test_suite.py /work/build-test-suite/build_fuzzer_test_suite.py
RUN python3 build_fuzzer_test_suite.py parallel

### Google fuzzer test suite analysis_binaries
FROM google-build AS google
RUN dnf install -y --refresh \
        python3-pip \
        boost-static \
        boost-program-options && \
    pip3 install pylddwrap
COPY --from=llvm-passes /work/llvm-passes/build/AnalysisPasses-0.1.1-Linux.sh /work
RUN /work/AnalysisPasses-0.1.1-Linux.sh --skip-license --prefix=/usr
COPY --from=dfsan-libcxx /usr/lib64/dfsan_libc++.a /usr/lib64/dfsan_libc++.a
COPY --from=dfsan-libcxx /usr/lib64/dfsan_libc++abi.a /usr/lib64/dfsan_libc++abi.a
COPY --from=icount-libcxx /usr/lib64/icount_libc++.a /usr/lib64/icount_libc++.a
COPY --from=icount-libcxx /usr/lib64/icount_libc++abi.a /usr/lib64/icount_libc++abi.a
COPY --from=gllvm /go/bin/get-bc /usr/bin/get-bc
COPY misc/google/llvm.cpp llvm.cpp
COPY misc/google/llvm.c llvm.c
COPY misc/google/build_fuzzer_test_suite_bc.py /work/build-test-suite/build_fuzzer_test_suite_bc.py
ENV LD_LIBARY_PATH=/usr/local/lib64
RUN python3 build_fuzzer_test_suite_bc.py

### Framework
FROM base-builder AS framework
USER root
RUN dnf install -y --refresh \
        cargo \
        protobuf-compiler \
        sqlite-devel \
        zeromq-devel
USER coll
COPY --chown=coll:coll framework /work/collab-fuzz/framework/
WORKDIR /work/collab-fuzz/framework
RUN cargo install --root ~/.local --locked --path .

### Base runtime image
FROM base AS runtime
RUN dnf install -y --refresh zeromq
WORKDIR /work
COPY --from=llvm-passes /work/llvm-passes/build/AnalysisPasses-0.1.1-Linux.sh /work
RUN /work/AnalysisPasses-0.1.1-Linux.sh --skip-license --prefix=/usr
RUN useradd -ms /bin/bash coll
RUN mkdir /in
RUN mkdir /data && chown -R coll:coll /data
COPY docker/server/entry.sh /entry.sh
USER coll
WORKDIR /home/coll
RUN mkdir analysis_binaries
ENV INPUT_DIR=in OUTPUT_DIR=out ANALYSIS_BIN_DIR=/home/coll/analysis_binaries/ SCHEDULER=enfuzz
ENV LD_LIBRARY_PATH=/usr/local/lib64
ENV PATH=/home/coll/.local/bin/:$PATH
CMD ["/entry.sh"]

### Framework + LAVA
FROM runtime AS framework-lava
COPY --from=lava /work/analysis_binaries /home/coll/analysis_binaries/
COPY --from=framework /home/coll/.local/bin/collab_fuzz_server \
                      /home/coll/.local/bin/collab_fuzz_server
COPY --from=framework /home/coll/.local/bin/collab_fuzz_pass_runner \
                      /home/coll/.local/bin/collab_fuzz_pass_runner

### Framework + binutils
FROM runtime AS framework-binutils
COPY --from=binutils /work/analysis_binaries /home/coll/analysis_binaries/
COPY --from=framework /home/coll/.local/bin/collab_fuzz_server \
                      /home/coll/.local/bin/collab_fuzz_server
COPY --from=framework /home/coll/.local/bin/collab_fuzz_pass_runner \
                      /home/coll/.local/bin/collab_fuzz_pass_runner

### Framework + google
FROM runtime AS framework-google
USER root
RUN dnf install -y --refresh \
        make \
        libcxx \
        libarchive glib2 libxml2 libgcrypt \
        openssl zlib bzip2 xz \
        libvorbis libogg ragel nasm \
        autoconf dbus readline lcov
COPY --from=gss-build /work/gss /work/gss
RUN cd /work/gss && make install
USER coll
COPY --from=google /home/coll/analysis_binaries /home/coll/analysis_binaries/
COPY --from=framework /home/coll/.local/bin/collab_fuzz_server \
                      /home/coll/.local/bin/collab_fuzz_server
COPY --from=framework /home/coll/.local/bin/collab_fuzz_pass_runner \
                      /home/coll/.local/bin/collab_fuzz_pass_runner
