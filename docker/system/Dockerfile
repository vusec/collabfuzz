FROM ubuntu:18.04

RUN useradd -ms /bin/bash coll

RUN apt-get update && apt-get install -y wget make build-essential gcc libbfd-dev libunwind-dev libipt-dev binutils-dev git libtool libtool-bin automake bison libglib2.0-dev python python-pip cmake curl libc-ares-dev libfreetype6-dev libarchive-dev ragel bzip2 libgss-dev subversion libgcrypt-dev autoconf-archive virtualenv libdbus-glib-1-dev libgirepository1.0-dev libturbojpeg libssh-dev libssl-dev nasm libbz2-dev libxml2-dev software-properties-common libacl1-dev
RUN pip install watchdog
RUN env python -m pip install watchdog

RUN echo "deb http://apt.llvm.org/bionic/ llvm-toolchain-bionic-9 main" >> /etc/apt/sources.list
RUN echo "deb-src http://apt.llvm.org/bionic/ llvm-toolchain-bionic-9 main" >> /etc/apt/sources.list
RUN apt-key adv --keyserver keyserver.ubuntu.com --recv-keys 15CF4D18AF4F7421
RUN apt-get update && apt-get install -y clang-9 llvm-9-dev lld-9 lld-9 clangd-9
#RUN rm /usr/bin/clang && rm /usr/bin/clang++ && rm /usr/bin/llvm-config
RUN ln -s /usr/bin/clang-9 /usr/bin/clang
RUN ln -s /usr/bin/clang++-9 /usr/bin/clang++
RUN ln -s /usr/bin/llvm-config-9 /usr/bin/llvm-config

RUN add-apt-repository -y ppa:longsleep/golang-backports && apt-get update && apt-get install -y golang-go
RUN go get github.com/SRI-CSL/gllvm/cmd/...

RUN mkdir /targets && chown -R coll:coll /targets

USER coll
WORKDIR /home/coll
