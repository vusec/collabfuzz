from runner.compose import setup_run, Config
from runner.settings import get_default_analysis_bin_dir, get_default_args
from argparse import ArgumentParser
import runner.build as build
import logging
import docker
import os

logging.basicConfig(level=os.environ.get("LOGLEVEL", "INFO").upper())
logger = logging.getLogger(__name__)

def main():

    parser = ArgumentParser()
    parser.add_argument('target',
                        type=str,
                        help='Target to run')

    parser.add_argument('-f',
                        '--fuzzers',
                        nargs='+',
                        type=str,
                        required=True,
                        #choices=list(FuzzerType),
                        help='Fuzzers to run')


    parser.add_argument('-s', 
                        '--scheduler', 
                        default="broadcast", 
                        required=False,
                        help='Scheduler to use for the framework')

    parser.add_argument('--args',
                        type=str,
                        required=False,
                        help='Program arguments (e.g., "objdump -x @@")')

    parser.add_argument('--analysis-bin-dir',
                        type=str,
                        required=False,
                        help='Directory containing analysis binaries in framework container')

    parser.add_argument('-v',
                        '--verbose',
                        action='store_true',
                        help='Set log level to DEBUG')

    parser.add_argument('--stdout',
                        action='store_true',
                        help='Set log level to DEBUG')

    parser.add_argument('--disable-docker-control',
                        action='store_false',
                        help='Enable driver docker control (pause, resume fuzzer container). Requires privileged mode.')

    parser.add_argument('--enable-afl-affinity',
                        action='store_true',
                        help='Allow AFL to do core pinning (requires host pid map in docker)')

    parser.add_argument('--build-test-all',
                        action='store_true',
                        help='Build and test all available target containers')

    parser.add_argument('--build-test',
                        action='store_true',
                        help='Build and test specified target containers')

    args = parser.parse_args()


    if args.verbose:
        logger.setLevel(logging.DEBUG)
        logging.getLogger().setLevel(logging.DEBUG)

    if args.build_test_all:
        client = docker.from_env()
        build.init_req(client)
        build.build_and_test_all(client)

    if args.build_test:
        client = docker.from_env()
        build.init_req(client)
        for fuzzer in args.fuzzers:
            build.build_and_test(client, args.target, fuzzer)

    config = Config(analysis_bin_dir=args.analysis_bin_dir if args.analysis_bin_dir is not None else get_default_analysis_bin_dir(args.target) ,
                    args = args.args if args.args is not None else get_default_args(args.target),
                    target=args.target,
                    enable_docker_control=args.disable_docker_control,
                    scheduler=args.scheduler, enable_afl_affinity=args.enable_afl_affinity)

    result = setup_run(config, args.target, args.fuzzers)
    if args.stdout:
        print(result)
    else:
        if os.path.exists("docker-compose.yaml"):
            logger.error("Output 'docker-compose.yaml' file already exists!")
        else:
            with open("docker-compose.yaml", "w+") as f:
                f.write(result)


if __name__ == '__main__':
    main()
