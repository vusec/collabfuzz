use crate::analysis::Pass;

pub struct Worker {
    distribute_socket: zmq::Socket,
    report_socket: zmq::Socket,
    pass: Box<dyn Pass>,
}

impl Worker {
    pub fn new(ctx: zmq::Context, pass: Box<dyn Pass>) -> Result<Worker, zmq::Error> {
        let distribute_socket = ctx.socket(zmq::PULL)?;
        distribute_socket.connect(&format!("inproc://{}-distribute", pass.pass_type()))?;

        let report_socket = ctx.socket(zmq::PUSH)?;
        report_socket.connect("inproc://workers-report")?;

        Ok(Worker {
            distribute_socket,
            report_socket,
            pass,
        })
    }

    pub fn run(&self) {
        let pass_name = self.pass.pass_type().to_string();

        loop {
            match self.distribute_socket.recv_multipart(0) {
                Ok(parts) => {
                    let test_id_part = &parts[0];
                    let test_case_part = &parts[1];

                    let mut test_id_bytes = [0; 8];
                    test_id_bytes.copy_from_slice(&test_id_part);
                    let test_id = u64::from_le_bytes(test_id_bytes);
                    log::info!("Processing test case: {}", test_id);

                    let report = match self.pass.process(test_case_part) {
                        Ok(analysis_output) => analysis_output,
                        Err(e) => {
                            log::error!("Processing error: {}", e);
                            Vec::new() // Sending empty report on error
                        }
                    };

                    let parts = [test_id_part, pass_name.as_bytes(), &report];
                    if let Err(e) = self.report_socket.send_multipart(parts.iter(), 0) {
                        log::error!("ZMQ send error: {}", e);
                    }
                }
                Err(e) => {
                    if let zmq::Error::ETERM = e {
                        log::info!("Received ETERM, killing worker");
                        break;
                    } else {
                        log::error!("ZMQ receive error: {}", e);
                    }
                }
            }
        }
    }
}
