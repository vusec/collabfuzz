from collabfuzz_generic_driver.seedmsg_pb2 import SeedMsg, JobMsg
from collabfuzz_generic_driver.fuzzerctrlmsg_pb2 import FuzzerCtrlMsg, CtrlCommand
from collabfuzz_generic_driver.config import Config, FuzzerType
from collabfuzz_generic_driver.connection import FrameworkConnection, TimeoutExpired
from collabfuzz_generic_driver.id_dicts import IDDicts
from collabfuzz_generic_driver.receiver import (
    AFLReceiver,
    QSYMReceiver,
    LibFuzzerReceiver,
    HonggFuzzReceiver,
)

from typing import Union, Optional, List
from unittest.mock import Mock, MagicMock
from threading import Event
from time import sleep
from pathlib import Path
import logging
import docker

TEST_SERVER_ID = "test_server_id"
TEST_SEED_ID = "test_seed_id"
TEST_SEED_CONTENT = b"mock_seed_content"
TEST_AFL_PATH = Path("/path/to/afl_root")
TEST_TARGET_CMDLINE = ["mock_target", "--mock=argument"]
TEST_DOCKER_ID = "test_docker_id"
TEST_FUZZER_PRIORITY = 7


class MockConnection:
    def __init__(self, pull_objects: List[Union[FuzzerCtrlMsg, JobMsg]]):
        self.push_test_case = Mock(
            spec_set=FrameworkConnection.push_test_case, return_value=TEST_SERVER_ID
        )
        self.report_ready = Mock(spec_set=FrameworkConnection.report_ready)
        self.close = Mock(spec_set=FrameworkConnection.close)

        self._pull_event = Event()
        self._pull_objects = pull_objects

    def pull_from_server(
        self, timeout: Optional[int] = None
    ) -> Union[JobMsg, FuzzerCtrlMsg]:
        logging.debug("Mock pull_from_server called")

        # The timeout is in milliseconds
        real_timeout = timeout / 1000 if timeout is not None else None
        if not self._pull_event.wait(real_timeout):
            logging.debug("Mock pull_from_server timed out")
            raise TimeoutExpired

        self._pull_event.clear()

        logging.debug("Returning pull object")
        return self._pull_objects.pop(0)

    def trigger_pull(self) -> None:
        self._pull_event.set()


def _get_mock_jobmsg(seed_id=TEST_SEED_ID):
    seed_msg = SeedMsg()
    seed_msg.id = seed_id
    seed_msg.content = TEST_SEED_CONTENT

    job_msg = JobMsg()
    job_msg.seeds.append(seed_msg)

    return job_msg


def _get_mock_ctrlmsg(command):
    ctrl_msg = FuzzerCtrlMsg()
    if command == "run":
        ctrl_msg.command = CtrlCommand.COMMAND_RUN
    elif command == "pause":
        ctrl_msg.command = CtrlCommand.COMMAND_PAUSE
    elif command == "kill":
        ctrl_msg.command = CtrlCommand.COMMAND_KILL
    elif command == "set_priority":
        ctrl_msg.command = CtrlCommand.COMMAND_SET_PRIORITY
        ctrl_msg.fuzzer_priority = TEST_FUZZER_PRIORITY
    else:
        raise ValueError

    return ctrl_msg


def test_afl_receiver(caplog, tmp_path, monkeypatch):
    caplog.set_level(logging.DEBUG)

    config = Config(
        fuzzer_type=FuzzerType.AFL,  # unused
        output_dir=tmp_path,
        docker_enabled=False,
        afl_path=None,  # unused
        target_cmdline=[],  # unused
        ctrl_uri="",  # unused
        pull_uri="",  # unused
        push_uri="",  # unused
    )

    job_msgs = [_get_mock_jobmsg(), _get_mock_jobmsg(TEST_SEED_ID + "_2")]

    connection = MockConnection(job_msgs)
    id_dicts = IDDicts(MagicMock(), MagicMock())

    # Shorten the timeout to make test succeed more quickly
    monkeypatch.setattr(AFLReceiver, "POLLING_TIMEOUT", 500)

    receiver = AFLReceiver(config, connection, id_dicts)

    # AFL expects a parallel instance to store its queue in
    # "<outdir>/<name>/queue", this folder needs to be created by the driver.
    queue_path = tmp_path / "framework/queue"
    assert queue_path.is_dir()

    receiver.start(daemon=True)
    sleep(0.2)

    # The receiver should report that it is ready to receive new seeds
    connection.report_ready.assert_called_once()
    connection.report_ready.reset_mock()

    connection.trigger_pull()
    sleep(0.2)

    # The seed should be written down in valid AFL seed format to be imported
    # by the fuzzer
    seed_path = queue_path / "id:000000"
    assert seed_path.is_file()
    with open(seed_path, "rb") as seed_file:
        assert seed_file.read() == TEST_SEED_CONTENT

    # Check that the seed ID has been correctly stored
    id_dicts.sync_to_server_ids.__setitem__.assert_called_with(0, TEST_SEED_ID)

    # The receiver should report that it is ready to receive new seeds
    connection.report_ready.assert_called_once()
    connection.report_ready.reset_mock()

    connection.trigger_pull()
    sleep(0.2)

    # Check that the ID was actually incremented
    seed_path = queue_path / "id:000001"
    assert seed_path.is_file()
    with open(seed_path, "rb") as seed_file:
        assert seed_file.read() == TEST_SEED_CONTENT

    # Check that the seed ID has been correctly stored
    id_dicts.sync_to_server_ids.__setitem__.assert_called_with(1, TEST_SEED_ID + "_2")

    receiver.stop(2)
    assert not receiver.is_alive()


def test_qsym_receiver(caplog, tmp_path, monkeypatch):
    caplog.set_level(logging.DEBUG)

    config = Config(
        fuzzer_type=FuzzerType.AFL,  # unused
        output_dir=tmp_path,
        docker_enabled=False,
        afl_path=TEST_AFL_PATH,
        target_cmdline=TEST_TARGET_CMDLINE,
        ctrl_uri="",  # unused
        pull_uri="",  # unused
        push_uri="",  # unused
    )

    job_msg = _get_mock_jobmsg()

    connection = MockConnection([job_msg])
    id_dicts = IDDicts(MagicMock(), MagicMock())

    # Shorten the timeout to make test succeed more quickly
    monkeypatch.setattr(QSYMReceiver, "POLLING_TIMEOUT", 500)

    receiver = QSYMReceiver(config, connection, id_dicts)

    # QSYM expects a parallel instance to store its queue in
    # "<outdir>/<name>/queue", this folder needs to be created by the driver.
    queue_path = tmp_path / "framework/queue"
    assert queue_path.is_dir()

    # QSYM also expects that a parallel AFL instance creates a fuzzer_stats file
    # with the command line used to start the fuzzer
    stats_path = tmp_path / "framework/fuzzer_stats"
    assert stats_path.is_file()
    with open(stats_path) as stats_file:
        content = stats_file.read()
        target_cmdline = " ".join(TEST_TARGET_CMDLINE)
        assert content == f"command_line: {TEST_AFL_PATH}/afl-fuzz -- {target_cmdline}"

    receiver.start(daemon=True)
    sleep(0.2)

    # The receiver should report that it is ready to receive new seeds
    connection.report_ready.assert_called_once()

    connection.trigger_pull()
    sleep(0.2)

    # The seed should be written down in valid AFL seed format to be imported
    # by QSYM. Writing down the ID part is sufficient to pass all checks.
    seed_path = queue_path / "id:000000"
    assert seed_path.is_file()
    with open(seed_path, "rb") as seed_file:
        assert seed_file.read() == TEST_SEED_CONTENT

    # Check that the seed ID has been correctly stored
    id_dicts.sync_to_server_ids.__setitem__.assert_called_with(0, TEST_SEED_ID)

    receiver.stop(2)
    assert not receiver.is_alive()


def test_libfuzzer_receiver(caplog, tmp_path, monkeypatch):
    caplog.set_level(logging.DEBUG)

    config = Config(
        fuzzer_type=FuzzerType.AFL,  # unused
        output_dir=tmp_path,
        docker_enabled=False,
        afl_path=None,  # unused
        target_cmdline=[],  # unused
        ctrl_uri="",  # unused
        pull_uri="",  # unused
        push_uri="",  # unused
    )

    job_msg = _get_mock_jobmsg()

    connection = MockConnection([job_msg])

    # LibFuzzer expects a parallel instance to store its test cases in
    # "<outdir>/queue", which is shared among all instances and thus with
    # LibFuzzerWatcher as well. The component which starts the driver is
    # supposed to create the folder.
    queue_path = tmp_path / "queue"
    queue_path.mkdir()

    # Shorten the timeout to make test succeed more quickly
    monkeypatch.setattr(LibFuzzerReceiver, "POLLING_TIMEOUT", 500)

    receiver = LibFuzzerReceiver(config, connection)
    receiver.start(daemon=True)
    sleep(0.2)

    # The receiver should report that it is ready to receive new seeds
    connection.report_ready.assert_called_once()

    connection.trigger_pull()
    sleep(0.2)

    # The seed should be written down with a "framework-" prefix, so that the
    # watcher can easily filter it out when reporting to the server
    seed_path = queue_path / f"framework-{TEST_SEED_ID}"
    assert seed_path.is_file()
    with open(seed_path, "rb") as seed_file:
        assert seed_file.read() == TEST_SEED_CONTENT

    receiver.stop(2)
    assert not receiver.is_alive()


def test_honggfuzz_receiver(caplog, tmp_path, monkeypatch):
    caplog.set_level(logging.DEBUG)

    config = Config(
        fuzzer_type=FuzzerType.AFL,  # unused
        output_dir=tmp_path,
        docker_enabled=False,
        afl_path=None,  # unused
        target_cmdline=[],  # unused
        ctrl_uri="",  # unused
        pull_uri="",  # unused
        push_uri="",  # unused
    )

    job_msg = _get_mock_jobmsg()

    connection = MockConnection([job_msg])

    # Shorten the timeout to make test succeed more quickly
    monkeypatch.setattr(HonggFuzzReceiver, "POLLING_TIMEOUT", 500)

    receiver = HonggFuzzReceiver(config, connection)

    # Verify that the driver is creating the sync folder if not already present
    sync_path = tmp_path / "sync"
    assert sync_path.is_dir()

    receiver.start(daemon=True)
    sleep(0.2)

    # The receiver should report that it is ready to receive new seeds
    connection.report_ready.assert_called_once()

    connection.trigger_pull()
    sleep(0.2)

    # The seed should be written down in the "sync" folder, the name is not relevant
    seed_path = sync_path / TEST_SEED_ID
    assert seed_path.is_file()
    with open(seed_path, "rb") as seed_file:
        assert seed_file.read() == TEST_SEED_CONTENT

    receiver.stop(2)
    assert not receiver.is_alive()


# The docker management functionality is the same for all the receivers, so it
# is tested only on LibFuzzerReceiver for simplicity
def test_docker_libfuzzer_receiver(caplog, tmp_path, monkeypatch):
    caplog.set_level(logging.DEBUG)

    config = Config(
        fuzzer_type=FuzzerType.AFL,  # unused
        output_dir=tmp_path,
        docker_enabled=True,
        afl_path=None,  # unused
        target_cmdline=[],  # unused
        ctrl_uri="",  # unused
        pull_uri="",  # unused
        push_uri="",  # unused
    )

    ctrl_msgs = [
        _get_mock_ctrlmsg("pause"),
        _get_mock_ctrlmsg("run"),
        _get_mock_ctrlmsg("set_priority"),
        _get_mock_ctrlmsg("kill"),
    ]
    connection = MockConnection(ctrl_msgs)

    # LibFuzzer expects a parallel instance to store its test cases in
    # "<outdir>/queue", which is shared among all instances and thus with
    # LibFuzzerWatcher as well. The component which starts the driver is
    # supposed to create the folder.
    queue_path = tmp_path / "queue"
    queue_path.mkdir()

    # Shorten the timeouts to make test succeed more quickly
    monkeypatch.setattr(LibFuzzerReceiver, "POLLING_TIMEOUT", 500)
    monkeypatch.setattr(LibFuzzerReceiver, "HOSTNAME_TIMEOUT", 0.1)

    # Patch docker module so that the client is a mock object
    mock_docker_client = Mock(name="docker.from_env()")
    monkeypatch.setattr(docker, "from_env", lambda: mock_docker_client)

    receiver = LibFuzzerReceiver(config, connection)
    receiver.start(daemon=True)
    sleep(0.2)

    # When docker is enabled, the script which launches the binary should esure
    # that the docker_hostname file is present and contains the ID of the
    # target container. This may happen after the receiver is started though,
    # test this.
    with open(tmp_path / "docker_hostname", "w") as hostname_file:
        hostname_file.write(TEST_DOCKER_ID)
    sleep(0.2)

    mock_container = mock_docker_client.containers.get(TEST_DOCKER_ID)

    connection.report_ready.assert_called_once()
    connection.report_ready.reset_mock()
    connection.trigger_pull()  # pause
    sleep(0.1)
    mock_container.pause.assert_called_once()

    connection.report_ready.assert_called_once()
    connection.report_ready.reset_mock()
    connection.trigger_pull()  # run
    sleep(0.1)
    mock_container.unpause.assert_called_once()

    connection.report_ready.assert_called_once()
    connection.report_ready.reset_mock()
    connection.trigger_pull()  # set_priority
    sleep(0.1)
    mock_container.update.assert_called_once_with(cpu_shares=TEST_FUZZER_PRIORITY)

    connection.report_ready.assert_called_once()
    connection.report_ready.reset_mock()
    connection.trigger_pull()  # kill
    sleep(0.1)
    mock_container.kill.assert_called_once()

    receiver.stop(2)
    assert not receiver.is_alive()
