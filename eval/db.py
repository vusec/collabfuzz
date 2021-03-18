import cbor
import sqlite3
import glob
from datetime import datetime
import sys, os
import matplotlib
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
import scipy.stats as st
import math

#matplotlib.style.use('ggplot')

FILE_OUTPUT = os.environ.get("FILE_OUTPUT", None)


def plot_conf(d, name="ex"):
    smooth_line = d.median(axis=1)

    N = len(d.columns)

    under_line = d.agg(lambda e: sorted(e)[round(N/2 - (1.96 * math.sqrt(N)/2))], axis=1)
    over_line = d.agg(lambda e: sorted(e)[round(1 + N/2 + (1.96 * math.sqrt(N))/2)],axis=1)

    #smooth_line.plot()
    smooth_line.index = smooth_line.index + pd.to_timedelta(0, unit='s')
    plt.plot(smooth_line, linewidth=2)
    #smooth_line.plot()
    plt.fill_between(smooth_line.index, under_line, over_line, color='b', alpha=.1)
    if FILE_OUTPUT:
        plt.savefig(f"{name}-95-median.png")
        plt.close()
    else:
        plt.show()


def gather_fuzzer_cov(c):
    result = []
    seed_times = gather_seed_times(c)
    d = c.execute("SELECT * FROM analysis_states WHERE analysis_id = 2")
    total_cov = set()
    result.append((0, 0))
    
    for i, e in enumerate(d):
        test_case_hash = e[0]
        dump = e[2]
        data = cbor.loads(dump)
        if data:
            entries = set(map(lambda d: (d['source'], d['target']), data))
            total_cov |= entries
            time = seed_times[test_case_hash]
            result.append((time, len(total_cov)))
    return result

def gather_seed_times(c):
    res = {}
    d = c.execute("SELECT * FROM test_cases")
    for e in d:
        name = e[0]
        time = e[3]
        res[name] = time
    return res

def get_databases(folder):
    dbs = glob.glob(os.path.join(folder, "/**/run_info.sqlite"), recursive=True)
    for db in dbs:
        conn = sqlite3.connect(db)
        gather_seed_times(conn)

def plot_experiment(dbs, name="ex"):
    all_ts = []
    for db in dbs:
        conn = sqlite3.connect(db)
        c = conn.cursor()
        data = gather_fuzzer_cov(c)
        if not data:
            continue
        x,y = zip(*data)
        x = pd.to_timedelta(x, unit='s')
        ts = pd.Series(y, index=x, name="coverage")
        ts.index.rename('time', inplace=True)
        # Remove duplicates
        ts = ts.groupby(ts.index).max()
        all_ts.append(ts)

    plot_data = {}
    for i, d in enumerate(all_ts):
        plot_data[i] = d
    d = pd.DataFrame(plot_data)
    #print(d)

    # Fix up NaN
    d.rename_axis('run', axis=1, inplace=True)
    d = d.fillna(method='pad')
    d = d.replace(np.nan, 0)

    d.plot()

    if FILE_OUTPUT:
        plt.savefig(f"{name}-raw.png")
        plt.close()
    else:
        plt.show()

    plot_conf(d, name=name)

    return d

if __name__ == "__main__":
    if len(sys.argv) > 1:
        db_files = sys.argv[1:]
    else:
        db_files = glob.glob(os.path.join('.', "**/run_info.sqlite"), recursive=True)

    sep = db_files.index('-') if '-' in db_files else None
    print(sep)

    if sep is None:
        plot_experiment(db_files)
    else:
        exp1 = db_files[:sep]
        exp2 = db_files[sep+1:]

        #print(db_files)
        print(f"Experiment 1: {exp1}")
        df1 = plot_experiment(exp1, name="ex1")

        print(f"Experiment 2: {exp2}")
        df2 = plot_experiment(exp2, name="ex2")

        max1 = df1.max()
        max2 = df2.max()

        print(max1)
        print(max2)

        mwu = st.mannwhitneyu(max1, max2)

        print(f"Mann-Whitney U test:")
        print(mwu)


