#![feature(c_variadic)]
#![allow(non_snake_case)] // crate name is not snake case

mod dfsan;
mod io_wrappers;
mod tainter;
mod tracer;

use dfsan::dfsan_label;
use std::env;
use std::path::PathBuf;
use std::process;
use std::sync::Once;
use tainter::{ByteRange, Tainter, TainterBuilder};
use tracer::{Tracer, TracerBuilder};

type IDType = u64;

const OUTPUT_FILE_VARNAME: &str = "TRACER_OUTPUT_FILE";
const INPUT_PATH_VARNAME: &str = "TRACER_INPUT_FILE";
const RANGE_START_VARNAME: &str = "TRACER_RANGE_START";
const RANGE_SIZE_VARNAME: &str = "TRACER_RANGE_SIZE";

static RUN_CONSTRUCTOR: Once = Once::new();
static RUN_DESTRUCTOR: Once = Once::new();

fn initialize_tainter() {
    let mut builder = TainterBuilder::new();

    if let Ok(input_path_string) = env::var(INPUT_PATH_VARNAME) {
        builder.taint_file(PathBuf::from(input_path_string));
    }

    if let Ok(range_start_string) = env::var(RANGE_START_VARNAME) {
        if let Ok(range_size_string) = env::var(RANGE_SIZE_VARNAME) {
            let range_start: usize = range_start_string.parse().unwrap_or_else(|e| {
                eprintln!("Invalid range start: {}", e);
                process::exit(1);
            });

            let range_size: usize = range_size_string.parse().unwrap_or_else(|e| {
                eprintln!("Invalid range size: {}", e);
                process::exit(1);
            });

            builder.taint_range(ByteRange::new(range_start, range_start + range_size));
        }
    }

    builder.build_global().unwrap_or_else(|e| {
        eprintln!("Error during tainter initialization: {}", e);
        process::exit(1);
    });
}

fn initialize_tracer() {
    let mut builder = TracerBuilder::new();

    if let Ok(output_path_string) = env::var(OUTPUT_FILE_VARNAME) {
        builder.output_file(PathBuf::from(output_path_string));
    }

    builder.build_global().unwrap_or_else(|e| {
        eprintln!("Error during tainter initialization: {}", e);
        process::exit(1);
    });
}

#[no_mangle]
pub extern "C" fn __bb_taint_tracer_create() {
    RUN_CONSTRUCTOR.call_once(|| {
        env_logger::init();
        initialize_tainter();
        initialize_tracer();
    });
}

#[no_mangle]
pub extern "C" fn __bb_taint_tracer_destroy() {
    RUN_DESTRUCTOR.call_once(|| {
        if Tainter::global().is_none() || Tracer::global().is_none() {
            log::debug!("Initialization failed, skipping destructor");
            return;
        }

        let tainter = Tainter::global().unwrap();
        let tracer = Tracer::global().unwrap();

        if !tainter.is_enabled() || !tracer.is_enabled() {
            log::info!("Instrumentation disabled, skipping output");
            return;
        }

        tracer.write_data().unwrap_or_else(|e| {
            eprintln!("Could not write to output file: {}", e);
            process::exit(1);
        });
    });
}

#[no_mangle]
pub extern "C" fn __dfsw___bb_taint_tracer_trace(
    instruction_id: IDType,
    _traced_value: u64,
    _instruction_id_label: dfsan_label,
    traced_value_label: dfsan_label,
) {
    let mut tracer = if let Some(tracer) = Tracer::global() {
        tracer
    } else {
        // Try executing initialization now
        __bb_taint_tracer_create();
        log::warn!("Tracing function called before initialization");

        Tracer::global().unwrap()
    };

    // At this point, the tainter is necessarely initialized
    let tainter = Tainter::global().unwrap();

    tracer.trace_terminator_taint(
        instruction_id,
        traced_value_label,
        tainter.get_label_to_offsets_map(),
    );
}
