from .connection import FrameworkConnection, TimeoutExpired, ConnectionException
from .fuzzerctrlmsg_pb2 import FuzzerCtrlMsg, CtrlCommand
from .seedmsg_pb2 import JobMsg
from .id_dicts import IDDicts
from .config import Config, FuzzerType

from threading import Thread, Event
import logging
import shutil
import docker
from typing import Set, Optional
from pathlib import Path
import time
from abc import ABC, abstractmethod


logger = logging.getLogger(__name__)


class ReceiverException(Exception):
    pass


class Receiver(ABC):
    POLLING_TIMEOUT = 5000  # Polling timeout in milliseconds
    HOSTNAME_TIMEOUT = 0.5  # docker_hostname file polling timeout in seconds

    def __init__(
        self, config: Config, connection: FrameworkConnection, target_path: Path,
    ):
        super().__init__()

        self._config = config
        self._connection = connection
        self._target_path = target_path

        self._stopping = Event()
        self._receiver_thread = Thread(target=self.run, name="receiver-thread")
        self._container: Optional[docker.models.containers.Container] = None
        self._received_ids: Set[str] = set()

        logger.info("Receiver initialized.")

    def start(self, daemon=False) -> None:
        self._receiver_thread.daemon = daemon
        self._receiver_thread.start()

    def is_alive(self) -> bool:
        return self._receiver_thread.is_alive()

    def stop(self, timeout: Optional[int] = None) -> None:
        self._stopping.set()

        # It is necessary to wait for the polling to be terminated, otherwise
        # the calling thread may decide to close the connection while the
        # polling is still being performed.
        logger.debug("Waiting for polling to finish.")
        self._receiver_thread.join(timeout)
        logger.info("Receiver stopped.")

    def _handle_ctrl_msg(self, msg: FuzzerCtrlMsg) -> None:
        if self._container is None:
            logger.warning(f"Not running in docker mode, control message discarded")
            return

        logger.info(f"Executing control command: {msg.command}")

        if msg.command == CtrlCommand.COMMAND_RUN:
            self._container.unpause()
        elif msg.command == CtrlCommand.COMMAND_PAUSE:
            self._container.pause()
        elif msg.command == CtrlCommand.COMMAND_KILL:
            self._container.kill()
        elif msg.command == CtrlCommand.COMMAND_SET_PRIORITY:
            self._container.update(cpu_shares=msg.fuzzer_priority)
        else:
            logger.warning(f"Unexpected control command: {msg.command}")

    @abstractmethod
    def _get_test_case_filename(self, test_case_id: str) -> str:
        ...

    def _handle_job_msg(self, msg: JobMsg) -> None:
        for test_case in msg.seeds:
            # Assuming the id is unique for this fuzzer
            if test_case.id in self._received_ids:
                logger.error("Test case already received: {test_case.id}")
                continue
            self._received_ids.add(test_case.id)

            test_case_filename = self._get_test_case_filename(test_case.id)
            test_case_path = self._target_path / test_case_filename
            if test_case_path.exists():  # It should never happen
                raise ReceiverException(f"Test case already exists: {test_case_path}")

            with open(test_case_path, "wb") as test_case_file:
                test_case_file.write(test_case.content)

            logger.info(f"Writing test case {test_case.id} to {test_case_filename}")

    def _init_container(self) -> None:
        # Get container id from "docker_hostname" file. It is created when the
        # fuzzer container is started, but it may not be immediately present.
        hostname_path = self._config.output_dir / "docker_hostname"
        while not hostname_path.exists():
            # Do not get stuck here if the process is being killed
            if self._stopping.is_set():
                return
            time.sleep(self.HOSTNAME_TIMEOUT)

        with open(hostname_path) as hostname_file:
            container_name = hostname_file.read().strip()

        logger.info(f'Take control of docker container: "{container_name}"')
        docker_client = docker.from_env()
        self._container = docker_client.containers.get(container_name)

    def run(self) -> None:
        if self._config.docker_enabled:
            self._init_container()

        # The fuzzer needs to be marked as ready before receiving
        # additional job messages. If this does not happen, the
        # framework will think that the previous test cases are still
        # being processed.
        self._connection.report_ready()

        while not self._stopping.is_set():
            try:
                logger.debug("Waiting for new test cases or control message")
                msg = self._connection.pull_from_server(self.POLLING_TIMEOUT)

                if isinstance(msg, FuzzerCtrlMsg):
                    self._handle_ctrl_msg(msg)
                elif isinstance(msg, JobMsg):
                    self._handle_job_msg(msg)
                else:
                    raise ReceiverException(f"Unknown message type: {type(msg)}")

                self._connection.report_ready()

            except TimeoutExpired:
                # Force checking _stopping when the timeout expires
                pass

            except ConnectionException as e:
                # The connection with the server is not in a known state
                # anymore, so kill this thread.
                logger.error(f"Killing receiver thread: {e}")
                break


class AFLReceiver(Receiver):
    def __init__(
        self, config: Config, connection: FrameworkConnection, id_dicts: IDDicts,
    ):
        target_path = config.output_dir / "framework/queue"

        # AFL will not sync with folders that do not contain a queue folder,
        # so it needs to be created before starting the fuzzer. It will
        # always be empty on launch, so that the framework will start the
        # communication in a known state.
        shutil.rmtree(target_path, ignore_errors=True)
        target_path.mkdir(parents=True)

        super().__init__(config, connection, target_path)

        # AFL will pick only test cases that respect the serial ID encoding
        self._serial_id = 0
        self._id_dicts = id_dicts

    def _get_test_case_filename(self, test_case_id: str) -> str:
        # Assuming id:123456,...
        current_id = self._serial_id
        self._serial_id += 1

        self._id_dicts.sync_to_server_ids[current_id] = test_case_id

        return "id:{:06}".format(current_id)


class QSYMReceiver(AFLReceiver):
    def __init__(
        self, config: Config, connection: FrameworkConnection, id_dicts: IDDicts,
    ):
        super().__init__(config, connection, id_dicts)

        # QSYM reads from the framework's directory the `fuzzer_stats` file to
        # get information about the running AFL instance (here "faked" by the
        # framework). In particular, it expects a line of the format
        # `command_line: [cmd]` where `cmd` is the command used to launch AFL.
        # From the `cmd` QSYM takes everything after `--`, the directory
        # containing `afl-fuzz` and whether `-Q` is in `cmd`.
        stats_path = config.output_dir / "framework/fuzzer_stats"
        with open(stats_path, "w") as stats_file:
            cmdline_str = " ".join(config.target_cmdline)
            stats_file.write(
                f"command_line: {config.afl_path}/afl-fuzz -- {cmdline_str}"
            )


class LibFuzzerReceiver(Receiver):
    def __init__(
        self, config: Config, connection: FrameworkConnection,
    ):
        target_path = config.output_dir / "queue"

        # The same folder is shared between watcher, receiver and is also used
        # to store the seeds, so it should be create by whoever needs it first.
        target_path.mkdir(parents=True, exist_ok=True)

        super().__init__(config, connection, target_path)

    def _get_test_case_filename(self, test_case_id: str) -> str:
        # The test cases sent by the framework should be appropriately prefixed
        # so that the watcher can easily filter them out.
        return f"framework-{test_case_id}"


class HonggFuzzReceiver(Receiver):
    def __init__(
        self, config: Config, connection: FrameworkConnection,
    ):
        target_path = config.output_dir / "sync"

        # The sync folder is used only by the receiver and is not created by
        # our patched version of honggfuzz. Remove it if existent to start in a
        # known state.
        shutil.rmtree(target_path, ignore_errors=True)
        target_path.mkdir(parents=True)

        super().__init__(config, connection, target_path)

    def _get_test_case_filename(self, test_case_id: str) -> str:
        # The name of the test case is not important for our patched honggfuzz
        return test_case_id


def get_receiver(
    config: Config, connection: FrameworkConnection, id_dicts: Optional[IDDicts],
) -> Receiver:
    receiver: Receiver

    if (
        config.fuzzer_type == FuzzerType.AFL
        or config.fuzzer_type == FuzzerType.AFLFAST
        or config.fuzzer_type == FuzzerType.FAIRFUZZ
        or config.fuzzer_type == FuzzerType.RADAMSA
    ):
        assert id_dicts is not None
        receiver = AFLReceiver(config, connection, id_dicts)
    elif config.fuzzer_type == FuzzerType.QSYM:
        assert id_dicts is not None
        receiver = QSYMReceiver(config, connection, id_dicts)
    elif config.fuzzer_type == FuzzerType.LIBFUZZER:
        receiver = LibFuzzerReceiver(config, connection)
    elif config.fuzzer_type == FuzzerType.HONGGFUZZ:
        receiver = HonggFuzzReceiver(config, connection)
    else:
        raise Exception(f"Invalid fuzzer type: {config.fuzzer_type}")

    return receiver
