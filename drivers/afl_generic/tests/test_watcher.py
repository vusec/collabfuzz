from collabfuzz_generic_driver.watcher import (
    decode_afl_filename,
    AFLWatcher,
    AngoraWatcher,
    QSYMWatcher,
    LibFuzzerWatcher,
    HonggFuzzWatcher,
)
from collabfuzz_generic_driver.config import FuzzerType, Config
from collabfuzz_generic_driver.id_dicts import IDDicts
from collabfuzz_generic_driver.seedmsg_pb2 import SeedMsg

import logging
from unittest.mock import Mock
from threading import Barrier, Thread
from time import sleep

TEST_SEED_ID = "mock_seed_id"
TEST_QUEUE_ID = "mock_queue_id"
TEST_CRASH_ID = "mock_crash_id"
TEST_HANG_ID = "mock_hang_id"
TEST_SYNC_ID = "mock_sync_id"
TEST_SEED_CONTENT = b"mock_seed_content"
TEST_QUEUE_CONTENT = b"mock_queue_content"
TEST_CRASH_CONTENT = b"mock_crash_content"
TEST_HANG_CONTENT = b"mock_hang_content"


def test_filename_decoding():
    filename1 = "id:000000,orig:seed"
    output1 = decode_afl_filename(filename1)
    assert output1["id"] == "000000"
    assert output1["orig"] == "seed"

    filename2 = "id:000001,src:000000,op:flip1,pos:0,+cov"
    output2 = decode_afl_filename(filename2)
    assert output2["id"] == "000001"
    assert output2["src"] == "000000"
    assert output2["op"] == "flip1"
    assert output2["pos"] == "0"
    assert "+cov" in output2

    filename3 = "id:000092,src:000040+000088,op:splice,rep:2"
    output3 = decode_afl_filename(filename3)
    assert output3["id"] == "000092"
    assert output3["src"] == "000040+000088"
    assert output3["op"] == "splice"
    assert output3["rep"] == "2"

    filename4 = "id:000024,src:000001,op:int16,pos:0,val:be:+512,+cov"
    output4 = decode_afl_filename(filename4)
    assert output4["id"] == "000024"
    assert output4["src"] == "000001"
    assert output4["op"] == "int16"
    assert output4["pos"] == "0"
    assert output4["val"] == "be:+512"
    assert "+cov" in output4

    filename_qsym = "id:000000,src:id:000008"
    output_qsym = decode_afl_filename(filename_qsym)
    assert output_qsym["id"] == "000000"
    assert output_qsym["src"] == "000008"


def _afl_thread(fuzzer_path, barrier):
    # This is also the order of creation AFL follows
    queue_path = fuzzer_path / "queue"
    crashes_path = fuzzer_path / "crashes"
    hangs_path = fuzzer_path / "hangs"

    queue_path.mkdir()
    crashes_path.mkdir()
    hangs_path.mkdir()

    barrier.wait()  # Wait for watcher to start

    with open(queue_path / "id:000000,orig:seed", "wb") as test_case_file:
        test_case_file.write(TEST_SEED_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    with open(
        queue_path / "id:000001,src:000000,op:havoc,rep:16,+cov", "wb"
    ) as test_case_file:
        test_case_file.write(TEST_QUEUE_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    with open(
        queue_path / "id:000002,sync:framework,src:000042", "wb"
    ) as test_case_file:
        test_case_file.write(TEST_QUEUE_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    with open(
        hangs_path / "id:000000,src:000001,op:havoc,rep:16", "wb"
    ) as test_case_file:
        test_case_file.write(TEST_HANG_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    with open(
        crashes_path / "id:000000,sig:06,src:000001,op:havoc,rep:8", "wb"
    ) as test_case_file:
        test_case_file.write(TEST_CRASH_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()


def test_afl_watcher(caplog, tmp_path):
    caplog.set_level(logging.DEBUG)

    config = Config(
        fuzzer_type=FuzzerType.AFL,
        output_dir=tmp_path,
        docker_enabled=False,  # unused
        afl_path=None,  # unused
        target_cmdline=[],  # unused
        ctrl_uri="",  # unused
        pull_uri="",  # unused
        push_uri="",  # unused
    )

    connection = Mock()
    id_dicts = IDDicts({}, {42: TEST_SYNC_ID})

    watcher = AFLWatcher(config, connection, id_dicts)

    # The fuzzer folder gets reused by AFL, so it can be created in advance
    fuzzer_path = tmp_path / str(config.fuzzer_type)
    assert fuzzer_path.is_dir()

    barrier = Barrier(2, timeout=2)
    fuzzer_thread = Thread(target=_afl_thread, args=(fuzzer_path, barrier), daemon=True)
    fuzzer_thread.start()

    watcher.start()

    connection.push_test_case = Mock(return_value=TEST_SEED_ID)
    barrier.wait()

    barrier.wait()
    # XXX: Make it assert_called_once when multiple seed reporting has been fixed
    connection.push_test_case.assert_called()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_SEED_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.NORMAL
    assert seed_msg.parent_ids == []
    assert id_dicts.local_to_server_ids[0] == TEST_SEED_ID
    connection.push_test_case = Mock(return_value=TEST_QUEUE_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_QUEUE_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.NORMAL
    assert seed_msg.parent_ids == [TEST_SEED_ID]
    assert id_dicts.local_to_server_ids[1] == TEST_QUEUE_ID
    connection.push_test_case = Mock(return_value=TEST_QUEUE_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_QUEUE_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.NORMAL
    assert seed_msg.parent_ids == [TEST_SYNC_ID]
    assert id_dicts.local_to_server_ids[2] == TEST_QUEUE_ID
    connection.push_test_case = Mock(return_value=TEST_HANG_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_HANG_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.HANG
    assert seed_msg.parent_ids == [TEST_QUEUE_ID]
    connection.push_test_case = Mock(return_value=TEST_CRASH_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_CRASH_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.CRASH
    assert seed_msg.parent_ids == [TEST_QUEUE_ID]
    barrier.wait()

    watcher.stop()


def _angora_thread(fuzzer_path, barrier):
    # This is also the order of creation Angora follows
    crashes_path = fuzzer_path / "crashes"
    hangs_path = fuzzer_path / "hangs"
    queue_path = fuzzer_path / "queue"

    crashes_path.mkdir(parents=True)
    hangs_path.mkdir(parents=True)
    queue_path.mkdir(parents=True)

    barrier.wait()  # Wait for watcher to start

    with open(queue_path / "id:000000", "wb") as test_case_file:
        test_case_file.write(TEST_QUEUE_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    with open(hangs_path / "id:000000", "wb") as test_case_file:
        test_case_file.write(TEST_HANG_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    with open(crashes_path / "id:000000", "wb") as test_case_file:
        test_case_file.write(TEST_CRASH_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()


def test_angora_watcher(caplog, tmp_path):
    caplog.set_level(logging.DEBUG)

    config = Config(
        fuzzer_type=FuzzerType.ANGORA,
        output_dir=tmp_path,
        docker_enabled=False,  # unused
        afl_path=None,  # unused
        target_cmdline=[],  # unused
        ctrl_uri="",  # unused
        pull_uri="",  # unused
        push_uri="",  # unused
    )

    connection = Mock()

    watcher = AngoraWatcher(config, connection)

    # Angora wants to create its own output folder, so ensure it is not created
    fuzzer_path = tmp_path / str(config.fuzzer_type)
    assert not fuzzer_path.is_dir()

    barrier = Barrier(2, timeout=2)
    fuzzer_thread = Thread(
        target=_angora_thread, args=(fuzzer_path, barrier), daemon=True
    )
    fuzzer_thread.start()

    watcher.start()

    connection.push_test_case = Mock(return_value=TEST_QUEUE_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_QUEUE_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.NORMAL
    connection.push_test_case = Mock(return_value=TEST_HANG_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_HANG_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.HANG
    connection.push_test_case = Mock(return_value=TEST_CRASH_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_CRASH_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.CRASH
    barrier.wait()

    watcher.stop()


def _qsym_thread(fuzzer_path, barrier):
    # This is also the order of creation QSYM follows
    queue_path = fuzzer_path / "queue"
    hangs_path = fuzzer_path / "hangs"
    crashes_path = fuzzer_path / "errors"

    barrier.wait()  # Wait for watcher to start

    with open(queue_path / "id:000000,src:id:000042", "wb") as test_case_file:
        test_case_file.write(TEST_QUEUE_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    with open(hangs_path / "id:000002", "wb") as test_case_file:
        test_case_file.write(TEST_HANG_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    with open(crashes_path / "id:000007", "wb") as test_case_file:
        test_case_file.write(TEST_CRASH_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()


def test_qsym_watcher(caplog, tmp_path):
    caplog.set_level(logging.DEBUG)

    config = Config(
        fuzzer_type=FuzzerType.QSYM,
        output_dir=tmp_path,
        docker_enabled=False,  # unused
        afl_path=None,  # unused
        target_cmdline=[],  # unused
        ctrl_uri="",  # unused
        pull_uri="",  # unused
        push_uri="",  # unused
    )

    connection = Mock()
    id_dicts = IDDicts({}, {42: TEST_SYNC_ID})

    watcher = QSYMWatcher(config, connection, id_dicts)

    # All folders get reused by QSYM if found, so the driver should create them
    fuzzer_path = tmp_path / str(config.fuzzer_type)
    assert fuzzer_path.is_dir()

    queue_path = fuzzer_path / "queue"
    hangs_path = fuzzer_path / "hangs"
    crashes_path = fuzzer_path / "errors"
    assert queue_path.is_dir()
    assert hangs_path.is_dir()
    assert crashes_path.is_dir()

    barrier = Barrier(2, timeout=2)
    fuzzer_thread = Thread(
        target=_qsym_thread, args=(fuzzer_path, barrier), daemon=True
    )
    fuzzer_thread.start()

    watcher.start(daemon=True)

    connection.push_test_case = Mock(return_value=TEST_QUEUE_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_QUEUE_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.NORMAL
    assert seed_msg.parent_ids == [TEST_SYNC_ID]
    connection.push_test_case = Mock(return_value=TEST_HANG_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_HANG_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.HANG
    # QSYM keeps parent information only for queue test cases
    assert seed_msg.parent_ids == []
    connection.push_test_case = Mock(return_value=TEST_CRASH_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_CRASH_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.CRASH
    # QSYM keeps parent information only for queue test cases
    assert seed_msg.parent_ids == []
    barrier.wait()

    watcher.stop()


# XXX: This can all be done in the same thread, since the directories are
# created by who starts the driver.
def _libfuzzer_thread(fuzzer_path, barrier):
    queue_path = fuzzer_path / "queue"
    artifacts_path = fuzzer_path / "artifacts"

    barrier.wait()  # Wait for watcher to start

    with open(queue_path / "123456789abcdef", "wb") as test_case_file:
        test_case_file.write(TEST_QUEUE_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    with open(queue_path / f"framework-{TEST_SYNC_ID}", "wb") as test_case_file:
        test_case_file.write(TEST_QUEUE_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    with open(artifacts_path / "timeout-123456789abcdef", "wb") as test_case_file:
        test_case_file.write(TEST_HANG_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    with open(artifacts_path / "crash-123456789abcdef", "wb") as test_case_file:
        test_case_file.write(TEST_CRASH_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()


def test_libfuzzer_watcher(caplog, tmp_path):
    caplog.set_level(logging.DEBUG)

    # LibFuzzer expects that queue and artifacts folders are created before it
    # is started, this should be done by who starts the driver.
    queue_path = tmp_path / "queue"
    queue_path.mkdir()
    artifacts_path = tmp_path / "artifacts"
    artifacts_path.mkdir()

    # The seeds are placed, with the appropriate prefix, in the queue folder
    # before starting the driver.
    with open(queue_path / "seed-123456789abcdef", "wb") as test_case_file:
        test_case_file.write(TEST_SEED_CONTENT)

    config = Config(
        fuzzer_type=FuzzerType.LIBFUZZER,  # unused
        output_dir=tmp_path,
        docker_enabled=False,  # unused
        afl_path=None,  # unused
        target_cmdline=[],  # unused
        ctrl_uri="",  # unused
        pull_uri="",  # unused
        push_uri="",  # unused
    )

    connection = Mock()

    watcher = LibFuzzerWatcher(config, connection)

    barrier = Barrier(2, timeout=2)
    fuzzer_thread = Thread(
        target=_libfuzzer_thread, args=(tmp_path, barrier), daemon=True
    )
    fuzzer_thread.start()

    watcher.start(daemon=True)

    # The seed should be detected and reported immediately
    sleep(0.2)
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_SEED_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.NORMAL
    connection.push_test_case = Mock(return_value=TEST_QUEUE_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_QUEUE_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.NORMAL
    connection.push_test_case.reset_mock()
    barrier.wait()

    barrier.wait()
    # Check that "framework-xxx" seeds are not reported
    connection.push_test_case.assert_not_called()
    connection.push_test_case = Mock(return_value=TEST_HANG_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_HANG_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.HANG
    connection.push_test_case = Mock(return_value=TEST_CRASH_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_CRASH_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.CRASH
    barrier.wait()

    watcher.stop()


# XXX: This can all be done in the same thread, since the directories are
# created by who starts the driver or by the driver itself.
def _honggfuzz_thread(fuzzer_path, barrier):
    queue_path = fuzzer_path / "queue"

    crashes_path = fuzzer_path / "crashes"
    crashes_path.mkdir(exist_ok=True)  # Only crashes is created by the fuzzer

    barrier.wait()  # Wait for watcher to start

    with open(
        queue_path / "00036000000000000360000000000000.00000002.honggfuzz.cov", "wb"
    ) as test_case_file:
        test_case_file.write(TEST_QUEUE_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()

    crash_filename = (
        "SIGABRT.PC.7ffff7c80625.STACK.2b9ffa4b0.CODE.-6."
        "ADDR.0.INSTR.mov____0x108(%rsp),%rax.fuzz"
    )
    with open(crashes_path / crash_filename, "wb",) as test_case_file:
        test_case_file.write(TEST_CRASH_CONTENT)
    sleep(0.2)
    barrier.wait()  # Trigger assert
    barrier.wait()


def test_honggfuzz_watcher(caplog, tmp_path):
    caplog.set_level(logging.DEBUG)

    queue_path = tmp_path / "queue"

    config = Config(
        fuzzer_type=FuzzerType.HONGGFUZZ,  # unused
        output_dir=tmp_path,
        docker_enabled=False,  # unused
        afl_path=None,  # unused
        target_cmdline=[],  # unused
        ctrl_uri="",  # unused
        pull_uri="",  # unused
        push_uri="",  # unused
    )

    connection = Mock()

    watcher = HonggFuzzWatcher(config, connection)

    # The driver should create the output folder if it does not already find one
    assert queue_path.is_dir()

    barrier = Barrier(2, timeout=2)
    fuzzer_thread = Thread(
        target=_honggfuzz_thread, args=(tmp_path, barrier), daemon=True
    )
    fuzzer_thread.start()

    watcher.start(daemon=True)

    connection.push_test_case = Mock(return_value=TEST_QUEUE_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_QUEUE_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.NORMAL
    connection.push_test_case = Mock(return_value=TEST_QUEUE_ID)
    barrier.wait()

    barrier.wait()
    connection.push_test_case.assert_called_once()
    args, _ = connection.push_test_case.call_args
    seed_msg = args[0]
    assert seed_msg.content == TEST_CRASH_CONTENT
    assert seed_msg.type == SeedMsg.SeedType.CRASH
    barrier.wait()

    watcher.stop()
