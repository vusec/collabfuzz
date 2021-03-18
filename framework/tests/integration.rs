use collab_fuzz;
use collab_fuzz::protos::{
    CtrlCommand, FuzzerCtrlMsg, JobMsg, SeedMsg, SeedMsg_SeedType, TestCaseReportReply,
};

use protobuf::Message;

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const FROM_DRIVERS_URI: &str = "ipc:///tmp/collab_fuzz_from_drivers";
const TO_DRIVERS_URI: &str = "ipc:///tmp/collab_fuzz_to_drivers";
const CONTROL_URI: &str = "ipc:///tmp/collab_fuzz_control";
const INPUT_DIR_NAME: &str = "collab-fuzz-input-test";
const OUTPUT_DIR_NAME: &str = "collab-fuzz-output-test";

fn setup_environment(
    scheduler_type: collab_fuzz::SchedulerType,
    test_name: &str,
) -> collab_fuzz::Config {
    let _ = env_logger::builder().is_test(true).try_init();

    let tmp_dir = env::temp_dir();
    let input_dir = tmp_dir.join(format!("{}-{}", INPUT_DIR_NAME, test_name));
    let output_dir = tmp_dir.join(format!("{}-{}", OUTPUT_DIR_NAME, test_name));

    // Cleanup environment if previous test failed
    teardown_environment(&input_dir, &output_dir).ok();

    fs::create_dir(&input_dir).expect("Could not create test input directory");

    collab_fuzz::Config {
        name: String::from("testing"),
        scheduler: scheduler_type,
        input_dir: input_dir.clone(),
        output_dir: output_dir.clone(),
        uri_listener: String::from(format!("{}-{}.ipc", FROM_DRIVERS_URI, test_name)),
        uri_scheduler: String::from(format!("{}-{}.ipc", TO_DRIVERS_URI, test_name)),
        uri_control: String::from(format!("{}-{}.ipc", CONTROL_URI, test_name)),
        uri_analysis: String::from("unused"),
        pass_config: collab_fuzz::PassConfig {
            program_arguments: vec![String::from("arg1"), String::from("arg2")],
            analysis_artifacts_dir: PathBuf::from("/tmp/collab_fuzz_test_binaries"),
            analysis_input_dir: env::temp_dir().join("collab_fuzz_analysis"),
        },
        refresh: Duration::from_secs(2),
    }
}

fn teardown_environment(input_dir: &PathBuf, output_dir: &PathBuf) -> io::Result<()> {
    fs::remove_dir_all(input_dir)?;
    fs::remove_dir_all(output_dir)?;
    Ok(())
}

#[test]
fn start_and_stop() {
    let config = setup_environment(collab_fuzz::SchedulerType::Test, "start_and_stop");

    let config_clone = config.clone();
    let (kill_tx, kill_rx) = mpsc::channel();
    let server_thread = thread::spawn(move || {
        collab_fuzz::start(&config_clone, kill_rx).expect("Could not start server");
    });

    // XXX: If the server is killed while not completely up, the threads will panic because the
    // context has already been destroyed. To avoid this issue, Box<dyn Error> should not be used.
    thread::sleep(Duration::from_secs(2));

    kill_tx.send(()).expect("Could not send kill message");

    server_thread.join().expect("Could not join server thread");

    teardown_environment(&config.input_dir, &config.output_dir).expect("Could not teardown");
}

#[test]
fn register_deregister_fuzzer() {
    let config = setup_environment(
        collab_fuzz::SchedulerType::Test,
        "register_deregister_fuzzer",
    );

    let config_clone = config.clone();
    let (kill_tx, kill_rx) = mpsc::channel();

    eprintln!("Starting server thread");
    let server_thread = thread::spawn(move || {
        collab_fuzz::start(&config_clone, kill_rx).expect("Could not start server");
    });

    // XXX: This creates a new context instance as compared to the one in the server, maybe it
    // should be passed in the configuration. It also prevents the use of inproc sockets.
    let context = zmq::Context::new();

    let control_sock = context
        .socket(zmq::REQ)
        .expect("Could not create control socket");
    control_sock
        .connect(&config.uri_control)
        .expect("Could not connect control socket");

    let mut register_req = FuzzerCtrlMsg::new();
    register_req.set_command(CtrlCommand::COMMAND_REGISTER);
    let ctrl_msg_bytes = register_req
        .write_to_bytes()
        .expect("Could not serialize register message");
    eprintln!("Sending register request");
    control_sock
        .send_multipart(&[b"C".to_vec(), ctrl_msg_bytes], 0)
        .expect("Could not send register message");

    eprintln!("Waiting for register reply");
    let register_reply_parts = control_sock
        .recv_multipart(0)
        .expect("Could not receive reply");
    assert_eq!(register_reply_parts[0], b"C");

    let register_reply: FuzzerCtrlMsg =
        protobuf::parse_from_bytes(&register_reply_parts[1]).expect("Could not deserialize reply");
    let reply_type = register_reply.get_command();
    assert_eq!(reply_type, CtrlCommand::COMMAND_REGISTER);

    let mut deregister_req = FuzzerCtrlMsg::new();
    deregister_req.set_command(CtrlCommand::COMMAND_DEREGISTER);
    deregister_req.set_fuzzer_id(register_reply.get_fuzzer_id().into());
    let ctrl_msg_bytes = deregister_req
        .write_to_bytes()
        .expect("Could not serialize deregister message");
    eprintln!("Sending deregister request");
    control_sock
        .send_multipart(&[b"C".to_vec(), ctrl_msg_bytes], 0)
        .expect("Could not send deregister message");

    eprintln!("Waiting for ack reply");
    let ack_reply_parts = control_sock
        .recv_multipart(0)
        .expect("Could not receive reply");
    assert_eq!(ack_reply_parts[0], b"C");

    let ack_reply: FuzzerCtrlMsg =
        protobuf::parse_from_bytes(&ack_reply_parts[1]).expect("Could not deserialize reply");
    let reply_type = ack_reply.get_command();
    assert_eq!(reply_type, CtrlCommand::COMMAND_ACK);

    eprintln!("Killing server");
    kill_tx.send(()).expect("Could not send kill message");

    server_thread.join().expect("Could not join server thread");

    teardown_environment(&config.input_dir, &config.output_dir).expect("Could not teardown");
}

#[test]
fn full_cycle() {
    let config = setup_environment(collab_fuzz::SchedulerType::Test, "full_cycle");

    let config_clone = config.clone();
    let (kill_tx, kill_rx) = mpsc::channel();

    eprintln!("Starting server thread");
    let server_thread = thread::spawn(move || {
        collab_fuzz::start(&config_clone, kill_rx).expect("Could not start server");
    });

    // XXX: This creates a new context instance as compared to the one in the server, maybe it
    // should be passed in the configuration. It also prevents the use of inproc sockets.
    let context = zmq::Context::new();

    // Connect control socket
    let control_sock = context
        .socket(zmq::REQ)
        .expect("Could not create control socket");
    control_sock
        .connect(&config.uri_control)
        .expect("Could not connect control socket");

    // Register fake driver
    let mut register_req = FuzzerCtrlMsg::new();
    register_req.set_command(CtrlCommand::COMMAND_REGISTER);
    let ctrl_msg_bytes = register_req
        .write_to_bytes()
        .expect("Could not serialize register message");
    eprintln!("Sending register request");
    control_sock
        .send_multipart(&[b"C".to_vec(), ctrl_msg_bytes], 0)
        .expect("Could not send register message");

    eprintln!("Waiting for register reply");
    let register_reply_parts = control_sock
        .recv_multipart(0)
        .expect("Could not receive reply");
    assert_eq!(register_reply_parts[0], b"C");

    let register_reply: FuzzerCtrlMsg =
        protobuf::parse_from_bytes(&register_reply_parts[1]).expect("Could not deserialize reply");
    let reply_type = register_reply.get_command();
    assert_eq!(reply_type, CtrlCommand::COMMAND_REGISTER);

    let driver_id = register_reply.get_fuzzer_id();
    eprintln!("Driver ID: {}", driver_id);

    let report_socket = context
        .socket(zmq::REQ)
        .expect("Could not create report socket");
    report_socket
        .connect(&config.uri_listener)
        .expect("Could not connect report socket");

    let scheduler_socket = context
        .socket(zmq::SUB)
        .expect("Could not create scheduler socket");
    scheduler_socket
        .set_subscribe(driver_id.as_bytes())
        .expect("Could not set subscription");
    scheduler_socket
        .connect(&config.uri_scheduler)
        .expect("Could not connect seed socket");

    // Report ready
    let mut ready_report = FuzzerCtrlMsg::new();
    ready_report.set_command(CtrlCommand::COMMAND_READY);
    ready_report.set_fuzzer_id(String::from(driver_id));
    let ready_report_bytes = ready_report
        .write_to_bytes()
        .expect("Could not serialize ready message");
    control_sock
        .send_multipart(&[b"C".to_vec(), ready_report_bytes], 0)
        .expect("Could not send ready message");

    eprintln!("Waiting for ready ack");
    let ready_ack_parts = control_sock
        .recv_multipart(0)
        .expect("Could not receive reply");
    assert_eq!(ready_ack_parts[0], b"C");

    let ready_ack: FuzzerCtrlMsg =
        protobuf::parse_from_bytes(&ready_ack_parts[1]).expect("Could not deserialize reply");
    assert_eq!(ready_ack.get_command(), CtrlCommand::COMMAND_ACK);

    // Report fake test case
    let mut test_case_report = SeedMsg::new();
    test_case_report.set_id(String::from("test_report"));
    test_case_report.set_content("test_content".as_bytes().to_vec());
    test_case_report.set_field_type(SeedMsg_SeedType::NORMAL);
    test_case_report.set_conditional(123);
    test_case_report.set_fuzzer_id(String::from(driver_id));

    let test_case_report_bytes = test_case_report
        .write_to_bytes()
        .expect("Could not serialize register message");

    report_socket
        .send_multipart(&[b"S".to_vec(), test_case_report_bytes], 0)
        .expect("Could not send test report message");

    eprintln!("Waiting for report reply");
    let reply_encoded = report_socket
        .recv_bytes(0)
        .expect("Could not receive report reply");
    let reply: TestCaseReportReply =
        protobuf::parse_from_bytes(&reply_encoded).expect("Could not deserialize report reply");
    assert!(reply.has_id());
    assert_eq!(
        reply.get_id(),
        "594a1b494545be568120d28c43b3319e41d7b8e51a8112ebbece7b3275591a9a"
    );

    // Receive fake test case
    eprintln!("Waiting for fake test case");
    let job_msg_parts = scheduler_socket
        .recv_multipart(0)
        .expect("Could not receive fake test case");
    assert_eq!(job_msg_parts[0], driver_id.as_bytes());
    assert_eq!(job_msg_parts[1], b"S");

    let job_msg: JobMsg = protobuf::parse_from_bytes(&job_msg_parts[2])
        .expect("Could not deserialize fake test case");
    assert_eq!(job_msg.get_fuzzer_id(), driver_id);

    let test_cases = job_msg.get_seeds();
    assert_eq!(test_cases.len(), 1);
    assert_eq!(test_cases[0].get_content(), "test_content".as_bytes());

    // Kill server
    eprintln!("Killing server");
    kill_tx.send(()).expect("Could not send kill message");

    server_thread.join().expect("Could not join server thread");

    teardown_environment(&config.input_dir, &config.output_dir).expect("Could not teardown");
}

#[test]
fn full_cycle_duplicates() {
    let config = setup_environment(collab_fuzz::SchedulerType::Test, "full_cycle_duplicates");

    let config_clone = config.clone();
    let (kill_tx, kill_rx) = mpsc::channel();

    eprintln!("Starting server thread");
    let server_thread = thread::spawn(move || {
        collab_fuzz::start(&config_clone, kill_rx).expect("Could not start server");
    });

    // XXX: This creates a new context instance as compared to the one in the server, maybe it
    // should be passed in the configuration. It also prevents the use of inproc sockets.
    let context = zmq::Context::new();

    // Connect control socket
    let control_sock = context
        .socket(zmq::REQ)
        .expect("Could not create control socket");
    control_sock
        .connect(&config.uri_control)
        .expect("Could not connect control socket");

    // Register fake driver
    let mut register_req = FuzzerCtrlMsg::new();
    register_req.set_command(CtrlCommand::COMMAND_REGISTER);
    let ctrl_msg_bytes = register_req
        .write_to_bytes()
        .expect("Could not serialize register message");
    eprintln!("Sending register request");
    control_sock
        .send_multipart(&[b"C".to_vec(), ctrl_msg_bytes], 0)
        .expect("Could not send register message");

    eprintln!("Waiting for register reply");
    let register_reply_parts = control_sock
        .recv_multipart(0)
        .expect("Could not receive reply");
    assert_eq!(register_reply_parts[0], b"C");

    let register_reply: FuzzerCtrlMsg =
        protobuf::parse_from_bytes(&register_reply_parts[1]).expect("Could not deserialize reply");
    let reply_type = register_reply.get_command();
    assert_eq!(reply_type, CtrlCommand::COMMAND_REGISTER);

    let driver_id1 = register_reply.get_fuzzer_id();
    eprintln!("Driver ID: {}", driver_id1);

    // Register fake driver
    let mut register_req = FuzzerCtrlMsg::new();
    register_req.set_command(CtrlCommand::COMMAND_REGISTER);
    let ctrl_msg_bytes = register_req
        .write_to_bytes()
        .expect("Could not serialize register message");
    eprintln!("Sending register request");
    control_sock
        .send_multipart(&[b"C".to_vec(), ctrl_msg_bytes], 0)
        .expect("Could not send register message");

    eprintln!("Waiting for register reply");
    let register_reply_parts = control_sock
        .recv_multipart(0)
        .expect("Could not receive reply");
    assert_eq!(register_reply_parts[0], b"C");

    let register_reply: FuzzerCtrlMsg =
        protobuf::parse_from_bytes(&register_reply_parts[1]).expect("Could not deserialize reply");
    let reply_type = register_reply.get_command();
    assert_eq!(reply_type, CtrlCommand::COMMAND_REGISTER);

    let driver_id2 = register_reply.get_fuzzer_id();
    eprintln!("Driver ID: {}", driver_id2);

    let report_socket = context
        .socket(zmq::REQ)
        .expect("Could not create report socket");
    report_socket
        .connect(&config.uri_listener)
        .expect("Could not connect report socket");

    let scheduler_socket = context
        .socket(zmq::SUB)
        .expect("Could not create scheduler socket");
    scheduler_socket
        .set_subscribe(driver_id1.as_bytes())
        .expect("Could not set subscription");
    scheduler_socket
        .set_subscribe(driver_id2.as_bytes())
        .expect("Could not set subscription");
    scheduler_socket
        .connect(&config.uri_scheduler)
        .expect("Could not connect seed socket");

    // Report ready
    let mut ready_report = FuzzerCtrlMsg::new();
    ready_report.set_command(CtrlCommand::COMMAND_READY);
    ready_report.set_fuzzer_id(String::from(driver_id1));
    let ready_report_bytes = ready_report
        .write_to_bytes()
        .expect("Could not serialize ready message");
    control_sock
        .send_multipart(&[b"C".to_vec(), ready_report_bytes], 0)
        .expect("Could not send ready message");

    eprintln!("Waiting for ready ack");
    let ready_ack_parts = control_sock
        .recv_multipart(0)
        .expect("Could not receive reply");
    assert_eq!(ready_ack_parts[0], b"C");

    let ready_ack: FuzzerCtrlMsg =
        protobuf::parse_from_bytes(&ready_ack_parts[1]).expect("Could not deserialize reply");
    assert_eq!(ready_ack.get_command(), CtrlCommand::COMMAND_ACK);

    // Report ready
    let mut ready_report = FuzzerCtrlMsg::new();
    ready_report.set_command(CtrlCommand::COMMAND_READY);
    ready_report.set_fuzzer_id(String::from(driver_id2));
    let ready_report_bytes = ready_report
        .write_to_bytes()
        .expect("Could not serialize ready message");
    control_sock
        .send_multipart(&[b"C".to_vec(), ready_report_bytes], 0)
        .expect("Could not send ready message");

    eprintln!("Waiting for ready ack");
    let ready_ack_parts = control_sock
        .recv_multipart(0)
        .expect("Could not receive reply");
    assert_eq!(ready_ack_parts[0], b"C");

    let ready_ack: FuzzerCtrlMsg =
        protobuf::parse_from_bytes(&ready_ack_parts[1]).expect("Could not deserialize reply");
    assert_eq!(ready_ack.get_command(), CtrlCommand::COMMAND_ACK);

    // Report fake test case
    let mut test_case_report = SeedMsg::new();
    test_case_report.set_id(String::from("test_report"));
    test_case_report.set_content("test_content".as_bytes().to_vec());
    test_case_report.set_field_type(SeedMsg_SeedType::NORMAL);
    test_case_report.set_conditional(123);
    test_case_report.set_fuzzer_id(String::from(driver_id1));

    let test_case_report_bytes = test_case_report
        .write_to_bytes()
        .expect("Could not serialize register message");

    report_socket
        .send_multipart(&[b"S".to_vec(), test_case_report_bytes], 0)
        .expect("Could not send test report message");

    eprintln!("Waiting for report reply");
    let reply_encoded = report_socket
        .recv_bytes(0)
        .expect("Could not receive report reply");
    let reply: TestCaseReportReply =
        protobuf::parse_from_bytes(&reply_encoded).expect("Could not deserialize report reply");
    assert!(reply.has_id());
    assert_eq!(
        reply.get_id(),
        "594a1b494545be568120d28c43b3319e41d7b8e51a8112ebbece7b3275591a9a"
    );

    // Receive fake test case
    eprintln!("Waiting for fake test case");
    let job_msg_parts = scheduler_socket
        .recv_multipart(0)
        .expect("Could not receive fake test case");
    assert_eq!(job_msg_parts[0], driver_id1.as_bytes());
    assert_eq!(job_msg_parts[1], b"S");

    let job_msg: JobMsg = protobuf::parse_from_bytes(&job_msg_parts[2])
        .expect("Could not deserialize fake test case");
    assert_eq!(job_msg.get_fuzzer_id(), driver_id1);

    let test_cases = job_msg.get_seeds();
    assert_eq!(test_cases.len(), 1);
    assert_eq!(test_cases[0].get_content(), "test_content".as_bytes());

    // Report fake test case (duplicate)
    let mut test_case_report = SeedMsg::new();
    test_case_report.set_id(String::from("test_report"));
    test_case_report.set_content("test_content".as_bytes().to_vec());
    test_case_report.set_field_type(SeedMsg_SeedType::NORMAL);
    test_case_report.set_conditional(123);
    test_case_report.set_fuzzer_id(String::from(driver_id2));

    let test_case_report_bytes = test_case_report
        .write_to_bytes()
        .expect("Could not serialize register message");

    report_socket
        .send_multipart(&[b"S".to_vec(), test_case_report_bytes], 0)
        .expect("Could not send test report message");

    eprintln!("Waiting for report reply");
    let reply_encoded = report_socket
        .recv_bytes(0)
        .expect("Could not receive report reply");
    let reply: TestCaseReportReply =
        protobuf::parse_from_bytes(&reply_encoded).expect("Could not deserialize report reply");
    assert!(reply.has_id());
    assert_eq!(
        reply.get_id(),
        "594a1b494545be568120d28c43b3319e41d7b8e51a8112ebbece7b3275591a9a"
    );

    // Receive fake test case (duplicate)
    eprintln!("Waiting for fake test case");
    let job_msg_parts = scheduler_socket
        .recv_multipart(0)
        .expect("Could not receive fake test case");
    assert_eq!(job_msg_parts[0], driver_id2.as_bytes());
    assert_eq!(job_msg_parts[1], b"S");

    let job_msg: JobMsg = protobuf::parse_from_bytes(&job_msg_parts[2])
        .expect("Could not deserialize fake test case");
    assert_eq!(job_msg.get_fuzzer_id(), driver_id2);

    let test_cases = job_msg.get_seeds();
    assert_eq!(test_cases.len(), 1);
    assert_eq!(test_cases[0].get_content(), "test_content".as_bytes());

    // Kill server
    eprintln!("Killing server");
    kill_tx.send(()).expect("Could not send kill message");

    server_thread.join().expect("Could not join server thread");

    teardown_environment(&config.input_dir, &config.output_dir).expect("Could not teardown");
}
