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
from datetime import date
from datetime import timedelta


NPAR = int(os.environ.get("NPAR", int(multiprocessing.cpu_count()/5)))
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
    def __init__(self, name, input_folder):
        self.name = name
        self.input = os.path.abspath(os.path.join(SCRIPT_BASE_PATH, input_folder)) if input_folder else None

class Fuzzer:
    def __init__(self, name, f_type, parallel):
        self.name = name
        self.type = f_type
        self.parallel = parallel

class Experiment:
    def __init__(self, name, timeout, repeat, output_folder, use_collab, scheduler, enable_afl_affinity):
        self.name = name
        self.timeout = timeout
        self.repeat = repeat
        self.output = output_folder
        self.fuzzers = []
        self.targets = []
        self.use_collab = use_collab
        self.scheduler = scheduler
        self.enable_afl_affinity = enable_afl_affinity

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
        self.log_out = None
        self.err_out = None

    def run(self):
        # Setup Task directory
        dirname = get_valid_filename(self.name)
        os.mkdir(dirname)
        workdir = os.path.join(os.getcwd(), dirname)
        self.log_out = open(os.path.join(workdir, "task.out"), "a+")
        self.err_out = open(os.path.join(workdir, "task.err"), "a+")
        for cmd in self.cmds:
            p = self.__exec(cmd, workdir)
            p.communicate()


    def kill(self):
        for p in self.procs:
            p.kill()
            os.killpg(os.getpgid(p.pid), signal.SIGINT)
            os.killpg(os.getpgid(p.pid), signal.SIGKILL)
   

    def __exec(self, cmd, workdir):
        print("Execute: '{}'".format(cmd))
        p = subprocess.Popen(cmd, shell=True, stdout=self.log_out, stderr=self.err_out, preexec_fn=os.setsid, cwd=workdir)
        self.procs.append(p)
        return p




def parse_fuzzer(d):
    return Fuzzer(d["name"], d["type"], d.get("parallel", 1))

def parse_target(d):
    return Target(d["name"], d.get("input", None))

def parse_experiment(d):
    return Experiment(d["name"], d.get("timeout", 0), d.get("repeat", 1), d.get("output", None), d.get("use_collab", True), d.get("scheduler", DEFAULT_SCHEDULER), d.get("enable_afl_affinity", False))

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
        for n in range(experiment.repeat):
            fuzzers = " ".join(map(lambda f: " ".join([f.name] * f.parallel), experiment.fuzzers))
            afl_affinity_flag = "--enable-afl-affinity" if experiment.enable_afl_affinity else ""
            cmds = [f"collab_fuzz_compose -v {afl_affinity_flag} -f {fuzzers} --scheduler={experiment.scheduler} -- {target.name}"]
            if target.input:
                cmds.append("rm -rf input")
                cmds.append(f"cp -r {target.input} input")
            cmds.append("docker-compose up -d")
            cmds.append(f"sleep {experiment.timeout}")
            cmds.append("docker-compose stop")
            today = date.today()
            d1 = today.strftime("%d-%m-%Y")
            t_name = "{date}-{ex_name}-{ex_id}-{target}-{n}".format(date=d1, ex_id=experiment_id, ex_name=experiment.name, target=target.name, n=n)
            task = Task(t_name, cmds, experiment.timeout)
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
            w.run()

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
    if len(sys.argv) < 2:
        print("USAGE: {} [config]".format(sys.argv[0]))
        sys.exit(1)

    SCRIPT_BASE_PATH = os.path.dirname(sys.argv[1])
    q = queue.Queue()
    for i, fname in enumerate(sys.argv[1:]):
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
