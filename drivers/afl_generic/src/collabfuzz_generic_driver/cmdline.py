from .config import Config, FuzzerType


def get_cmdline_suggestion(config: Config) -> str:
    if (
        config.fuzzer_type == FuzzerType.AFL
        or config.fuzzer_type == FuzzerType.AFLFAST
        or config.fuzzer_type == FuzzerType.FAIRFUZZ
        or config.fuzzer_type == FuzzerType.RADAMSA
    ):
        cmdline_args = (
            f"afl-fuzz -i input_dir "
            f"-o {config.output_dir} "
            f"-M {config.fuzzer_type} "
            f"-- /path/to/target.afl"
        )
    elif config.fuzzer_type == FuzzerType.ANGORA:
        cmdline_args = (
            f"angora_fuzzer -i input_dir "
            f"-o {config.output_dir} -S "
            f"-t /path/to/target.track "
            f"-- /path/to/target.fast"
        )
    elif config.fuzzer_type == FuzzerType.QSYM:
        cmdline_args = (
            f"bin/run_qsym_afl.py "
            f"-a framework "
            f"-o {config.output_dir} "
            f"-n qsym -- /path/to/target"
        )
    elif config.fuzzer_type == FuzzerType.LIBFUZZER:
        cmdline_args = (
            f"TARGET_BIN "
            f"-artifact_prefix={config.output_dir}/artifacts/ "
            f"{config.output_dir}/queue"
        )
    elif config.fuzzer_type == FuzzerType.HONGGFUZZ:
        cmdline_args = (
            f"honggfuzz --input {config.output_dir}/seeds "
            f"--output {config.output_dir}/queue "
            f"--crashdir {config.output_dir}/crashes "
            f"-y {config.output_dir}/sync -- "
            f"/path/to/target.honggfuzz"
        )
    else:
        raise Exception(f"Invalid fuzzer type: {config.fuzzer_type}")

    return cmdline_args
