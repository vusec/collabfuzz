#!/usr/bin/env python3
import sys
import os
import subprocess
import argparse
from concurrent import futures

TARGETS = [
    "boringssl-2016-02-12", "c-ares-CVE-2016-5180", "freetype2-2017",
    "guetzli-2017-3-30", "harfbuzz-1.3.2", "json-2017-02-12",
    "lcms-2017-03-21", "libarchive-2017-01-04", "libjpeg-turbo-07-2017",
    "libpng-1.2.56", 
    #"libssh-2017-1272", "libxml2-v2.9.2", "llvm-libcxxabi-2017-01-27", 
    "openssl-1.0.1f", "openssl-1.0.2d",
    "openssl-1.1.0c", "openthread-2018-02-27", "pcre2-10.00",
    "proj4-2017-08-14", "re2-2014-12-09", "sqlite-2016-11-14",
    "vorbis-2017-12-11", "woff2-2016-05-06", "wpantund-2018-02-27"
]


def build_target(target, serial):
    env_vars = {
        'CC': 'gclang',
        'CXX': 'gclang++',
        'CFLAGS': ' ',
        'CXXFLAGS': '-fPIC -stdlib=libc++',
        'LIBFUZZER_SRC': '/work/llvm-project/compiler-rt/lib/fuzzer/',
        'FUZZING_ENGINE': 'coverage'
    }

    if serial:
        stdout = None
        stderr = None
    else:
        stdout = subprocess.PIPE
        stderr = subprocess.PIPE

    env = os.environ.copy()
    env.update(env_vars)
    return subprocess.run(['/work/fuzzer-test-suite/build.sh', target],
                          env=env,
                          stdout=stdout,
                          stderr=stderr,
                          text=True)


def start_build(targets, serial):
    if serial:
        num_workers = 1
    else:
        # Limit workers to CPU count since the builds are themselves parallel
        num_workers = os.cpu_count() // 2

    with futures.ThreadPoolExecutor(num_workers) as executor:
        future_to_target = {}
        for target in targets:
            future = executor.submit(build_target, target, serial)
            future_to_target[future] = target

        for future in futures.as_completed(future_to_target):
            completed = future.result()
            if completed.returncode != 0:
                print(
                    f'Failed building {future_to_target[future]}! Output was:',
                    completed.stdout,
                    "STDERR",
                    completed.stderr,
                    sep="\n",
                    file=sys.stderr)
                sys.exit(1)

            print(f'Built: {future_to_target[future]}', flush=True)


def main(args):
    print(f'Beginning to build in {args.mode} mode', flush=True)
    start_build(TARGETS, args.mode == 'serial')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('mode',
                        choices=['serial', 'parallel'],
                        default='parallel')
    args = parser.parse_args()
    main(args)
