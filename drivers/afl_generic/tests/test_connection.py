from collabfuzz_generic_driver.connection import FrameworkConnection
from collabfuzz_generic_driver.config import Config, FuzzerType
from collabfuzz_generic_driver.fuzzerctrlmsg_pb2 import FuzzerCtrlMsg, CtrlCommand
from collabfuzz_generic_driver.seedmsg_pb2 import SeedMsg, JobMsg
from collabfuzz_generic_driver.seedmsg_pb2 import (
    TestCaseReportReply as LocalTestCaseReportReply,
)

from pathlib import Path
import zmq
from threading import Thread
import logging
from contextlib import closing

POLL_TIMEOUT_MILLISECONDS = 1000

TEST_FUZZER_TYPE = FuzzerType.AFL
TEST_FUZZER_ID = "42"
TEST_SEED_ID = "test_seed_id"
TEST_SERVER_SEED_ID = "test_server_seed_id"
TEST_CTRL_COMMAND = CtrlCommand.COMMAND_PAUSE


def _run_connection_test(caplog, tmp_path, server_thread, client_thread):
    ctrl_uri = "ipc://{}".format(tmp_path / "ctrl_socket.ipc")
    pull_uri = "ipc://{}".format(tmp_path / "pull_socket.ipc")
    push_uri = "ipc://{}".format(tmp_path / "push_socket.ipc")

    config = Config(
        fuzzer_type=TEST_FUZZER_TYPE,
        output_dir=Path(),  # unused
        docker_enabled=False,  # unused
        afl_path=None,  # unused
        target_cmdline=[],  # unused
        ctrl_uri=ctrl_uri,
        pull_uri=pull_uri,
        push_uri=push_uri,
    )

    caplog.set_level(logging.DEBUG)

    zmq_ctx = zmq.Context.instance()

    ctrl_sock = zmq_ctx.socket(zmq.REP)
    pull_sock = zmq_ctx.socket(zmq.PUB)
    push_sock = zmq_ctx.socket(zmq.REP)

    logging.info("Binding server sockets")
    ctrl_sock.bind(ctrl_uri)
    pull_sock.bind(pull_uri)
    push_sock.bind(push_uri)

    try:
        logging.info("Starting client thread")
        # The thread is marked as daemon so that, if it fails to join, it will
        # not keep the test process running.
        connection_thread = Thread(target=client_thread, args=[config], daemon=True)
        connection_thread.start()

        logging.info("Running server thread")
        server_thread(ctrl_sock, pull_sock, push_sock)

        logging.info("Joining client thread")
        connection_thread.join(2)
        assert not connection_thread.is_alive()

    finally:
        logging.info("Closing server sockets")
        ctrl_sock.close()
        pull_sock.close()
        push_sock.close()
        logging.info("Server sockets closed")


def _check_register_request(ctrl_sock):
    logging.info("Receiving registration request")
    n_events = ctrl_sock.poll(POLL_TIMEOUT_MILLISECONDS, zmq.POLLIN)
    assert n_events == 1

    msg_type, registration_request_bytes = ctrl_sock.recv_multipart()
    assert msg_type == b"C"
    registration_request = FuzzerCtrlMsg()
    registration_request.ParseFromString(registration_request_bytes)
    assert registration_request.command == CtrlCommand.COMMAND_REGISTER
    assert registration_request.fuzzer_type == TEST_FUZZER_TYPE.to_pb2_type()


def _send_register_reply(ctrl_sock):
    logging.info("Sending registration reply")
    registration_reply = FuzzerCtrlMsg()
    registration_reply.command = CtrlCommand.COMMAND_REGISTER
    registration_reply.fuzzer_id = TEST_FUZZER_ID
    registration_reply_bytes = registration_reply.SerializeToString()
    ctrl_sock.send_multipart([b"C", registration_reply_bytes])


def _check_deregister_request(ctrl_sock):
    logging.info("Receiving deregistration request")
    n_events = ctrl_sock.poll(POLL_TIMEOUT_MILLISECONDS, zmq.POLLIN)
    assert n_events == 1

    msg_type, registration_request_bytes = ctrl_sock.recv_multipart()
    assert msg_type == b"C"
    registration_request = FuzzerCtrlMsg()
    registration_request.ParseFromString(registration_request_bytes)
    assert registration_request.command == CtrlCommand.COMMAND_DEREGISTER
    assert registration_request.fuzzer_id == TEST_FUZZER_ID


def _check_ready_report(ctrl_sock):
    logging.info("Receiving ready report")
    n_events = ctrl_sock.poll(POLL_TIMEOUT_MILLISECONDS, zmq.POLLIN)
    assert n_events == 1

    msg_type, ready_report_bytes = ctrl_sock.recv_multipart()
    assert msg_type == b"C"
    registration_request = FuzzerCtrlMsg()
    registration_request.ParseFromString(ready_report_bytes)
    assert registration_request.command == CtrlCommand.COMMAND_READY
    assert registration_request.fuzzer_id == TEST_FUZZER_ID


def _send_ack_reply(ctrl_sock):
    logging.info("Sending ack reply")
    registration_reply = FuzzerCtrlMsg()
    registration_reply.command = CtrlCommand.COMMAND_ACK
    registration_reply_bytes = registration_reply.SerializeToString()
    ctrl_sock.send_multipart([b"C", registration_reply_bytes])


def _send_ctrl_msg(pull_sock):
    logging.info("Sending control message")
    ctrl_message = FuzzerCtrlMsg()
    ctrl_message.fuzzer_id = TEST_FUZZER_ID
    ctrl_message.command = TEST_CTRL_COMMAND
    ctrl_message_bytes = ctrl_message.SerializeToString()
    pull_sock.send_multipart([TEST_FUZZER_ID.encode(), b"C", ctrl_message_bytes])


def _send_job_msg(pull_sock):
    logging.info("Sending job message")

    seed_message = SeedMsg()
    seed_message.id = TEST_SEED_ID

    job_message = JobMsg()
    job_message.fuzzer_id = TEST_FUZZER_ID
    job_message.seeds.append(seed_message)
    job_message_bytes = job_message.SerializeToString()
    pull_sock.send_multipart([TEST_FUZZER_ID.encode(), b"S", job_message_bytes])


def _check_test_case(push_sock):
    logging.info("Receiving test case report")
    n_events = push_sock.poll(POLL_TIMEOUT_MILLISECONDS, zmq.POLLIN)
    assert n_events == 1

    msg_type, seed_msg_bytes = push_sock.recv_multipart()
    assert msg_type == b"S"
    seed_msg = SeedMsg()
    seed_msg.ParseFromString(seed_msg_bytes)
    assert seed_msg.id == TEST_SEED_ID
    assert seed_msg.fuzzer_id == TEST_FUZZER_ID


def _send_report_reply(push_sock):
    logging.info("Sending report reply")

    report_reply = LocalTestCaseReportReply()
    report_reply.id = TEST_SERVER_SEED_ID
    report_reply_bytes = report_reply.SerializeToString()
    push_sock.send(report_reply_bytes)


def test_registration(caplog, tmp_path):
    def client_thread(config):
        logging.info("Initiating client connection")
        connection = FrameworkConnection(config)
        assert connection._fuzzer_id is not None

        logging.info("Closing client connection")
        connection.close()
        logging.info("Connection closed")

    def server_thread(ctrl_sock, pull_sock, push_sock):
        _check_register_request(ctrl_sock)
        _send_register_reply(ctrl_sock)
        _check_deregister_request(ctrl_sock)
        _send_ack_reply(ctrl_sock)

    _run_connection_test(caplog, tmp_path, server_thread, client_thread)


def test_with_closing(caplog, tmp_path):
    def client_thread(config):
        logging.info("Initiating client connection")

        with closing(FrameworkConnection(config)) as connection:
            assert connection._fuzzer_id is not None
            logging.info("Closing client connection")

        logging.info("Connection closed")

    def server_thread(ctrl_sock, pull_sock, push_sock):
        _check_register_request(ctrl_sock)
        _send_register_reply(ctrl_sock)
        _check_deregister_request(ctrl_sock)
        _send_ack_reply(ctrl_sock)

    _run_connection_test(caplog, tmp_path, server_thread, client_thread)


def test_ready(caplog, tmp_path):
    def client_thread(config):
        logging.info("Initiating client connection")
        connection = FrameworkConnection(config)
        assert connection._fuzzer_id is not None

        logging.info("Reporting client ready")
        connection.report_ready()

        logging.info("Closing client connection")
        connection.close()
        logging.info("Connection closed")

    def server_thread(ctrl_sock, pull_sock, push_sock):
        _check_register_request(ctrl_sock)
        _send_register_reply(ctrl_sock)
        _check_ready_report(ctrl_sock)
        _send_ack_reply(ctrl_sock)
        _check_deregister_request(ctrl_sock)
        _send_ack_reply(ctrl_sock)

    _run_connection_test(caplog, tmp_path, server_thread, client_thread)


def test_pull_ctrl_msg(caplog, tmp_path):
    def client_thread(config):
        logging.info("Initiating client connection")
        connection = FrameworkConnection(config)
        assert connection._fuzzer_id is not None

        logging.info("Reporting client ready")
        connection.report_ready()

        logging.info("Pulling message from server")
        ctrl_msg = connection.pull_from_server()
        assert type(ctrl_msg) == FuzzerCtrlMsg
        assert ctrl_msg.fuzzer_id == TEST_FUZZER_ID
        assert ctrl_msg.command == TEST_CTRL_COMMAND

        logging.info("Closing client connection")
        connection.close()
        logging.info("Connection closed")

    def server_thread(ctrl_sock, pull_sock, push_sock):
        _check_register_request(ctrl_sock)
        _send_register_reply(ctrl_sock)
        _check_ready_report(ctrl_sock)
        _send_ack_reply(ctrl_sock)
        _send_ctrl_msg(pull_sock)
        _check_deregister_request(ctrl_sock)
        _send_ack_reply(ctrl_sock)

    _run_connection_test(caplog, tmp_path, server_thread, client_thread)


def test_pull_job_msg(caplog, tmp_path):
    def client_thread(config):
        logging.info("Initiating client connection")
        connection = FrameworkConnection(config)
        assert connection._fuzzer_id is not None

        logging.info("Reporting client ready")
        connection.report_ready()

        logging.info("Pulling message from server")
        job_msg = connection.pull_from_server()
        assert type(job_msg) == JobMsg
        assert job_msg.fuzzer_id == TEST_FUZZER_ID

        seed_msg = job_msg.seeds[0]
        assert seed_msg.id == TEST_SEED_ID

        logging.info("Closing client connection")
        connection.close()
        logging.info("Connection closed")

    def server_thread(ctrl_sock, pull_sock, push_sock):
        _check_register_request(ctrl_sock)
        _send_register_reply(ctrl_sock)
        _check_ready_report(ctrl_sock)
        _send_ack_reply(ctrl_sock)
        _send_job_msg(pull_sock)
        _check_deregister_request(ctrl_sock)
        _send_ack_reply(ctrl_sock)

    _run_connection_test(caplog, tmp_path, server_thread, client_thread)


def test_push_test_case(caplog, tmp_path):
    def client_thread(config):
        logging.info("Initiating client connection")
        connection = FrameworkConnection(config)
        assert connection._fuzzer_id is not None

        seed_msg = SeedMsg()
        seed_msg.id = TEST_SEED_ID
        server_test_id = connection.push_test_case(seed_msg)
        assert server_test_id == TEST_SERVER_SEED_ID

        logging.info("Closing client connection")
        connection.close()
        logging.info("Connection closed")

    def server_thread(ctrl_sock, pull_sock, push_sock):
        _check_register_request(ctrl_sock)
        _send_register_reply(ctrl_sock)
        _check_test_case(push_sock)
        _send_report_reply(push_sock)
        _check_deregister_request(ctrl_sock)
        _send_ack_reply(ctrl_sock)

    _run_connection_test(caplog, tmp_path, server_thread, client_thread)
