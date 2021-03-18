import sys
import yaml
import os
import threading
import subprocess
import signal
import asyncio
import random
import time
import queue
import multiprocessing
import re
from datetime import timedelta

from argparse import ArgumentParser


NPAR = int(multiprocessing.cpu_count()/5)
DEFAULT_SCHEDULER = "enfuzz"
SCRIPT_BASE_PATH = None

def get_valid_filename(s):
    s = str(s).strip().replace(' ', '_')
    return re.sub(r'(?u)[^-\w.]', '', s)

def docker_env_str(env):
    return " ".join(map(lambda v: "-e {}={}".format(v[0],v[1]), env.items()))

def set_envs(env):
    for k,v in env.items():
        os.environ[k] = v


class Target:
    def __init__(self, name, binary, input_folder):
        self.name = name
        self.binary = binary
        self.input = os.path.abspath(os.path.join(SCRIPT_BASE_PATH, input_folder)) if input_folder else None

class Fuzzer:
    def __init__(self, name, f_type, parallel):
        self.name = name
        self.type = f_type
        self.parallel = parallel

class Experiment:
    def __init__(self, name, timeout, repeat, output_folder, use_collab, scheduler):
        self.name = name
        self.timeout = timeout
        self.repeat = repeat
        self.output = output_folder
        self.fuzzers = []
        self.targets = []
        self.use_collab = use_collab
        self.scheduler = scheduler

    # Applies the supplied function `f(fuzzer,target)` to each (fuzzer,target) pair
    # and returns the value of each call in a list
    def map(self, f):
        r = []
        for fuzzer in self.fuzzers:
            for target in self.targets:
                r.append(f(fuzzer,target))
        return r

    def map_dict(self, f):
        r = {}
        for target in self.targets:
            r[target.name] = {}
            for fuzzer in self.fuzzers:
                r[target.name][fuzzer.name] = f(fuzzer,target)
        return r

class Task():
    def __init__(self, name, cmds, timeout):
        self.cmds = cmds
        self.timeout = timeout
        self.procs = []
        self.name = name

    def run(self):
        for cmd in self.cmds:
            self.__exec(cmd)

    def run_sleep(self):
        self.run()
        time.sleep(self.timeout)

    def run_blocking(self):
        self.run()
        try:
            for p in self.procs:
                p.communicate(timeout=self.timeout + 60)
        except subprocess.TimeoutExpired:
            print("Task {name} expired. Killing...".format(name=self.name), file=sys.stderr)
            self.kill()


    def kill(self):
        for p in self.procs:
            p.kill()
            os.killpg(os.getpgid(p.pid), signal.SIGINT)
            os.killpg(os.getpgid(p.pid), signal.SIGKILL)
   

    def __exec(self, cmd):
        print("Execute: '{}'".format(cmd))
        dirname = get_valid_filename(self.name)
        os.mkdir(dirname)
        workdir = os.path.join(os.getcwd(), dirname)
        log_out = open(os.path.join(workdir, "task.out"), "a+")
        err_out = open(os.path.join(workdir, "task.err"), "a+")
        p = subprocess.Popen(cmd, shell=True, stdout=log_out, stderr=err_out, preexec_fn=os.setsid, cwd=workdir)
        self.procs.append(p)




def parse_fuzzer(d):
    return Fuzzer(d["name"], d["type"], d.get("parallel", 1))

def parse_target(d):
    return Target(d["name"], d["binary"], d.get("input", None))

def parse_experiment(d):
    return Experiment(d["name"], d.get("timeout", 0), d.get("repeat", 1), d.get("output", None), d.get("use_collab", True), d.get("scheduler", DEFAULT_SCHEDULER))

def parse_input(d, experiment_id=0):
    fuzzers = []
    for f in d["fuzzers"]:
        fuzzer = parse_fuzzer(f)
        fuzzers.append(fuzzer)

    targets = []
    for t in d["targets"]:
        target = parse_target(t)
        targets.append(target)

    experiment = parse_experiment(d["experiment"])
    experiment.fuzzers = fuzzers
    experiment.targets = targets

    tasks = []
    for target in experiment.targets:
        input_conf = "--input-dir={}".format(target.input) if target.input else ""
        for n in range(experiment.repeat):
            #l = ["collab_fuzz_runner -v -f {fuzzers} --scheduler={scheduler} -s {target_name} -t {timeout} --disable-checks {input_conf}".format(fuzzers=" ".join(map(lambda f: " ".join([f.name] * f.parallel), experiment.fuzzers)), target_name=target.binary, timeout=experiment.timeout, scheduler=experiment.scheduler, input_conf=input_conf)]

            #1) Generate docker-compose

            #2) Run

            #3) Teardown

            #4) Collect data

            t_name = "{ex_name}-{ex_id}-{target}-{n}".format(ex_id=experiment_id, ex_name=experiment.name, target=target.name, n=n)
            task = Task(t_name, l, experiment.timeout)
            tasks.append(task)
    return tasks



class Worker(threading.Thread):
    def __init__(self, q):
        threading.Thread.__init__(self)
        self.kill_received = False
        self.cur_w = None
        self.q = q

    def run(self):
        while not self.kill_received:
            w = self.q.get()
            if w is None:
                break
            self.cur_w = w

            start = time.time()
            print("Starting task '{name}'".format(name=w.name))
            ## Create experiment output dir
            w.run_blocking()

            end = time.time()
            elapsed = end - start
            print("Finished task {} after {}s".format(w.name, str(timedelta(seconds=elapsed))))
            self.q.task_done()

    def kill(self):
        self.kill_received = True
        self.cur_w.kill()




def run_queue(q, npar=1):
    threads = []
    for i in range(npar):
        t = Worker(q)
        t.start()
        threads.append(t)
    try:
        q.join()
        for i in range(npar):
            q.put(None)

    except KeyboardInterrupt:
        print("Killing work!")
        for t in threads:
            t.kill()

    for t in threads:
        t.join()
    print("Done!")




def main():
    global SCRIPT_BASE_PATH

    parser = ArgumentParser()
    parser.add_argument('files',
                        type=str,
                        nargs="+",
                        help='Experiment YAML files to run')

    args = parser.parse_args()

    SCRIPT_BASE_PATH = os.path.dirname(args.files[0])
    q = queue.Queue()
    for i, fname in enumerate(args.files):
        with open(fname) as fh:
            d = yaml.load(fh.read())
            if not d:
                print("Error loading experiment file {fname}".format(fname=fname))
            tasks = parse_input(d, experiment_id=i)
            for t in tasks:
                q.put(t)

    run_queue(q, npar=NPAR)


if __name__ == "__main__":
    main()
