use crate::analysis::{AnalysisType, PassType};
use crate::config::Config;
use crate::fuzzers::FuzzersHandler;
use crate::logger::Logger;
use crate::reactor::{Reactor, ReactorSharedObjects, Worker};
use crate::scheduler::{SchedulerFacadeRef, SchedulerHandler, SchedulerHandlerControlMessage};
use crate::storage::Storage;
use crate::types::{
    SResult, SharedFuzzersHandler, SharedGlobalStates, SharedLogger, SharedStorage,
};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::iter::FromIterator;
use std::process::exit;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

fn start_scheduler(
    config: &Config,
    ctx: zmq::Context,
    global_states: SharedGlobalStates,
    fuzzers_handler: SharedFuzzersHandler,
    storage: SharedStorage,
    thread_control: mpsc::Receiver<SchedulerHandlerControlMessage>,
    logger: SharedLogger,
) -> SResult {
    let facade = SchedulerFacadeRef::new(
        ctx,
        config.uri_scheduler.clone(),
        global_states,
        fuzzers_handler,
        storage,
        logger,
    )?;
    let mut handler =
        SchedulerHandler::new(config.scheduler, facade, thread_control, config.refresh);
    handler.run();

    Ok(())
}

// TODO: This function probably needs to be turned into a Server class, which can be constructed
// first and then started, so that it is easier to test correctly.
pub fn start(config: &Config, kill_rx: mpsc::Receiver<()>) -> SResult {
    log::debug!("Beginning server setup");

    log::debug!("Setting up storage");
    let storage = Arc::new(Mutex::new(Storage::new(config.output_dir.clone())?));

    // log::debug!("Reading corpus");
    // let corpus = utils::get_input_seeds(&config.input_dir)?;
    // TODO: The corpus needs to go through the AnalysisHandler to be processed correctly and
    // become visible to the scheduler as new seeds. Unless you register a fake driver for the
    // seeds, the best way to do this is read them here and then move them in the AnalysisHandler
    // thread before processing them.

    log::debug!("Setting up global states");
    let global_states: SharedGlobalStates = Arc::new(Mutex::new(HashMap::new()));

    log::debug!("Setting up fuzzers handler");
    let fuzzers_handler: SharedFuzzersHandler = Arc::new(Mutex::new(FuzzersHandler::new()));

    log::debug!("Setting up shared logger");
    let logger = Arc::new(Mutex::new(Logger::new(&config.output_dir)?));

    // TODO: The context can probably be moved inside the configuration object, the handling of
    // its lifetime would need to be rethought though. Maybe move the config instead of taking a
    // reference?
    let mut ctx = zmq::Context::new();

    let analysis_requirements: HashSet<AnalysisType> =
        HashSet::from_iter(config.scheduler.get_requirements());
    let mut pass_to_duplicates: HashMap<PassType, bool> = HashMap::new();

    log::info!("Required analyses: {:?}", analysis_requirements);

    log::debug!("Filling global states");
    {
        let mut global_states = global_states.lock().unwrap();
        for analysis_type in analysis_requirements {
            let logger = Arc::clone(&logger);
            let analysis_state = analysis_type.get_analysis_state(&config.pass_config, logger);

            if let Some(required_passes) = analysis_state.get_required_passes() {
                for required_pass in &required_passes {
                    if let Some(needs_duplicates) = pass_to_duplicates.get_mut(required_pass) {
                        if !*needs_duplicates && analysis_type.needs_duplicates() {
                            *needs_duplicates = analysis_type.needs_duplicates();
                        }
                    } else {
                        pass_to_duplicates.insert(*required_pass, analysis_type.needs_duplicates());
                    }
                }
                log::debug!("{:?} requires: {:?}", analysis_type, required_passes);
            }

            global_states.insert(analysis_type, analysis_state);
        }
    }

    log::info!("Required passes: {:?}", pass_to_duplicates.keys());

    log::debug!("Starting worker threads");
    fs::create_dir_all(&config.pass_config.analysis_input_dir)?;
    let mut worker_threads = Vec::new();
    for pass_type in pass_to_duplicates.keys() {
        let ctx = zmq::Context::clone(&ctx);
        let pass_config = config.pass_config.clone();
        let worker = match pass_type.get_pass(pass_config) {
            Ok(pass) => Worker::new(ctx, pass).unwrap(),
            Err(e) => {
                log::error!("Failed to initialize pass {}: {}", pass_type, e);
                exit(1);
            }
        };
        worker_threads.push(
            thread::Builder::new()
                .name("worker".into())
                .spawn(move || worker.run())
                .expect("Could not spawn worker thread"),
        );
    }

    let (sched_thread_ctrl_tx, sched_thread_ctrl_rx) = mpsc::channel();
    log::debug!("Setting up ZMQ scheduler");
    // 3) Start scheduler
    let scheduler_thread = {
        let ctx = zmq::Context::clone(&ctx);
        let global_states = Arc::clone(&global_states);
        let storage = Arc::clone(&storage);
        let config = config.clone();
        let fuzzers_handler = Arc::clone(&fuzzers_handler);
        let logger = Arc::clone(&logger);

        let builder = thread::Builder::new().name("scheduler".into());
        builder
            .spawn(move || {
                start_scheduler(
                    &config,
                    ctx,
                    global_states,
                    fuzzers_handler,
                    storage,
                    sched_thread_ctrl_rx,
                    logger,
                )
                .expect("Could not start scheduler thread");
            })
            .expect("Could not spawn scheduler thread")
    };

    log::debug!("Setting up reactor");
    let reactor_thread = {
        let config = config.clone();
        let ctx = zmq::Context::clone(&ctx);

        let reactor_shared_objs = ReactorSharedObjects {
            storage: Arc::clone(&storage),
            global_states: Arc::clone(&global_states),
            fuzzers_handler,
            logger: Arc::clone(&logger),
        };

        let sched_thread_ctrl_tx = sched_thread_ctrl_tx.clone();

        let builder = thread::Builder::new().name("reactor".into());
        builder
            .spawn(move || {
                let mut reactor = Reactor::new(
                    ctx,
                    &config.uri_listener,
                    &config.uri_control,
                    reactor_shared_objs,
                    sched_thread_ctrl_tx,
                );

                log::debug!("Registering passes");
                for (pass_type, run_on_duplicates) in pass_to_duplicates {
                    reactor
                        .register_pass_type(pass_type, run_on_duplicates)
                        .unwrap();
                }

                reactor.listen().unwrap();
            })
            .expect("Could not start reactor thread")
    };

    // TODO: Run UI thread here, on this thread.
    kill_rx.recv().unwrap();

    // Kill scheduler thread
    sched_thread_ctrl_tx
        .send(SchedulerHandlerControlMessage::Die)
        .unwrap();

    // When Ctrl-C is received, destroy the context and join all threads.
    log::debug!("Destroying ZMQ context");
    ctx.destroy().expect("Could not kill context");

    scheduler_thread
        .join()
        .expect("Could not join scheduler thread");
    log::debug!("Scheduler thread joined");

    reactor_thread
        .join()
        .expect("Cannot join AnalysisListener thread");
    log::debug!("Analysis thread joined");

    for thread in worker_threads {
        thread.join().expect("Could not join worker thread");
    }
    log::debug!("Worker threads joined");

    let mut logger = logger.lock().unwrap();
    if let Err(e) = logger.dump() {
        log::error!("{}", e);
    }

    Ok(())
}
