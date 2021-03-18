#!/usr/bin/env python3
from collections import namedtuple
from argparse import ArgumentParser
from pathlib import Path
import json
import os
from multiprocessing import Pool
import pandas as pd
import numpy as np
import sqlite3
import cbor2
import csv
import subprocess

Config = namedtuple("Config", "data_path,output_dir,jobs,load,targets")
Target = namedtuple("Target", "name,fuzzers")
Fuzzer = namedtuple("Fuzzer", "name,runs")


def make_connection(config, target, run):
    return sqlite3.connect(config.data_path / f"{target}{run}" / "_data" /
                           "collab" / "out" / "run_info.sqlite")


def get_observed_branches(arr, sz):
    nparr = np.array([False] * sz)
    for i in range(sz):
        idx = i // 32
        b = arr[idx]
        check = 1 << (i % 32)
        nparr[i] = (b & check) == check
    return nparr


def query_instruction_counts(config, target, run):
    with make_connection(config, target, run) as conn:
        query = ("SELECT analysis_dump "
                 "FROM analysis_states "
                 "JOIN analysis_types ON analysis_id = id "
                 "JOIN discoveries USING(discovery_id) "
                 "WHERE description = \"instruction_count\" "
                 "AND is_new = 1 "
                 "ORDER BY discovery_id ASC;")
        cursor = conn.execute(query)
        inst_counts_raw = [cbor2.loads(row[0]) for row in cursor.fetchall()]

    inst_counts = {}
    for row in inst_counts_raw:
        for elm in row:
            inst_counts[int(elm["id"])] = elm["count"]
    return inst_counts


def query_time_to_solve(config, target, fuzzer, run):
    solved_condition_min_time = {}
    with make_connection(config, target, run) as conn:
        query = ("SELECT d.discovery_id, d.discovery_time, "
                 "s1.analysis_dump, s2.analysis_dump "
                 "FROM discoveries d "
                 "JOIN fuzzers fs ON fs.fuzzer_id = d.discovery_fuzzer "
                 "JOIN fuzzer_types ft ON ft.id = fs.fuzzer_type_id "
                 "JOIN ("
                 "  SELECT a.discovery_id, a.analysis_dump "
                 "  FROM analysis_states a "
                 "  JOIN analysis_types ON a.analysis_id = id "
                 "  WHERE description = \"fuzzer_observed_conditions\""
                 ") s1 USING(discovery_id) "
                 "JOIN ("
                 "  SELECT a.discovery_id, a.analysis_dump "
                 "  FROM analysis_states a "
                 "  JOIN analysis_types ON a.analysis_id = id "
                 "  WHERE description = \"tainted_conditions\""
                 ") s2 USING(discovery_id) "
                 f"WHERE ft.description = \"{fuzzer}\" "
                 "ORDER BY d.discovery_id ASC;")
        cursor = conn.execute(query)

        built_conditions = {}
        for row in cursor.fetchall():
            discovery_time = row[1]
            observed_conditions_raw = row[2]
            tainted_conditions_raw = row[3]

            tainted_conditions = cbor2.loads(tainted_conditions_raw)

            for o in cbor2.loads(observed_conditions_raw):
                cond_id = int(o["id"])
                if cond_id in solved_condition_min_time or cond_id not in tainted_conditions:
                    continue

                os = o["observed_states"]
                observed_states = get_observed_branches(os[1], os[0])
                if cond_id in built_conditions:
                    built_conditions[cond_id] |= observed_states
                else:
                    built_conditions[cond_id] = observed_states

                if built_conditions[cond_id].all():
                    solved_condition_min_time[cond_id] = discovery_time
    return solved_condition_min_time


def gather_data_run(config, target, fuzzer, run):
    inst_counts = query_instruction_counts(config, target, run)
    solved_conditions = query_time_to_solve(config, target, fuzzer, run)

    result = []
    for cond_id, min_time in solved_conditions.items():
        if cond_id not in inst_counts:
            print(f"Condition {cond_id} was not in icount!")
            continue

        cost = inst_counts[cond_id]
        result.append({
            "target": target,
            "run_number": run,
            "fuzzer": fuzzer,
            "condition_id": cond_id,
            "inst_count": cost,
            "time": min_time
        })

    df = pd.DataFrame.from_dict(result)
    print("{:<16} / {:<16} / {:02} / {:>6}".format(target, fuzzer, run,
                                                   df.shape[0]))
    return df


def get_static_features(config):
    dfs = []
    for target in config.targets:
        csv_path = config.data_path / f"{target.name}-static.csv"
        with open(csv_path, "r") as csvfile:
            data = {(target.name, int(row["Condition"])): {
                "cyclomatic": int(row["Cyclomatic"]),
                "oviedo": int(row["Oviedo"]),
                "chain_size": int(row["ChainSize"]),
                "cmp_size": int(row["CompareSize"]),
                "compares_const": bool(int(row["ComparesConstant"])),
                "compares_point": bool(int(row["ComparesPointer"])),
                "is_equality": bool(int(row["IsEquality"])),
                "is_constant": bool(int(row["IsConstant"])),
                "cases": int(row["Cases"])
            }
                    for row in csv.DictReader(csvfile)}
        index = pd.MultiIndex.from_tuples(data.keys())
        dfs.append(pd.DataFrame.from_records(list(data.values()), index=index))
    return pd.concat(dfs, sort=False)


def gather_data(config):
    args = [(config, target.name, fuzzer.name, run) \
            for target in config.targets
            for fuzzer in target.fuzzers
            for run in fuzzer.runs]

    with Pool(config.jobs) as pool:
        dfs = pool.starmap_async(gather_data_run, args).get()

    df = pd.concat(dfs, axis=0, ignore_index=True, sort=False)
    df = df.join(get_static_features(config), on=["target", "condition_id"])
    assert not df.isna().any().any()
    return df


def parse_config():
    parser = ArgumentParser(
        description="Build training set for regression models")

    parser.add_argument("-p",
                        "--data-path",
                        type=Path,
                        required=True,
                        help="Path to input data folder")

    parser.add_argument("-c",
                        "--config",
                        type=Path,
                        required=True,
                        help="Configuration JSON")

    parser.add_argument("-o",
                        "--output-dir",
                        type=Path,
                        required=True,
                        help="Output directory")

    parser.add_argument("-j",
                        "--jobs",
                        type=int,
                        default=os.cpu_count() // 2,
                        help="How many parallel jobs")

    parser.add_argument("-L",
                        "--load",
                        type=bool,
                        const=True,
                        default=False,
                        help="Load training data CSV instead of extracting it")

    args = parser.parse_args()
    with open(args.config) as f:
        targets_config = json.load(f)

    targets = []
    for target, fuzzers in targets_config.items():
        target_fuzzers = []
        for fuzzer, runs in fuzzers.items():
            target_fuzzers.append(Fuzzer(fuzzer, runs))
        targets.append(Target(target, target_fuzzers))

    return Config(args.data_path, args.output_dir, args.jobs, args.load,
                  targets)


def main():
    config = parse_config()

    subprocess.run(["which", "svm-scale", "svm-train"],
                   stdout=subprocess.DEVNULL).check_returncode()

    config.output_dir.mkdir(parents=True, exist_ok=True)
    fuzzers = [f.name for f in config.targets[0].fuzzers]

    if config.load:
        df = pd.read_csv(config.output_dir / "train.csv")
    else:
        df = gather_data(config)
        df.to_csv(config.output_dir / "train.csv")

    print(f"Loaded {df.shape[0]} rows")

    for fuzzer in fuzzers:
        print(f"Starting training for {fuzzer}")

        train_file = config.output_dir / f"train-{fuzzer}.dat"
        range_file = config.output_dir / f"{fuzzer}.range"
        scaled_file = config.output_dir / f"scaled-{fuzzer}.dat"
        model_file = config.output_dir / f"{fuzzer}.model"

        with open(train_file, "w") as out:
            for row in df[df.fuzzer == fuzzer].itertuples():
                out.write(f"{row.time} "
                          f"1:{row.oviedo} 2:{row.chain_size} "
                          f"3:{row.cmp_size} 4:{row.inst_count}\n")
        print(f"Training data stored to {train_file}")

        scale_proc = subprocess.run(
            ["svm-scale", "-s", range_file, train_file],
            stdout=subprocess.PIPE)
        scale_proc.check_returncode()
        print(f"Range info stored to {range_file}")

        with open(scaled_file, "wb") as f:
            f.write(scale_proc.stdout)
        print(f"Scaled training data stored to {scaled_file}")

        subprocess.run([
            "svm-train", "-s", "3", "-t", "2", "-p", "1", "-h", "0",
            scaled_file, model_file
        ]).check_returncode()
        print(f"Trained model stored to {model_file}")
        print("=" * 60)


if __name__ == "__main__":
    main()
