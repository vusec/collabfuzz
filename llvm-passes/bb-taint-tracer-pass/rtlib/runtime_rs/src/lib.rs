mod tracer;
use tracer::Tracer;

mod tainter;
use tainter::Tainter;

mod dfsan_interface;
use dfsan_interface::create_label;

use lazy_static::lazy_static;
use std::env;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;

type ShadowType = u16;
type IDType = u64;

const ENABLE_OUTPUT_VARNAME: &str = "TRACER_ENABLE_FILE_OUTPUT";
const ENABLE_OUTPUT_DEFAULT: bool = false;
const OUTPUT_FILE_VARNAME: &str = "TRACER_OUTPUT_FILE";
const OUTPUT_FILE_DEFAULT: &str = "trace_data.csv";
const INPUT_PATH_VARNAME: &str = "TRACER_INPUT_FILE";
const ENABLE_DEBUG_VARNAME: &str = "TRACER_DEBUG";
const ENABLE_DEBUG_DEFAULT: bool = false;

lazy_static! {
    static ref TRACER: Mutex<Option<Tracer>> = Mutex::new(None);
    static ref TAINTER: Mutex<Option<Tainter>> = Mutex::new(None);
}

#[no_mangle]
pub extern "C" fn __bb_taint_tracer_create() {
    let input_label = create_label("input_label").unwrap();

    let mut tracer_opt = TRACER.lock().unwrap();
    let mut tainter_opt = TAINTER.lock().unwrap();
    if tracer_opt.is_some() {
        // Be safe against being called multiple times
        assert!(tracer_opt.is_some() && tainter_opt.is_some());
        return;
    }
    assert!(tracer_opt.is_none() && tainter_opt.is_none());

    let output_path = if let Ok(path_string) = env::var(OUTPUT_FILE_VARNAME) {
        PathBuf::from(path_string)
    } else {
        OUTPUT_FILE_DEFAULT.into()
    };

    let enable_output = if let Ok(enable_output_string) = env::var(ENABLE_OUTPUT_VARNAME) {
        match bool::from_str(&enable_output_string.to_lowercase()) {
            Ok(enable_output) => enable_output,
            Err(error) => {
                eprintln!(
                    "Environment variable {} has invalid value: {}",
                    ENABLE_OUTPUT_VARNAME, error
                );
                ENABLE_OUTPUT_DEFAULT
            }
        }
    } else {
        ENABLE_OUTPUT_DEFAULT
    };

    let input_path_opt = if let Ok(input_path_string) = env::var(INPUT_PATH_VARNAME) {
        if let Ok(canonical_input_path) = PathBuf::from(&input_path_string).canonicalize() {
            Some(canonical_input_path)
        } else {
            eprintln!("Input file does not exist: {}", &input_path_string);
            None
        }
    } else {
        None
    };

    let enable_debug = if let Ok(enable_debug_string) = env::var(ENABLE_DEBUG_VARNAME) {
        match bool::from_str(&enable_debug_string.to_lowercase()) {
            Ok(enable_output) => enable_output,
            Err(error) => {
                eprintln!(
                    "Environment variable {} has invalid value: {}",
                    ENABLE_OUTPUT_VARNAME, error
                );
                ENABLE_OUTPUT_DEFAULT
            }
        }
    } else {
        ENABLE_DEBUG_DEFAULT
    };

    *tracer_opt = Some(Tracer::new(input_label, output_path, enable_output));
    *tainter_opt = Some(Tainter::new(input_label, input_path_opt, enable_debug));
}

#[no_mangle]
pub extern "C" fn __bb_taint_tracer_destroy() {
    let mut tracer_opt = TRACER.lock().unwrap();
    let mut tainter_opt = TAINTER.lock().unwrap();
    if tracer_opt.is_none() {
        // Be safe against being called multiple times
        assert!(tracer_opt.is_none() && tainter_opt.is_none());
        return;
    }
    assert!(tracer_opt.is_some() && tainter_opt.is_some());

    let tracer = tracer_opt.as_mut().unwrap();
    tracer.write_data().unwrap_or_else(|err| {
        eprintln!("Could not write to output file: {}", err);
    });

    *tracer_opt = None;
    *tainter_opt = None;
}

#[no_mangle]
pub extern "C" fn __dfsw___bb_taint_tracer_trace(
    basic_block_id: IDType,
    instruction_id: IDType,
    _traced_value: u64,
    _basic_block_id_shadow: ShadowType,
    _instruction_id_shadow: ShadowType,
    traced_value_shadow: ShadowType,
) {
    let mut tracer_opt = TRACER.lock().unwrap();
    if tracer_opt.is_none() {
        // Be safe against being called before initialization
        return;
    }

    let tracer = tracer_opt.as_mut().unwrap();
    tracer.trace_terminator_taint(basic_block_id, instruction_id, traced_value_shadow);
}

/// # Safety
///
/// This function assumes that `file_path` is a valid pointer
#[no_mangle]
pub unsafe extern "C" fn tainter_trace_open(fd: c_int, file_path: *const c_char) {
    if file_path.is_null() {
        return;
    }

    let mut tainter_opt = TAINTER.lock().unwrap();
    if tainter_opt.is_none() {
        // Be safe against being called before initialization
        return;
    }

    let tainter = tainter_opt.as_mut().unwrap();

    // We can only trust the calling program, no check can be performed
    let file_path_str = CStr::from_ptr(file_path);

    if let Ok(file_path) = file_path_str.to_str() {
        tainter.trace_open(fd, Path::new(file_path));
    }
}

#[no_mangle]
pub extern "C" fn tainter_trace_close(fd: c_int) {
    let mut tainter_opt = TAINTER.lock().unwrap();
    if tainter_opt.is_none() {
        // Be safe against being called before initialization
        return;
    }

    let tainter = tainter_opt.as_mut().unwrap();
    tainter.trace_close(fd);
}

#[no_mangle]
pub extern "C" fn tainter_is_input_fd(fd: c_int) -> bool {
    let tainter_opt = TAINTER.lock().unwrap();
    if tainter_opt.is_none() {
        // Be safe against being called before initialization
        return false;
    }

    let tainter = tainter_opt.as_ref().unwrap();
    tainter.is_input_fd(fd)
}

#[no_mangle]
pub extern "C" fn tainter_is_debug_enabled() -> bool {
    let tainter_opt = TAINTER.lock().unwrap();
    if tainter_opt.is_none() {
        // Be safe against being called before initialization
        return false;
    }

    let tainter = tainter_opt.as_ref().unwrap();
    tainter.is_debug_enabled()
}

#[no_mangle]
pub extern "C" fn tainter_get_input_label() -> ShadowType {
    let tainter_opt = TAINTER.lock().unwrap();
    if tainter_opt.is_none() {
        // Be safe against being called before initialization
        return 0;
    }

    let tainter = tainter_opt.as_ref().unwrap();
    tainter.get_input_label()
}
