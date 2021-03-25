#!/usr/bin/env python3

from __future__ import print_function
import os, sys

targets = [
  "boringssl-2016-02-12",
  "c-ares-CVE-2016-5180",
  "freetype2-2017",
  "guetzli-2017-3-30",
  "harfbuzz-1.3.2",
  "json-2017-02-12",
  "lcms-2017-03-21",
  "libarchive-2017-01-04",
  "libjpeg-turbo-07-2017",
  "libpng-1.2.56",
#  "libssh-2017-1272",
#  "libxml2-v2.9.2",
#  "llvm-libcxxabi-2017-01-27",
  "openssl-1.0.1f",
  "openssl-1.0.2d",
  "openssl-1.1.0c",
  "openthread-2018-02-27",
  "pcre2-10.00",
  "proj4-2017-08-14",
  "re2-2014-12-09",
  "sqlite-2016-11-14",
  "vorbis-2017-12-11",
  "woff2-2016-05-06",
  "wpantund-2018-02-27"
]

for target in targets:
    ret = os.system("CC=clang CXX=clang++ LIBFUZZER_SRC=/work/compiler-rt-9.0.0.src/lib/fuzzer/ AFL_SRC=/work/afl FUZZING_ENGINE=afl /work/fuzzer-test-suite/build.sh {}".format(target))
    if ret != 0:
        print("Failed building google fuzzer-test-suite", file=sys.stderr)
        sys.exit(1)
