import io
import docker
import tempfile
import os

MAX_RETRIES = 5

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
    "google-test-suite-plain": "/targets-plain/google/{bin}-coverage{ext}",
    "LAVA-M": "/targets/lava/{bin}/bin/{bin}"
    }

ONLY_PLAIN = ["qsym"]

FUZZERS = {
       "afl": "afl-fuzz {afl_opts} -i {input_dir} -o {output_dir} -M afl -- {target_cmd}",
       "aflfast": "afl-fuzz {afl_opts} -p fast -i {input_dir} -o {output_dir} -M afl -- {target_cmd}",
       "fairfuzz": "afl-fuzz {afl_opts} -i {input_dir} -o {output_dir} -M afl -- {target_cmd}",
       "qsym": "/qsym/bin/run_qsym_afl.py -a framework -o {output_dir} -n qsym -- {target_cmd}",
       "radamsa": "afl-fuzz {afl_opts} -RR -i {input_dir} -o {output_dir} -M afl -- {target_cmd}"
       #"honggfuzz": "afl-fuzz -RR -i {input_dir} -o {output_dir} -M afl -- {target_cmd}",
}

FUZZER2TYPE = {
        "qsym": "qsym"
        }


def gen_docker_file(base_container, cmd, bpath='.', target_cmd=""):
    return f"""
    FROM {base_container}
    USER root
    RUN echo "#!/usr/bin/env bash" > /entry.sh
    RUN echo "mkdir -p \$OUTPUT_DIR || true" >> /entry.sh
    RUN echo "set -Eeuxo pipefail" >> /entry.sh
    RUN echo cd {bpath} >> /entry.sh
    RUN echo "{cmd}" >> /entry.sh
    RUN echo "#!/usr/bin/env bash" > /test.sh
    RUN echo "set -Eeuxo pipefail" >> /test.sh
    RUN echo cd {bpath} >> /test.sh
    RUN echo mkdir -p \$OUTPUT_DIR/framework/queue >> /test.sh
    RUN echo echo "AAAAAAAA > \$OUTPUT_DIR/framework/queue/seed" >> /test.sh
    RUN echo "command_line: /fuzzers/afl/afl-fuzz -- {target_cmd}" >> /tmp/fuzzer_stats
    RUN echo cp /tmp/fuzzer_stats \$OUTPUT_DIR/framework >> /test.sh
    RUN echo \"timeout --preserve-status \$TIMEOUT {cmd} || true\" >> /test.sh
    RUN chmod +x /entry.sh
    RUN chmod +x /test.sh
    ENV INPUT_DIR=/in OUTPUT_DIR=out TIMEOUT=10s
    RUN mkdir /in && echo AAAAAAAA > /in/seed
    RUN mkdir /data && chown -R coll:coll /data
    USER coll
    CMD ["/entry.sh"]
    """

def build_req(client, fuzzer):
    name = f'fuzzer-{fuzzer}'
    try:
        client.images.get(name)
    except docker.errors.ImageNotFound as e:
        print(f'Base image {name} not found. Building...')
        #TODO: pull in image/ build
        #os.system(f'make -j`nproc` {name}')

def test_container(client, image, fuzzer_type="afl"):
    #print(f'Testing {image}')
    retries = 0
    while retries < MAX_RETRIES:
        print(f'Testing {image} [{retries}]')
        try:
            with tempfile.TemporaryDirectory() as tmp:
                volumes = {tmp: {'bind': '/out', 'mode': 'rw'}}
                client.containers.run(image, auto_remove = True, command = "/test.sh", environment = {"TIMEOUT": str(10*2^(retries))+"s", "OUTPUT_DIR": "/out"}, volumes=volumes, cap_add=['SYS_PTRACE'])
                if fuzzer_type == "qsym":
                    queue = os.listdir(f'{tmp}/qsym/queue')
                elif fuzzer_type == "afl":
                    queue = os.listdir(f'{tmp}/afl/queue')
                else:
                    raise "Unknown fuzzer type"
                if len(queue) > 1:
                    retries = MAX_RETRIES

                if retries == MAX_RETRIES - 1:
                    print(f"WARN: could not generate queue files for {image}")
                    #assert(len(queue) > 1)

        except docker.errors.ContainerError as e:
            print(f'Error for {image}: {e}')
            raise e
        retries += 1

def build_and_test(client, target, fuzzer):
    data = BIN2SUITE[target]
    suite = BIN2SUITE[target][0]
    binary = data[1]
    bpath = os.path.dirname(SUITEBIN[suite].format(bin=binary, ext=""))
    target_cmd = "{bin} {args}".format(bin=SUITEBIN[suite].format(bin=binary,  ext=""), args=BIN2ARGS[target])
    fuzzer_target_cmd = target_cmd if fuzzer not in ONLY_PLAIN else "{bin} {args}".format(bin=SUITEBIN[suite+"-plain"].format(bin=binary,  ext=""), args=BIN2ARGS[target])
    if len(data) > 2:
        # For target applications with multiple bins (e.g., openssl-1.1.0c)
        ext = "-{}".format(data[2])
        target_cmd = "{bin} {args}".format(bin=SUITEBIN[suite].format(bin=binary, ext=ext), args=BIN2ARGS[target])
    base_container = f'fuzzer-{fuzzer}'
    new_container = f'fuzzer-{fuzzer}-{target}'
    afl_opts = SUITEOPTS[suite]
    fuzzer_cmd = FUZZERS[fuzzer]
    cmd = fuzzer_cmd.format(target_cmd=fuzzer_target_cmd, input_dir="\$INPUT_DIR", output_dir="\$OUTPUT_DIR", afl_opts=afl_opts)
    fileobj = io.BytesIO(gen_docker_file(base_container, cmd, bpath=bpath, target_cmd=target_cmd).encode())

    print(f'Building {new_container}')
    client.images.build(fileobj=fileobj, tag=new_container, rm=True)
    fuzzer_type = FUZZER2TYPE.get(fuzzer, "afl")
    test_container(client, new_container, fuzzer_type=fuzzer_type)

def build_and_test_suite(client, suite):
    for fuzzer, fuzzer_cmd in FUZZERS.items():
        for target, data in BIN2SUITE.items():
            if suite != data[0]:
                continue
            build_and_test(client, target, fuzzer)

def build_and_test_all(client):
    suites = set(map(lambda e: e[0], BIN2SUITE.values()))
    for test_suite in suites:
        build_and_test_suite(client, test_suite)


def init_req(client):
    for fuzzer in FUZZERS:
        build_req(client, fuzzer)
    os.system("echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope")


def start():
    client = docker.from_env()
    init_req(client)
    #build_and_test(client, "objdump", "afl")
    build_and_test(client, "boringssl", "qsym")
    #build_and_test_all(client)

if __name__ == "__main__":
    start()
