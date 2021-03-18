use crate::dfsan::{dfsan_label, dfsan_set_label};
use crate::tainter::Tainter;
use libc::{c_char, c_int, c_void, off_t, size_t, ssize_t, FILE};
use std::convert::TryInto;
use std::ffi::CStr;

#[link(name = "c")]
extern "C" {
    // POSIX functions that are not present in libc crate
    fn getc_unlocked(stream: *mut FILE) -> c_int;
    fn getdelim(
        lineptr: *mut *mut c_char,
        n: *mut size_t,
        delim: c_int,
        stream: *mut FILE,
    ) -> ssize_t;

    // Non-POSIX function that is not preset in libc crate
    fn fgets_unlocked(buf: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char;
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_open(
    path: *const c_char,
    oflag: c_int,
    _path_label: dfsan_label,
    _oflag_label: dfsan_label,
    ret_label: *mut dfsan_label,
    mut arg: ...
) -> c_int {
    log::debug!("Wrapper called: {}", "open");
    *ret_label = 0;

    let mode = if open_needs_mode(oflag) { arg.arg() } else { 0 };
    let fd = libc::open(path, oflag, mode);
    if fd == -1 {
        // open failed
        return fd;
    }

    // We can only trust the calling program, no check can be performed
    let path_str = if let Ok(path_str) = CStr::from_ptr(path).to_str() {
        path_str
    } else {
        log::warn!("Could not convert path to UTF-8");
        return fd;
    };

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return fd;
    };

    tainter.trace_open(fd, path_str);

    fd
}

fn open_needs_mode(oflag: c_int) -> bool {
    // This definition is taken from "fcntl.h"
    oflag & libc::O_CREAT != 0 || oflag & libc::O_TMPFILE == libc::O_TMPFILE
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_fopen(
    filename: *const c_char,
    mode: *const c_char,
    _filename_label: dfsan_label,
    _mode_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> *mut FILE {
    log::debug!("Wrapper called: {}", "fopen");
    *ret_label = 0;

    let file = libc::fopen(filename, mode);
    if file.is_null() {
        // fopen failed
        return file;
    }

    // We can only trust the calling program, no check can be performed
    let filename_str = if let Ok(filename_str) = CStr::from_ptr(filename).to_str() {
        filename_str
    } else {
        log::warn!("Could not convert path to UTF-8");
        return file;
    };

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return file;
    };

    // fileno fails only if file is not a valid stream, checked before
    tainter.trace_open(libc::fileno(file), filename_str);

    file
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_fopen64(
    filename: *const c_char,
    mode: *const c_char,
    filename_label: dfsan_label,
    mode_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> *mut FILE {
    // On x86_64 it is a simple redirect
    log::debug!("Redirect from: {}", "fopen64");
    __dfsw_fopen(filename, mode, filename_label, mode_label, ret_label)
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_close(
    fd: c_int,
    _fd_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> c_int {
    log::debug!("Wrapper called: {}", "close");
    *ret_label = 0;

    let ret = libc::close(fd);
    if ret == -1 {
        // close failed
        return ret;
    }

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return ret;
    };

    tainter.trace_close(fd);

    ret
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_fclose(
    file: *mut FILE,
    _file_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> c_int {
    log::debug!("Wrapper called: {}", "fclose");
    *ret_label = 0;

    let fd = libc::fileno(file); // Accessing FILE after fclose is UB

    let ret = libc::fclose(file);
    if ret == libc::EOF {
        // fclose failed
        return ret;
    }
    assert!(fd != -1); // fclose should have failed if file is not valid

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return ret;
    };

    tainter.trace_close(fd);

    ret
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_mmap(
    addr: *mut c_void,
    len: size_t,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: off_t,
    _addr_label: dfsan_label,
    _len_label: dfsan_label,
    _prot_label: dfsan_label,
    _flags_label: dfsan_label,
    _fd_label: dfsan_label,
    _offset_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> *mut c_void {
    log::debug!("Wrapper called: {}", "mmap");
    *ret_label = 0;

    let addr_ret = libc::mmap(addr, len, prot, flags, fd, offset);
    if addr_ret == libc::MAP_FAILED {
        // mmap failed
        return addr_ret;
    }

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return addr_ret;
    };

    assert!(offset >= 0); // mmap fails with a negative offset
    tainter.trace_read(fd, addr_ret, offset as usize, len);

    addr_ret
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_munmap(
    addr: *mut c_void,
    len: size_t,
    _addr_label: dfsan_label,
    _len_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> c_int {
    log::debug!("Wrapper called: {}", "munmap");
    *ret_label = 0;

    let ret = libc::munmap(addr, len);
    if ret < 0 {
        // munmap failed
        return ret;
    }

    // In theory the conversion could fail, so panic if it does
    dfsan_set_label(0, addr, len.try_into().unwrap());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn __wrap___dfsw_read(
    fd: c_int,
    buf: *mut c_void,
    count: size_t,
    _fd_label: dfsan_label,
    _buf_label: dfsan_label,
    _count_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> ssize_t {
    log::debug!("Wrapper called: {}", "read");
    *ret_label = 0;

    let offset = libc::lseek(fd, 0, libc::SEEK_CUR);

    let bytes_read = libc::read(fd, buf, count);
    if bytes_read <= 0 {
        // No read occurred
        return bytes_read;
    }

    if offset < 0 {
        log::warn!(
            "Could not retrieve file offset: {}",
            get_c_error().to_string_lossy()
        );

        return bytes_read;
    }

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return bytes_read;
    };

    // offset and bytes_read are both positive
    tainter.trace_read(fd, buf, offset as usize, bytes_read as usize);

    bytes_read
}

#[no_mangle]
pub unsafe extern "C" fn __wrap___dfsw_pread(
    fd: c_int,
    buf: *mut c_void,
    count: size_t,
    offset: off_t,
    _fd_label: dfsan_label,
    _buf_label: dfsan_label,
    _count_label: dfsan_label,
    _offset_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> ssize_t {
    log::debug!("Wrapper called: {}", "pread");
    *ret_label = 0;

    let bytes_read = libc::pread(fd, buf, count, offset);
    if bytes_read <= 0 {
        // No read occurred
        return bytes_read;
    }

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return bytes_read;
    };

    // offset and bytes_read are both positive
    tainter.trace_read(fd, buf, offset as usize, bytes_read as usize);

    bytes_read
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_fread(
    ptr: *mut c_void,
    size: size_t,
    nobj: size_t,
    stream: *mut FILE,
    _ptr_label: dfsan_label,
    _size_label: dfsan_label,
    _nobj_label: dfsan_label,
    _stream_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> size_t {
    log::debug!("Wrapper called: {}", "fread");
    *ret_label = 0;

    let offset = libc::ftell(stream);

    let count = libc::fread(ptr, size, nobj, stream);
    if count == 0 {
        // No read occurred
        return count;
    }

    if offset < 0 {
        log::warn!(
            "Could not retrieve file offset: {}",
            get_c_error().to_string_lossy()
        );

        return count;
    }

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return count;
    };

    let bytes_read = if let Some(bytes_read) = count.checked_mul(size) {
        bytes_read
    } else {
        log::debug!("Overflow check failed");
        return count;
    };

    // stream is valid, otherwise fread would have failed
    // offset is positive, checked before
    tainter.trace_read(libc::fileno(stream), ptr, offset as usize, bytes_read);

    count
}

// Not in POSIX standard
#[no_mangle]
pub unsafe extern "C" fn __dfsw_fread_unlocked(
    ptr: *mut c_void,
    size: size_t,
    nobj: size_t,
    stream: *mut FILE,
    _ptr_label: dfsan_label,
    _size_label: dfsan_label,
    _nobj_label: dfsan_label,
    _stream_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> size_t {
    log::debug!("Wrapper called: {}", "fread_unlocked");
    *ret_label = 0;

    let offset = libc::ftell(stream);

    let count = libc::fread_unlocked(ptr, size, nobj, stream);
    if count == 0 {
        // No read occurred
        return count;
    }

    if offset < 0 {
        log::warn!(
            "Could not retrieve file offset: {}",
            get_c_error().to_string_lossy()
        );

        return count;
    }

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return count;
    };

    let bytes_read = if let Some(bytes_read) = count.checked_mul(size) {
        bytes_read
    } else {
        log::debug!("Overflow check failed");
        return count;
    };

    // stream is valid, otherwise fread would have failed
    // offset is positive, checked before
    tainter.trace_read(libc::fileno(stream), ptr, offset as usize, bytes_read);

    count
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_fgetc(
    stream: *mut FILE,
    _stream_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> c_int {
    log::debug!("Wrapper called: {}", "fgetc");
    *ret_label = 0;

    let offset = libc::ftell(stream);

    let c = libc::fgetc(stream);
    if c == libc::EOF {
        // Read failed
        return c;
    }

    if offset < 0 {
        log::warn!(
            "Could not retrieve file offset: {}",
            get_c_error().to_string_lossy()
        );

        return c;
    }

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return c;
    };

    // offset is guaranteed to be positive at this point
    if let Some(byte_label) = tainter.get_byte_label(libc::fileno(stream), offset as usize) {
        *ret_label = byte_label;
    }

    c
}

// Not in POSIX standard
#[no_mangle]
pub unsafe extern "C" fn __dfsw_fgetc_unlocked(
    stream: *mut FILE,
    stream_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> c_int {
    log::debug!("Redirect from: {}", "fgetc_unlocked");
    __dfsw_getc_unlocked(stream, stream_label, ret_label)
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_getc(
    stream: *mut FILE,
    stream_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> c_int {
    log::debug!("Redirect from: {}", "getc");
    __dfsw_fgetc(stream, stream_label, ret_label)
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_getc_unlocked(
    stream: *mut FILE,
    _stream_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> c_int {
    log::debug!("Wrapper called: {}", "getc_unlocked");
    *ret_label = 0;

    let offset = libc::ftell(stream);

    let c = getc_unlocked(stream);
    if c == libc::EOF {
        // Read failed
        return c;
    }

    if offset < 0 {
        log::warn!(
            "Could not retrieve file offset: {}",
            get_c_error().to_string_lossy()
        );

        return c;
    }

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return c;
    };

    // offset is guaranteed to be positive at this point
    if let Some(byte_label) = tainter.get_byte_label(libc::fileno(stream), offset as usize) {
        *ret_label = byte_label;
    }

    c
}

#[no_mangle]
pub unsafe extern "C" fn __wrap___dfsw_fgets(
    buf: *mut c_char,
    n: c_int,
    stream: *mut FILE,
    buf_label: dfsan_label,
    _n_label: dfsan_label,
    _stream_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> *mut c_char {
    log::debug!("Wrapper called: {}", "fgets");

    let offset = libc::ftell(stream);

    let buf_ret = libc::fgets(buf, n, stream);
    if buf_ret.is_null() {
        // Read failed
        *ret_label = 0;
        return buf_ret;
    } else {
        *ret_label = buf_label;
    }

    if offset < 0 {
        log::warn!(
            "Could not retrieve file offset: {}",
            get_c_error().to_string_lossy()
        );

        return buf_ret;
    }

    // fgets is guaranteed to append a \0 if it succeeds
    let bytes_read = libc::strlen(buf);

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return buf_ret;
    };

    // stream is valid, otherwise fgets would have failed
    // offset is positive, checked before
    tainter.trace_read(
        libc::fileno(stream),
        buf.cast(),
        offset as usize,
        bytes_read,
    );

    buf_ret
}

// Not in POSIX standard
#[no_mangle]
pub unsafe extern "C" fn __dfsw_fgets_unlocked(
    buf: *mut c_char,
    n: c_int,
    stream: *mut FILE,
    buf_label: dfsan_label,
    _n_label: dfsan_label,
    _stream_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> *mut c_char {
    log::debug!("Wrapper called: {}", "fgets_unlocked");

    let offset = libc::ftell(stream);

    let buf_ret = fgets_unlocked(buf, n, stream);
    if buf_ret.is_null() {
        // Read failed
        *ret_label = 0;
        return buf_ret;
    } else {
        *ret_label = buf_label;
    }

    if offset < 0 {
        log::warn!(
            "Could not retrieve file offset: {}",
            get_c_error().to_string_lossy()
        );

        return buf_ret;
    }

    // fgets is guaranteed to append a \0 if it succeeds
    let bytes_read = libc::strlen(buf);

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return buf_ret;
    };

    // stream is valid, otherwise fgets would have failed
    // offset is positive, checked before
    tainter.trace_read(
        libc::fileno(stream),
        buf.cast(),
        offset as usize,
        bytes_read,
    );

    buf_ret
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_getline(
    lineptr: *mut *mut c_char,
    n: *mut size_t,
    stream: *mut FILE,
    _lineptr_label: dfsan_label,
    _n_label: dfsan_label,
    _stream_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> ssize_t {
    log::debug!("Wrapper called: {}", "getline");
    *ret_label = 0;

    let offset = libc::ftell(stream);

    let bytes_read = libc::getline(lineptr, n, stream);
    if bytes_read <= 0 {
        // No read occurred
        return bytes_read;
    }

    if offset < 0 {
        log::warn!(
            "Could not retrieve file offset: {}",
            get_c_error().to_string_lossy()
        );

        return bytes_read;
    }

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return bytes_read;
    };

    // stream is valid, otherwise getline would have failed
    // offset and bytes_read are positive, checked before
    tainter.trace_read(
        libc::fileno(stream),
        *lineptr.cast(),
        offset as usize,
        bytes_read as usize,
    );

    bytes_read
}

#[no_mangle]
pub unsafe extern "C" fn __dfsw_getdelim(
    lineptr: *mut *mut c_char,
    n: *mut size_t,
    delim: c_int,
    stream: *mut FILE,
    _lineptr_label: dfsan_label,
    _n_label: dfsan_label,
    _delim_label: dfsan_label,
    _stream_label: dfsan_label,
    ret_label: *mut dfsan_label,
) -> ssize_t {
    log::debug!("Wrapper called: {}", "getdelim");
    *ret_label = 0;

    let offset = libc::ftell(stream);

    let bytes_read = getdelim(lineptr, n, delim, stream);
    if bytes_read <= 0 {
        // No read occurred
        return bytes_read;
    }

    if offset < 0 {
        log::warn!(
            "Could not retrieve file offset: {}",
            get_c_error().to_string_lossy()
        );

        return bytes_read;
    }

    let mut tainter = if let Some(tainter) = Tainter::global() {
        tainter
    } else {
        log::warn!("Tainter not initialized");
        return bytes_read;
    };

    // stream is valid, otherwise getdelim would have failed
    // offset and bytes_read are positive, checked before
    tainter.trace_read(
        libc::fileno(stream),
        *lineptr.cast(),
        offset as usize,
        bytes_read as usize,
    );

    bytes_read
}

fn get_c_error() -> &'static CStr {
    unsafe {
        let errno = *libc::__errno_location();
        &CStr::from_ptr(libc::strerror(errno))
    }
}
