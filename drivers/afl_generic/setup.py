from setuptools import setup, find_packages
from setuptools.command.build_py import build_py as _build_py
from setuptools.command.develop import develop as _develop
from distutils.command.clean import clean as _clean
from distutils.spawn import find_executable
from setuptools import Command

from pathlib import Path
import os
import subprocess

try:
    import mypy_protobuf

    assert mypy_protobuf
except ModuleNotFoundError:
    mypy_protobuf = False


PROJECT_ROOT = Path(__file__).parent.resolve()
PROTOS_PATH = PROJECT_ROOT / "protos"

PROTO_MAP = {
    PROTOS_PATH / "seedmsg.proto": PROJECT_ROOT / "src/collabfuzz_generic_driver",
    PROTOS_PATH / "fuzzerctrlmsg.proto": PROJECT_ROOT / "src/collabfuzz_generic_driver",
}


def find_protoc():
    if "PROTOC" in os.environ:
        protoc = Path(os.environ["PROTOC"])
        if not protoc.is_file():
            print(f"protoc not found, check PROTOC: {protoc}")
            exit(1)
    else:
        protoc = find_executable("protoc")

    if protoc is None:
        print("protoc not found in PATH")
        exit(1)

    return protoc


def _build_protos(proto_map):
    protoc = find_protoc()

    for source, output_dir in proto_map.items():
        include_dir = source.parent
        source_file = source.name
        if not source.is_file():
            print(f"proto file does not exist: {source}")
            exit(1)

        print(f"generating python module for {source_file}")

        cmdline = [str(protoc)]
        cmdline.append(f"-I{include_dir}")
        cmdline.append(f"--python_out={output_dir}")

        if mypy_protobuf:
            cmdline.append(f"--mypy_out={output_dir}")

        cmdline.append(source_file)

        subprocess.run(cmdline).check_returncode()


class build_protos(Command):
    user_options = []

    def initialize_options(self):
        pass

    def finalize_options(self):
        pass

    def run(self):
        _build_protos(PROTO_MAP)


class build_py(_build_py):
    def run(self):
        _build_protos(PROTO_MAP)
        super().run()


class develop(_develop):
    def run(self):
        _build_protos(PROTO_MAP)
        super().run()


class clean(_clean):
    def run(self):
        module_dir = PROJECT_ROOT / "src/collabfuzz_generic_driver"
        for py_file in module_dir.glob("**/*_pb2.py*"):
            py_file.unlink()


setup(
    name="collabfuzz_generic_driver",
    version="0.1",
    packages=find_packages("src"),
    package_dir={"": "src"},
    entry_points={
        "console_scripts": [
            "collabfuzz_generic_driver = collabfuzz_generic_driver:main"
        ]
    },
    install_requires=["protobuf>=3.9", "watchdog>=0.9", "pyzmq>=18.0", "docker>=4.1.0"],
    cmdclass={
        "build_protos": build_protos,
        "build_py": build_py,
        "develop": develop,
        "clean": clean,
    },
    include_package_data=True,
)
