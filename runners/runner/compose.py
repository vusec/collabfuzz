import yaml
import collections
import grp
import tempfile
import os
import subprocess

from runner.settings import BIN2ARGS, SUITEBIN, BIN2SUITE, get_default_args, get_default_analysis_bin_dir, get_suitebin, get_framework_suite_image_name

import logging
from argparse import ArgumentParser

VERBOSE = True

LISTENER_PORT = 3000
CONTROL_PORT = 3001
SCHEDULER_PORT = 3002

FUZZER_COUNT = collections.Counter()


Config = collections.namedtuple('Config', ['args', 'analysis_bin_dir', 'target', 'enable_docker_control', 'scheduler', 'enable_afl_affinity'], )
logger = logging.getLogger(__name__)



def setup_framework(config):
    #analysis_bin_dir = "/home/coll/analysis_binaries/objdump.analysis_binaries/"
    analysis_bin_dir = config.analysis_bin_dir
    args = config.args
    r = {}
    r["image"] = get_framework_suite_image_name(config.target)
    r["volumes"] = [f'data-vol:/data', './input:/in']
    r["environment"] = {
            "URI_LISTENER": f"tcp://*:{LISTENER_PORT}",
            "URI_CONTROL": f"tcp://*:{CONTROL_PORT}",
            "URI_SCHEDULER": f"tcp://*:{SCHEDULER_PORT}",
            "OUTPUT_DIR": "/data/collab/out",
            "INPUT_DIR": "/in",
            "ANALYSIS_BIN_DIR": analysis_bin_dir,
            "ARG": args,
            "SCHEDULER": config.scheduler,
            }
    if VERBOSE:
        r["environment"]["RUST_LOG"] = "debug"


    return "framework", r

def setup_driver(config, fuzzer_id, fuzzer_type, target):
    r = {}
    args = config.args
    binary = BIN2SUITE[target][1]
    suite = BIN2SUITE[target][0]
    ext = ""
    if len(BIN2SUITE[target]) > 2:
        ext = BIN2SUITE[target][2]

    bpath = get_suitebin(suite, fuzzer_type).format(bin=binary, ext=ext)

    r["image"] = "fuzzer-generic-driver"
    r["volumes"] = [f'data-vol:/data']
    r["links"] = ["framework"]
    r["depends_on"] = ["framework"]

    if config.enable_docker_control:
        r["volumes"].append("/var/run/docker.sock:/var/run/docker.sock")
        r["pid"] = "host"

    r["environment"] = {
            "URI_LISTENER": f"tcp://framework:{LISTENER_PORT}",
            "URI_CONTROL": f"tcp://framework:{CONTROL_PORT}",
            "URI_SCHEDULER": f"tcp://framework:{SCHEDULER_PORT}",
            "OUTPUT_DIR": f"/data/{fuzzer_id}/",
            "FUZZER_NAME": fuzzer_type,
            #"CONTAINER_ID": f"compose_{fuzzer_id}_1" if config.enable_docker_control else "none",
            "ARG": f"{bpath} {args}",
            }
    return f'driver-{fuzzer_id}', r

def setup_fuzzer(config, fuzzer, target):
    count = FUZZER_COUNT[fuzzer]
    name = f'{fuzzer}-{count}'
    r = {}

    r["image"] = f'fuzzer-{fuzzer}-{target}'
    r["volumes"] = [f'data-vol:/data', './input:/in']
    r["depends_on"] = ["framework", f"driver-{name}"]
    r["environment"] = {
            "OUTPUT_DIR": f'/data/{name}',
            }
    if config.enable_afl_affinity:
        r["pid"] = "host"
    else:
        r["environment"]["AFL_NO_AFFINITY"] = "1"

    if fuzzer == "qsym":
        r["stop_signal"] = "SIGKILL"

    FUZZER_COUNT[fuzzer] += 1
    return name, r


def generate_compose(compose):
    return yaml.dump(compose)

def setup_run(config, target, fuzzers):

    services = {}
    services["framework"] = setup_framework(config)[1]
    for fuzzer in fuzzers:
        fuzzer_name, fuzzer_data = setup_fuzzer(config, fuzzer, target)
        driver_name, driver_data = setup_driver(config, fuzzer_name, fuzzer, target)
        services[fuzzer_name] = fuzzer_data
        services[driver_name] = driver_data

    compose = {}
    compose["version"] = '2'
    compose["services"] = services 
    compose["volumes"] = {"data-vol": {}}

    res = generate_compose(compose)

    # Create default seed in input folder
    os.mkdir("input")
    with open(os.path.join("input", "seed"), "w") as f:
        f.write("AAAAAAAA")
    return res

def do_run(config, target, fuzzers, timeout=0):
    # Create tmp dir
    with tempfile.TemporaryDirectory() as tmp:
        orig_cwd = os.getcwd()
        os.chdir(tmp)
        # Generate compose
        with open("docker-compose.yaml", "w+") as f:
            f.write(setup_run(config, target, fuzzers))

        # Load input seeds

        # Docker compose up
        subprocess.call("docker-compose up -d")

        # Wait
        if timeout > 0:
            time.sleep(timeout)

        # Retrieve results
        # docker-compose ps -q --filter NAME=fuzzer-framework
        # docker cp XXX:/data ./out

        # Teardown
        subprocess.call("docker-compose down")
        os.chdir(orig_cwd)


    pass
