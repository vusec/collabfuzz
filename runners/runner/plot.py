import cbor
import sqlite3
import glob
import datetime as dt
from datetime import datetime
import sys, os
import matplotlib
import matplotlib.dates as mdates
import matplotlib.ticker as ticker
from matplotlib.dates import DateFormatter
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
import scipy.stats as st
import math
from argparse import ArgumentParser
import logging

FILE_OUTPUT = True

def __check_files_exist(files):
    for f in files:
        if not os.path.isfile(f):
            raise FileNotFoundError(f"file '{f}' does not exists")

def format_timedelta(x, pos=None):
    x = int(x)
    hours, remainder = divmod(x, 3600)
    minutes, seconds =  divmod(remainder, 60)
    return f"{hours:02}:{minutes:02}"

ax = None

def plot_with_time(df):
    global ax
    df = df.copy()
    #fig, ax = plt.subplots()
    a,b,c = ax.get_geometry()
    print(a,b,c)
    ax = plt.subplot(a,b,c)
    df.index = df.index.astype("timedelta64[s]")
    ax.xaxis.set_major_formatter(ticker.FuncFormatter(format_timedelta))
    ax.xaxis.set_major_locator(plt.MultipleLocator(60*60*2))

    df.plot(ax=ax)
    plt.gcf().autofmt_xdate()
    return df


def next_plot():
    global ax
    if not (ax.is_last_col() and ax.is_last_row()):
        a,b,c = ax.get_geometry()
        ax = plt.subplot(a,b,c + 1)


def plot_conf(d, name="ex"):
    global ax
    period = dt.timedelta(hours=24)
    d = d[d.index < period]
    smooth_line = d.median(axis=1)

    N = len(d.columns)

    under_line = d.agg(lambda e: sorted(e)[round(N/2 - (1.96 * math.sqrt(N)/2)) - 1], axis=1)
    over_line = d.agg(lambda e: sorted(e)[round(1 + N/2 + (1.96 * math.sqrt(N))/2) - 1],axis=1)

    smooth_line = plot_with_time(smooth_line)
    plt.fill_between(smooth_line.index, under_line, over_line, color='b', alpha=.1)
    next_plot()
    if FILE_OUTPUT:
        plt.savefig(f"{name}-95-median.png")
        plt.close()


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
    global ax
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


    plot_with_time(d)
    next_plot()

    if FILE_OUTPUT:
        plt.savefig(f"{name}-raw.png")
        plt.close()

    plot_conf(d, name=name)

    return d

def main_plot_nofiles():
    db_files = glob.glob(os.path.join('.', "**/run_info.sqlite"), recursive=True)
    main_plot(db_files)

def main_plot(db_files, name='ex'):
    print(db_files)
    if len(db_files) == 0:
        raise Exception("No run_info.sqlite files found.")
    __check_files_exist(db_files)
    return plot_experiment(db_files, name=name)

def main_plot_compare(exp1, exp2):
    print(f"Experiment 1: {exp1}")
    df1 = main_plot(exp1, name="ex1")

    print(f"Experiment 2: {exp2}")
    df2 = main_plot(exp2, name="ex2")

    max1 = df1.max()
    max2 = df2.max()

    print(max1)
    print(max2)


    print("Area under curve:")
    print(np.trapz(df1.median(axis=1)))
    print(np.trapz(df2.median(axis=1)))

    mwu = st.mannwhitneyu(max1, max2)

    print(f"Mann-Whitney U test:")
    print(mwu)

    print(df1)
    print(df2)




def main():

    parser = ArgumentParser()
    parser.add_argument('-e',
                        '--experiment',
                        metavar='RUN_INFO',
                        nargs='+',
                        action='append',
                        default = [],
                        help='Input files (run_info.sqlite) for experiment')

    parser.add_argument('--show',
                        action='store_true',
                        help='Show plots interactively')

    args = parser.parse_args()

    if args.show:
        print("SHOW")
        global FILE_OUTPUT
        FILE_OUTPUT = False

    global ax
    if len(args.experiment) == 0:
        ax = plt.subplot(1,2,1)
        main_plot_nofiles()
    elif len(args.experiment) == 1:
        ax = plt.subplot(1,2,1)
        main_plot(args.experiment[0])
    elif len(args.experiment) == 2:
        ax = plt.subplot(2,2,1)
        main_plot_compare(*args.experiment)
    else:
        print("Too many experiments provided. Give at most 2.")
        sys.exit(1)

    if args.show:
        plt.tight_layout()
        plt.show()



if __name__ == "__main__":
    main()
