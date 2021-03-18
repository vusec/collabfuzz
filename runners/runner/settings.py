BIN2SUITE = {
    "objdump": ("binutils", "objdump"),
    "addr2line": ("binutils", "addr2line"),
    "ar": ("binutils", "ar"),
    "strings": ("binutils", "strings"),
    #"nm-new": ("binutils", "nm-new"),
    "nm": ("binutils", "nm"),
    "readelf": ("binutils", "readelf"),
    #"strip-new": ("binutils", "strip-new"),
    "strip": ("binutils", "strip"),
    "boringssl": ("google-test-suite", "boringssl-2016-02-12"),
    "c-ares": ("google-test-suite", "c-ares-CVE-2016-5180"),
    "freetype2": ("google-test-suite", "freetype2-2017"),
    "guetzli": ("google-test-suite", "guetzli-2017-3-30"),
    "harfbuzz": ("google-test-suite", "harfbuzz-1.3.2"),
    "json": ("google-test-suite", "json-2017-02-12"),
    "lcms": ("google-test-suite", "lcms-2017-03-21"),
    "libarchive": ("google-test-suite", "libarchive-2017-01-04"),
    "libjpeg-turbo": ("google-test-suite", "libjpeg-turbo-07-2017"),
    "libpng": ("google-test-suite", "libpng-1.2.56"),
    "libssh": ("google-test-suite", "libssh-2017-1272"),
    "libxml2": ("google-test-suite", "libxml2-v2.9.2"),
    "llvm-libcxxabi": ("google-test-suite", "llvm-libcxxabi-2017-01-27"),
    "openssl-1.0.1f": ("google-test-suite", "openssl-1.0.1f"),
    "openssl-1.0.2d": ("google-test-suite", "openssl-1.0.2d"),
    #"openssl-1.1.0c": ("google-test-suite", "openssl-1.1.0c"),
    "openssl-1.1.0c-bignum": ("google-test-suite", "openssl-1.1.0c", "bignum"),
    "openssl-1.1.0c-x509": ("google-test-suite", "openssl-1.1.0c", "x509"),
    #"openthread": ("google-test-suite", "openthread-2018-02-27"),
    "openthread-ip6": ("google-test-suite", "openthread-2018-02-27", "ip6"),
    "openthread-radio": ("google-test-suite", "openthread-2018-02-27", "radio"),
    "pcre2": ("google-test-suite", "pcre2-10.00"),
    "proj4": ("google-test-suite", "proj4-2017-08-14"),
    "re2": ("google-test-suite", "re2-2014-12-09"),
    "sqlite": ("google-test-suite", "sqlite-2016-11-14"),
    "vorbis": ("google-test-suite", "vorbis-2017-12-11"),
    "woff2": ("google-test-suite", "woff2-2016-05-06"),
    "wpantund": ("google-test-suite", "wpantund-2018-02-27"),
    "base64": ("LAVA-M", "base64"),
    "md5sum": ("LAVA-M", "md5sum"),
    "uniq": ("LAVA-M", "uniq"),
    "who": ("LAVA-M", "who")
}

BIN2ARGS = {
    "addr2line": "-e @@",
    "ar": "-t @@",
    "strings": "-d @@",
    #"nm-new": "-a -C -l --synthetic @@",
    "nm": "-a -C -l --synthetic @@",
    "objdump": "--dwarf-check -C -g -f -dwarf -x @@",
    "readelf": "-a -c -w -I @@",
    #"strip-new": "-o /dev/null -s @@",
    "strip": "-o /dev/null -s @@",
    "xml": "@@",
    "gnuplot": "@@",
    "boringssl": "@@",
    "c-ares": "@@",
    "freetype2": "@@",
    "guetzli": "@@",
    "harfbuzz": "@@",
    "json": "@@",
    "lcms": "@@",
    "libarchive": "@@",
    "libjpeg-turbo": "@@",
    "libpng": "@@",
    "libssh": "@@",
    "libxml2": "@@",
    "llvm-libcxxabi": "@@",
    "openssl-1.0.1f": "@@",
    "openssl-1.0.2d": "@@",
    "openssl-1.1.0c": "@@",
    "openssl-1.1.0c-bignum": "@@",
    "openssl-1.1.0c-x509": "@@",
    "openthread": "@@",
    "openthread-ip6": "@@",
    "openthread-radio": "@@",
    "pcre2": "@@",
    "proj4": "@@",
    "re2": "@@",
    "sqlite": "@@",
    "vorbis": "@@",
    "woff2": "@@",
    "wpantund": "@@",
    "base64": "-d @@",
    "md5sum": "-c @@",
    "uniq": "@@",
    "who": "@@"
}

SUITEOPTS = {
    "binutils": "",
    "google-test-suite": "-m none",
    "LAVA-M": "",
    }

SUITEBIN = {
    "binutils": "/targets/binutils/bin/{bin}",
    "google-test-suite": "/targets/google/RUNDIR-{bin}/{bin}-afl{ext}",
    #"google-test-suite-plain": "/targets-plain/google/{bin}-coverage{ext}",
    "google-test-suite-plain": "/targets-plain/google/RUNDIR-{bin}/{bin}-coverage{ext}",
    "google-test-suite-libfuzzer": "/targets/google/RUNDIR-{bin}/{bin}-fsanitize_fuzzer{ext}",
    "google-test-suite-honggfuzz": "/targets/google/RUNDIR-{bin}/{bin}-honggfuzz{ext}",
    "LAVA-M": "/targets/lava/{bin}/bin/{bin}"
    }



ONLY_PLAIN = [("google-test-suite", "qsym")]

FUZZERS = {
       "afl": "afl-fuzz {afl_opts} -i {input_dir} -o {output_dir} -M afl -- {target_cmd}",
       "aflfast": "afl-fuzz {afl_opts} -p fast -i {input_dir} -o {output_dir} -M aflfast -- {target_cmd}",
       "fairfuzz": "afl-fuzz {afl_opts} -i {input_dir} -o {output_dir} -M fairfuzz -- {target_cmd}",
       "qsym": "/qsym/bin/run_qsym_afl.py -a framework -o {output_dir} -n qsym -- {target_cmd}",
       "radamsa": "afl-fuzz {afl_opts} -RR -i {input_dir} -o {output_dir} -M radamsa -- {target_cmd}",
       "honggfuzz": "honggfuzz -n 1 --input {output_dir}/seeds -z --covdir_all {output_dir}/queue  --crashdir {output_dir}/crashes -y {output_dir}/sync -Y 10 -- {target_cmd}",
       #"libfuzzer": "{target_cmd} --fork=1 {output_dir}/libfuzzer/queue {input_dir}",
       "libfuzzer": "{target_cmd} -fork=1 -ignore_crashes=1 -artifact_prefix={output_dir}/artifacts/ {output_dir}/queue",
}

def get_pre_cmd(fuzzer):
    if fuzzer == "honggfuzz":
        cmd = "mkdir -p {output_dir}/queue; mkdir -p {output_dir}/crashes; mkdir -p {output_dir}/sync; mkdir -p {output_dir}/seeds;"
        cmd += "for f in `ls {input_dir}`; do cp {input_dir}/$f {output_dir}/seeds/seed-${{f##*/}}; done;"
        return cmd
    if fuzzer == "libfuzzer":
        cmd =  "mkdir -p {output_dir}/queue; mkdir -p {output_dir}/artifacts;"
        cmd += "for f in `ls {input_dir}`; do cp {input_dir}/$f {output_dir}/queue/seed-${{f##*/}}; done;"
        return cmd
    if fuzzer == "qsym":
        cmd = "while [ ! -f {output_dir}/framework/fuzzer_stats ]; do sleep 1; done;"
        cmd += "cat {output_dir}/framework/fuzzer_stats"
        return cmd
    print(f"no pre cmd: {fuzzer}")
    return ""

def get_suitebin(suite, fuzzer, sanitize=True):
    if suite == 'google-test-suite':
        if fuzzer == 'qsym' and not sanitize:
            suite = 'google-test-suite-plain'
        elif fuzzer == 'libfuzzer':
            suite = 'google-test-suite-libfuzzer'
        elif fuzzer == 'honggfuzz':
            suite = 'google-test-suite-honggfuzz'
    return SUITEBIN[suite]

def replace_input_file(s, fuzzer):
    if fuzzer == "honggfuzz":
        s = s.replace('@@', '___FILE___')
    elif fuzzer == "libfuzzer":
        s = s.replace('@@', '')
    return s

def get_default_args(target):
    return f"{BIN2ARGS[target]}"

def get_default_analysis_bin_dir(target):
    return f"/home/coll/analysis_binaries/{target}.analysis_binaries/"

def get_framework_suite_image_name(target):
    suite = BIN2SUITE[target][0].split("-")[0].lower()
    return f"fuzzer-framework-{suite}"

