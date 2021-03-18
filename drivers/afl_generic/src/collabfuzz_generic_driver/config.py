from .fuzzerctrlmsg_pb2 import FuzzerType as PB2FuzzerType

from pathlib import Path
from enum import Enum
from typing import NamedTuple, Optional, List


class FuzzerType(Enum):
    AFL = "afl"
    ANGORA = "angora"
    QSYM = "qsym"
    LIBFUZZER = "libfuzzer"
    HONGGFUZZ = "honggfuzz"
    AFLFAST = "aflfast"
    FAIRFUZZ = "fairfuzz"
    RADAMSA = "radamsa"

    def __str__(self) -> str:
        return self.value

    def to_pb2_type(self) -> PB2FuzzerType:
        if self == FuzzerType.AFL:
            return PB2FuzzerType.FUZZER_TYPE_AFL
        elif self == FuzzerType.ANGORA:
            return PB2FuzzerType.FUZZER_TYPE_ANGORA
        elif self == FuzzerType.QSYM:
            return PB2FuzzerType.FUZZER_TYPE_QSYM
        elif self == FuzzerType.LIBFUZZER:
            return PB2FuzzerType.FUZZER_TYPE_LIBFUZZER
        elif self == FuzzerType.HONGGFUZZ:
            return PB2FuzzerType.FUZZER_TYPE_HONGGFUZZ
        elif self == FuzzerType.AFLFAST:
            return PB2FuzzerType.FUZZER_TYPE_AFLFAST
        elif self == FuzzerType.FAIRFUZZ:
            return PB2FuzzerType.FUZZER_TYPE_FAIRFUZZ
        elif self == FuzzerType.RADAMSA:
            return PB2FuzzerType.FUZZER_TYPE_RADAMSA
        raise NotImplementedError


class Config(NamedTuple):
    # Driver config
    fuzzer_type: FuzzerType
    output_dir: Path
    docker_enabled: bool

    # QSYM specific
    afl_path: Optional[Path]
    target_cmdline: List[str]

    # ZeroMQ config
    ctrl_uri: str
    pull_uri: str
    push_uri: str
