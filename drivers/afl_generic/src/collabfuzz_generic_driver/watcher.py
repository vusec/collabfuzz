from .connection import FrameworkConnection
from .seedmsg_pb2 import SeedMsg
from .id_dicts import IDDicts
from .config import Config, FuzzerType

import watchdog.observers
from watchdog.observers import Observer
from watchdog.events import DirCreatedEvent, FileCreatedEvent

from pathlib import Path
import logging
from abc import ABC, abstractmethod
import time
from typing import Dict, Optional, List, Union, Iterable, Set
from time import sleep
from threading import Event, Condition, Thread


logger = logging.getLogger(__name__)


class WatcherException(Exception):
    pass


class _NewTestCaseHandler(watchdog.events.FileSystemEventHandler):
    def __init__(
        self,
        test_case_queue: List[Path],
        test_in_queue: Condition,
        test_case_blacklist: Set[Path],
    ):
        self._test_case_queue = test_case_queue
        self._test_in_queue = test_in_queue
        self._test_case_blacklist = test_case_blacklist

    def on_created(self, event: Union[DirCreatedEvent, FileCreatedEvent]):
        if isinstance(event, FileCreatedEvent):
            with self._test_in_queue:
                test_case_path = Path(event.src_path)

                # Filter out test cases that have already been recorded on startup
                if test_case_path not in self._test_case_blacklist:
                    logger.debug(f"Found new test case: {test_case_path}")
                    self._test_case_queue.append(test_case_path)
                    self._test_in_queue.notify()


class Watcher(ABC):
    QUEUE_POLL_TIMEOUT = 0.5  # Queue polling in seconds
    WAIT_DIR_TIMEOUT = 0.5  # Directory waiting timeout in seconds
    FILE_READ_DELAY = 0.1  # Delay before reading a test case just created in seconds

    def __init__(
        self, target_directories: Iterable[Path], connection: FrameworkConnection
    ):
        self._target_directories = target_directories
        self._connection = connection
        self._observer: Optional[watchdog.observers.Observer] = None

        self._test_case_queue: List[Path] = []
        self._test_case_blacklist: Set[Path] = set()
        self._stopping = Event()
        self._test_in_queue = Condition()
        self._push_thread = Thread(target=self._push_thread_run, name="push-thread")

    def _wait_for_dir(self, dir_path) -> None:
        # This is a helper function that can be used inside _manage_directories
        # to wait for a directory to be created.
        while not dir_path.is_dir():
            time.sleep(self.WAIT_DIR_TIMEOUT)

    def _manage_directories(self) -> None:
        # When this function terminates, all the target directories should be
        # present and ready to be observed.
        pass

    def _initialize_observer(self) -> None:
        self._observer = Observer()
        new_test_case_scheduler = _NewTestCaseHandler(
            self._test_case_queue, self._test_in_queue, self._test_case_blacklist
        )

        for target_directory in self._target_directories:
            logger.debug(f"Observing directory: {target_directory}")
            self._observer.schedule(new_test_case_scheduler, str(target_directory))

    def _scan_target_folders(self) -> None:
        test_cases = []
        for target_directory in self._target_directories:
            for test_case in target_directory.iterdir():
                if test_case.is_file():
                    logger.debug(f"Found existing test case: {test_case}")
                    test_cases.append(test_case)

        test_cases.sort(key=lambda file_path: file_path.stat().st_ctime)
        self._test_case_queue.extend(test_cases)

        # Ensure that test cases detected by this function are not reported again
        self._test_case_blacklist.update(test_cases)

    def start(self, daemon=False) -> None:
        logger.info("Starting watcher")

        logger.debug("Preparing directories")
        self._manage_directories()
        logger.debug("Initializing watcher")
        self._initialize_observer()
        assert self._observer is not None

        # The push thread will block waiting for new test cases in the queue.
        self._push_thread.daemon = daemon
        self._push_thread.start()

        with self._test_in_queue:
            # The observer will not add new paths to the queue until
            # _test_in_queue is released, but it will start accumulating
            # events.
            logger.debug("Starting observer")
            self._observer.start()

            # Test cases created before the observer is started will be added
            # to the queue.
            logger.debug("Scanning for existing test cases")
            self._scan_target_folders()
            self._test_in_queue.notify()

        # _test_in_queue is released and the observer start queuing the paths
        # accumulated during initialization

    def is_alive(self) -> bool:
        return (
            self._push_thread.is_alive()
            and self._observer is not None
            and self._observer.is_alive()
        )

    def stop(self) -> None:
        logger.info("Stopping watcher")

        self._stopping.set()

        if self._observer is not None:
            self._observer.stop()

    def _ignore_test_case(self, test_case_path: Path) -> bool:
        return False

    @abstractmethod
    def _get_test_case_type(self, test_case_path: Path) -> SeedMsg.SeedType:
        ...

    def _get_test_case_parents(self, test_case_path: Path) -> Iterable[str]:
        return []

    def _process_server_id(self, test_case_path: Path, server_id: str) -> None:
        pass

    def _process_test_case(self) -> None:
        test_case_path = self._test_case_queue.pop(0)
        logger.debug(f"Processing test case: {test_case_path}")

        if self._ignore_test_case(test_case_path):
            logger.debug(f"Test case ignored: {test_case_path}")
            return

        # Give some time to the fuzzer to finish writing the file
        sleep(self.FILE_READ_DELAY)

        with open(test_case_path, "rb") as test_case_file:
            test_case = test_case_file.read()

        seed_msg = SeedMsg()
        seed_msg.id = test_case_path.name
        seed_msg.content = test_case
        seed_msg.type = self._get_test_case_type(test_case_path)
        seed_msg.parent_ids.extend(self._get_test_case_parents(test_case_path))

        server_id = self._connection.push_test_case(seed_msg)
        self._process_server_id(test_case_path, server_id)

    def _push_thread_run(self) -> None:
        logger.info("Starting push thread")

        with self._test_in_queue:
            while not self._stopping.is_set():
                if self._test_in_queue.wait_for(
                    lambda: len(self._test_case_queue) > 0, self.QUEUE_POLL_TIMEOUT
                ):
                    self._process_test_case()


def decode_afl_filename(filename: str) -> Dict[str, Optional[str]]:
    tokens = filename.split(",")
    metadata = {}
    for token in tokens:
        token_list = token.split(":")

        # Flag
        if len(token_list) == 1:
            key = token
            value = None
        # Key-value pair
        elif len(token_list) == 2:
            key = token_list[0]
            value = token_list[1]
        # Only special case with three tokens: val:be:+123
        elif len(token_list) == 3 and token_list[0] == "val" and token_list[1] == "be":
            key = token_list[0]
            value = token_list[1] + ":" + token_list[2]
        elif len(token_list) == 3 and token_list[0] == "src" and token_list[1] == "id":
            # QSYM weird bug case
            key = token_list[0]
            value = token_list[2]
        else:
            raise ValueError(f"Invalid token: {token}")

        metadata[key] = value
    return metadata


class AFLWatcher(Watcher):
    def __init__(
        self, config: Config, connection: FrameworkConnection, id_dicts: IDDicts,
    ):
        # AFL reuses the output directory if it already exists, but creates the
        # directories in it
        fuzzer_dir = config.output_dir / str(config.fuzzer_type)
        fuzzer_dir.mkdir(parents=True, exist_ok=True)

        target_directories = (
            fuzzer_dir / "queue",
            fuzzer_dir / "crashes",
            fuzzer_dir / "hangs",
        )

        super().__init__(target_directories, connection)

        self._id_dicts = id_dicts

    def _manage_directories(self) -> None:
        # AFL deletes the directories that need to be observed, if they exist,
        # and then it creates them again. As a consequence, we wait for it to
        # create them and then we start the observers.
        for target_directory in self._target_directories:
            self._wait_for_dir(target_directory)

    def _ignore_test_case(self, test_case_path: Path) -> bool:
        # Ignore files that do not conform to the naming convention
        return not test_case_path.name.startswith("id:")

    def _get_test_case_type(self, test_case_path: Path) -> SeedMsg.SeedType:
        if test_case_path.parts[-2] == "crashes":
            test_case_type = SeedMsg.SeedType.CRASH
        elif test_case_path.parts[-2] == "hangs":
            test_case_type = SeedMsg.SeedType.HANG
        elif test_case_path.parts[-2] == "queue":
            test_case_type = SeedMsg.SeedType.NORMAL
        else:
            raise ValueError("Unknown seed type observed.")

        return test_case_type

    def _get_test_case_parents(self, test_case_path: Path) -> Iterable[str]:
        afl_metadata = decode_afl_filename(test_case_path.name)

        if "orig" in afl_metadata:
            # A seed test case has no parents
            return []

        # AFL marks all test cases that are not seeds with "src"
        assert afl_metadata["src"] is not None

        parent_ids = []

        if "sync" in afl_metadata:
            # If the test case was imported from the framework folder
            if int(afl_metadata["src"]) in self._id_dicts.sync_to_server_ids:
                parent_server_id = self._id_dicts.sync_to_server_ids[
                    int(afl_metadata["src"])
                ]
                parent_ids.append(parent_server_id)
            else:
                logger.warning(f"Missing sync_to_server_ids key: {afl_metadata['src']}")

        else:
            # If the test case was generated by AFL itself
            for parent_id in afl_metadata["src"].split("+"):
                if int(parent_id) in self._id_dicts.local_to_server_ids:
                    parent_server_id = self._id_dicts.local_to_server_ids[
                        int(parent_id)
                    ]
                    parent_ids.append(parent_server_id)
                else:
                    logger.warning(f"Missing local_to_server_ids key: {parent_id}")

        return parent_ids

    def _process_server_id(self, test_case_path: Path, server_id: str) -> None:
        test_case_type = self._get_test_case_type(test_case_path)
        if test_case_type == SeedMsg.SeedType.NORMAL:
            afl_metadata = decode_afl_filename(test_case_path.name)
            assert afl_metadata["id"] is not None
            test_case_id = int(afl_metadata["id"])
            self._id_dicts.local_to_server_ids[test_case_id] = server_id


class AngoraWatcher(Watcher):
    def __init__(self, config: Config, connection: FrameworkConnection):
        self._angora_dir = config.output_dir / str(config.fuzzer_type)

        target_directories = (
            self._angora_dir / "queue",
            self._angora_dir / "crashes",
            self._angora_dir / "hangs",
        )

        super().__init__(target_directories, connection)

    def _manage_directories(self) -> None:
        # Wait for Angora to create its output directory
        self._wait_for_dir(self._angora_dir)

        # Wait for Angora to create all the directories that need to be
        # observed inside its output folder
        for target_directory in self._target_directories:
            self._wait_for_dir(target_directory)

    def _ignore_test_case(self, test_case_path: Path) -> bool:
        # Ignore files that do not conform to the naming convention
        return not test_case_path.name.startswith("id:")

    def _get_test_case_type(self, test_case_path: Path) -> SeedMsg.SeedType:
        if test_case_path.parts[-2] == "crashes":
            test_case_type = SeedMsg.SeedType.CRASH
        elif test_case_path.parts[-2] == "hangs":
            test_case_type = SeedMsg.SeedType.HANG
        elif test_case_path.parts[-2] == "queue":
            test_case_type = SeedMsg.SeedType.NORMAL
        else:
            raise ValueError("Unknown seed type observed.")

        return test_case_type


class QSYMWatcher(Watcher):
    def __init__(
        self, config: Config, connection: FrameworkConnection, id_dicts: IDDicts,
    ):
        # QSYM reuses all directories if they are found, so create them in advance
        fuzzer_dir = config.output_dir / str(config.fuzzer_type)

        target_directories = (
            fuzzer_dir / "queue",
            fuzzer_dir / "errors",
            fuzzer_dir / "hangs",
        )

        for target_directory in target_directories:
            target_directory.mkdir(parents=True, exist_ok=True)

        super().__init__(target_directories, connection)

        self._id_dicts = id_dicts

    def _ignore_test_case(self, test_case_path: Path) -> bool:
        # Ignore files that do not conform to the naming convention
        return not test_case_path.name.startswith("id:")

    def _get_test_case_type(self, test_case_path: Path) -> SeedMsg.SeedType:
        if test_case_path.parts[-2] == "errors":
            test_case_type = SeedMsg.SeedType.CRASH
        elif test_case_path.parts[-2] == "hangs":
            test_case_type = SeedMsg.SeedType.HANG
        elif test_case_path.parts[-2] == "queue":
            test_case_type = SeedMsg.SeedType.NORMAL
        else:
            raise ValueError("Unknown seed type observed.")

        return test_case_type

    def _get_test_case_parents(self, test_case_path: Path) -> Iterable[str]:
        afl_metadata = decode_afl_filename(test_case_path.name)

        if "src" not in afl_metadata:
            # QSYM does not save "src" info for hangs and crashes
            return []
        assert afl_metadata["src"] is not None

        parent_ids = []

        # QSYM always imports tests from the framework folder
        if int(afl_metadata["src"]) in self._id_dicts.sync_to_server_ids:
            parent_server_id = self._id_dicts.sync_to_server_ids[
                int(afl_metadata["src"])
            ]
            parent_ids.append(parent_server_id)
        else:
            logger.warning(f"Missing sync_to_server_ids key: {afl_metadata['src']}")

        return parent_ids


class LibFuzzerWatcher(Watcher):
    def __init__(self, config: Config, connection: FrameworkConnection):
        target_directories = (
            config.output_dir / "queue",
            config.output_dir / "artifacts",
        )

        super().__init__(target_directories, connection)

    def _manage_directories(self) -> None:
        # The entity which starts the driver is responsible for creating the
        # target folders when using libfuzzer. The queue is used for the seeds
        # as well, so it needs to be created before.
        for target_directory in self._target_directories:
            logger.debug(f"Waiting on directory: {target_directory}")
            self._wait_for_dir(target_directory)

    def _ignore_test_case(self, test_case_path: Path) -> bool:
        # Filter out the seeds coming from the framework, they are written in
        # the queue folder.
        return test_case_path.name.startswith("framework-")

    def _get_test_case_type(self, test_case_path: Path) -> SeedMsg.SeedType:
        if test_case_path.name.startswith("crash-"):
            test_case_type = SeedMsg.SeedType.CRASH
        elif test_case_path.name.startswith("leak-"):
            test_case_type = SeedMsg.SeedType.CRASH
        elif test_case_path.name.startswith("timeout-"):
            test_case_type = SeedMsg.SeedType.HANG
        elif test_case_path.name.startswith("oom-"):
            test_case_type = SeedMsg.SeedType.HANG
        else:
            # Normal test cases have no prefix for libfuzzer, so assume NORMAL
            # by default
            test_case_type = SeedMsg.SeedType.NORMAL

        return test_case_type


class HonggFuzzWatcher(Watcher):
    def __init__(self, config: Config, connection: FrameworkConnection):
        self._config = config

        target_directories = (
            config.output_dir / "queue",
            config.output_dir / "crashes",
        )

        # HonggFuzz reuses the directories if it finds them, so just ensure
        # that they exist.
        for target_directory in target_directories:
            target_directory.mkdir(exist_ok=True, parents=True)

        super().__init__(target_directories, connection)

    def _get_test_case_type(self, test_case_path: Path) -> SeedMsg.SeedType:
        if test_case_path.parent.name == "crashes":
            test_case_type = SeedMsg.SeedType.CRASH
        elif test_case_path.parent.name == "queue":
            test_case_type = SeedMsg.SeedType.NORMAL
        else:
            raise ValueError("Unknown seed type observed.")

        return test_case_type


def get_watcher(
    config: Config, connection: FrameworkConnection, id_dicts: Optional[IDDicts],
) -> Watcher:
    watcher: Watcher

    if (
        config.fuzzer_type == FuzzerType.AFL
        or config.fuzzer_type == FuzzerType.AFLFAST
        or config.fuzzer_type == FuzzerType.FAIRFUZZ
        or config.fuzzer_type == FuzzerType.RADAMSA
    ):
        assert id_dicts is not None
        watcher = AFLWatcher(config, connection, id_dicts)
    elif config.fuzzer_type == FuzzerType.ANGORA:
        watcher = AngoraWatcher(config, connection)
    elif config.fuzzer_type == FuzzerType.QSYM:
        assert id_dicts is not None
        watcher = QSYMWatcher(config, connection, id_dicts)
    elif config.fuzzer_type == FuzzerType.LIBFUZZER:
        watcher = LibFuzzerWatcher(config, connection)
    elif config.fuzzer_type == FuzzerType.HONGGFUZZ:
        watcher = HonggFuzzWatcher(config, connection)
    else:
        raise Exception(f"Invalid fuzzer type: {config.fuzzer_type}")

    return watcher
