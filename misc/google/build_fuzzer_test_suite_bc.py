#!/usr/bin/env python3
import os, sys
import lddwrap
import pathlib

mappings = {
    "boringssl": "boringssl-2016-02-12",
    "c-ares": "c-ares-CVE-2016-5180",
    "freetype2": "freetype2-2017",
    "guetzli": "guetzli-2017-3-30",
    "harfbuzz": "harfbuzz-1.3.2",
    "json": "json-2017-02-12",
    "lcms": "lcms-2017-03-21",
    "libarchive": "libarchive-2017-01-04",
    "libjpeg-turbo": "libjpeg-turbo-07-2017",
    "libpng": "libpng-1.2.56",
#    "libssh": "libssh-2017-1272",
#    "libxml2": "libxml2-v2.9.2",
#    "llvm-libcxxabi": "llvm-libcxxabi-2017-01-27",
    "openssl-1.0.1f": "openssl-1.0.1f",
    "openssl-1.0.2d": "openssl-1.0.2d",
    "openssl-1.1.0c-bignum": "openssl-1.1.0c-bignum",
    "openssl-1.1.0c-x509": "openssl-1.1.0c-x509",
    "openthread-ip6": "openthread-2018-02-27-ip6",
    "openthread-radio": "openthread-2018-02-27-radio",
    "pcre2": "pcre2-10.00",
    "proj4": "proj4-2017-08-14",
    "re2": "re2-2014-12-09",
    "sqlite": "sqlite-2016-11-14",
    "vorbis": "vorbis-2017-12-11",
    "woff2": "woff2-2016-05-06",
    "wpantund": "wpantund-2018-02-27",
}

inv_mappings = {v: k for k, v in mappings.items()}

targets = {
    "boringssl-2016-02-12": ("-lpthread -ldl", "BUILD/crypto/libcrypto.a"),
    "c-ares-CVE-2016-5180": ("", ""),
    "freetype2-2017": ("-larchive -lz", ""),
    "guetzli-2017-3-30": ("-lm", ""),
    "harfbuzz-1.3.2": ("-lglib-2.0", ""),
    "json-2017-02-12": ("-lm", ""),
    "lcms-2017-03-21": ("-lm", "BUILD/src/.libs/liblcms2.a"),
    "libarchive-2017-01-04": ("-lz -lbz2 -lxml2 -lcrypto -lssl -llzma", ""),
    "libjpeg-turbo-07-2017": ("", "BUILD/.libs/libturbojpeg.a"),
    "libpng-1.2.56": ("-lz -lm", ""),
#    "libssh-2017-1272": ("-lcrypto -lgss -lz", ""),
#    "libxml2-v2.9.2": ("-lz -lm", ""),
#    "llvm-libcxxabi-2017-01-27": ("", ""),
    "openssl-1.0.1f":
    ("-lpthread -ldl -DCERT_PATH=.", "BUILD/libcrypto.a BUILD/libssl.a"),
    "openssl-1.0.2d": ("-lpthread -ldl -lgcrypt -DCERT_PATH=.",
                       "BUILD/libcrypto.a BUILD/libssl.a"),
    "openssl-1.1.0c": ("-lpthread -ldl", "BUILD/libcrypto.a BUILD/libssl.a"),
    "openthread-2018-02-27": ("", ""),
    "pcre2-10.00": ("-lm", ""),
    "proj4-2017-08-14": ("-lm -lpthread", ""),
    "re2-2014-12-09": ("-lm -lpthread", ""),
    "sqlite-2016-11-14": ("-lpthread -ldl", ""),
    "vorbis-2017-12-11":
    ("-lm -L INSTALL/lib/ -lvorbisfile  -lvorbis -logg", ""),
    "woff2-2016-05-06": ("-lm", ""),
    "wpantund-2018-02-27": ("-lutil", "")
}

special_targets = {
    "openssl-1.1.0c":
    ["openssl-1.1.0c-coverage-bignum", "openssl-1.1.0c-coverage-x509"],
    "openthread-2018-02-27": [
        "openthread-2018-02-27-coverage-ip6",
        "openthread-2018-02-27-coverage-radio"
    ]
}

cxx_targets = [
    "c-ares-CVE-2016-5180",
    "freetype2-2017",
    "guetzli-2017-3-30",
    "harfbuzz-1.3.2",
    "json-2017-02-12",
    "libarchive-2017-01-04",
    "libjpeg-turbo-07-2017",
    "libpng-1.2.56",
#    "libssh-2017-1272",
#    "libxml2-v2.9.2",
#    "llvm-libcxxabi-2017-01-27",
    "openssl-1.0.1f",
    "openssl-1.0.2d",
    "openssl-1.1.0c",
    "openthread-2018-02-27",
    "pcre2-10.00",
    "proj4-2017-08-14",
    "re2-2014-12-09",
    "woff2-2016-05-06",
    "wpantund-2018-02-27",
]

LIB_BLACKLIST = [
    "libc++.so", "libc++abi.so", "libc.so", "libgcc_s.so", "libdl.so",
    "ld-linux-x86-64.so", "linux-vdso.so"
]

NO_LLVM_CPP = ["proj4-2017-08-14"]


def get_runner_name(target_bin):
    return inv_mappings[target_bin.replace("-coverage", "")]


def so_in_blacklist(soname):
    if soname is None:
        return False
    return any(map(lambda x: x in soname, LIB_BLACKLIST))


def get_target_bins(target):
    targets = special_targets.get(target, [f"{target}-coverage"])
    return targets


def gen_abilist(target_bin):
    p = pathlib.Path(target_bin)
    r = lddwrap.list_dependencies(path=p, env=os.environ.copy())
    for dep in r:
        if (not dep.path or dep.found == False or dep.unused == True
                or not os.path.isabs(dep.path)):
            continue
        if dep.soname is None:
            continue
        if so_in_blacklist(dep.soname):
            continue

        ret = os.system(
            f"../gen_library_abilist.sh {dep.path} discard >> custom_abilist.txt"
        )
        if ret != 0:
            print(f'Could not generate abilist for {target_bin}')
            sys.exit(1)


def main():
    for target in targets:
        old_cwd = os.getcwd()
        bdir = f"RUNDIR-{target}"
        os.chdir(bdir)
        for target_bin in get_target_bins(target):
            os.system(f"get-bc {target_bin}")
            flags, libs = targets[target]
            bc_file = f"{target_bin}.bc"

            gen_abilist(target_bin)

            cxx_flag = '--cxx' if target in cxx_targets else ''

            if target not in NO_LLVM_CPP:
                libs += ' ../llvm.c'

            runner_name = get_runner_name(target_bin)

            cmd = (
                f'collab_fuzz_wrapper ' +
                f'--custom-abilist $(pwd)/custom_abilist.txt {cxx_flag} ' +
                f'/home/coll/analysis_binaries/{runner_name}.analysis_binaries '
                + f'{bc_file} {flags} {libs}')
            ret = os.system(cmd)
            if ret != 0:
                print(f"Error building {target_bin}")
                print(cmd)
                sys.exit(1)
        os.chdir(old_cwd)


if __name__ == '__main__':
    main()
