//===-- icount_interface.h ------------------------------------------------===//
//
//                     The LLVM Compiler Infrastructure
//
// This file is distributed under the University of Illinois Open Source
// License. See LICENSE.TXT for details.
//
//===----------------------------------------------------------------------===//
//
// This file is a part of InstructionCounter.
//
// Public interface header.
//===----------------------------------------------------------------------===//
#ifndef ICOUNT_INTERFACE_H
#define ICOUNT_INTERFACE_H

#include <sanitizer/common_interface_defs.h>

#if !defined(__cplusplus)
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#else
#include <cstddef>
#include <cstdint>
#include <cstdio>

extern "C" {
#endif

typedef uint16_t icount_shadow;

/// Returns the result of the transfer function applied to \c shadow.
icount_shadow icount_transfer_shadow(icount_shadow shadow);

/// Returns the result of the combination function applied to \c s1 and \c s2.
icount_shadow icount_combine_shadows(icount_shadow s1, icount_shadow s2);

/// Returns the shadow for the range specified, which is calculated combining
/// the shadows for all the bytes in the range.
icount_shadow icount_get_range_shadow(const void *addr, size_t size);

/// Retrieves the shadow associated with the given data.
///
/// The type of 'data' is arbitrary. The function accepts a value of any type,
/// which can be truncated or extended (implicitly or explicitly) as necessary.
/// The truncation/extension operations will preserve the label of the original
/// value.
icount_shadow icount_get_shadow(long data);

/// Sets the shadow for the range specified to \c shadow.
void icount_set_range_shadow(icount_shadow shadow, void *addr, size_t size);

/// Sets the shadows for the range specified to 1. This function should be used
/// to mark which values should be used to perform the count.
void icount_enable_range_shadow(void *addr, size_t size);

/// Sets the shadows for the range specified to 0. The 0 shadow is a special
/// value that indicates that this shadow should be ignored.
void icount_disable_range_shadow(void *addr, size_t size);

/// Interceptor hooks.
/// Whenever a icount's custom function is called the corresponding
/// hook is called it non-zero. The hooks should be defined by the user.
/// The primary use case is taint-guided fuzzing, where the fuzzer
/// needs to see the parameters of the function and the labels.
void icount_weak_hook_open(const void *caller_pc, int fd, const char *path,
                           int oflags, icount_shadow path_shadow,
                           icount_shadow flag_shadow, icount_shadow *va_shadows,
                           icount_shadow *ret_shadow, int mode);

void icount_weak_hook_fopen(const void *caller_pc, FILE *stream,
                            const char *filename, const char *mode,
                            icount_shadow fn_shadow, icount_shadow mode_shadow,
                            icount_shadow *ret_shadow);

void icount_weak_hook_close(const void *caller_pc, int res, int fd,
                            icount_shadow fd_shadow, icount_shadow *ret_shadow);

void icount_weak_hook_fclose(const void *caller_pc, int res, FILE *stream,
                             icount_shadow file_shadow,
                             icount_shadow *ret_shadow);

void icount_weak_hook_mmap(const void *caller_pc, void *ret, void *addr,
                           size_t length, int prot, int flags, int fd,
                           off_t offset, icount_shadow addr_shadow,
                           icount_shadow length_shadow,
                           icount_shadow prot_shadow,
                           icount_shadow flags_shadow, icount_shadow fd_shadow,
                           icount_shadow offset_shadow,
                           icount_shadow *ret_shadow);

void icount_weak_hook_munmap(const void *caller_pc, int res, void *addr,
                             size_t length, icount_shadow addr_shadow,
                             icount_shadow length_shadow,
                             icount_shadow *ret_shadow);

void icount_weak_hook_fread(const void *caller_pc, size_t ret, void *ptr,
                            size_t size, size_t nmemb, FILE *stream,
                            icount_shadow ptr_label, icount_shadow size_label,
                            icount_shadow nmemb_label,
                            icount_shadow stream_label,
                            icount_shadow *ret_label);

void icount_weak_hook_fread_unlocked(const void *caller_pc, size_t ret,
                                     void *ptr, size_t size, size_t nmemb,
                                     FILE *stream, icount_shadow ptr_label,
                                     icount_shadow size_label,
                                     icount_shadow nmemb_label,
                                     icount_shadow stream_label,
                                     icount_shadow *ret_label);

void icount_weak_hook_read(const void *caller_pc, ssize_t ret, int fd,
                           void *buf, size_t count, icount_shadow fd_label,
                           icount_shadow buf_label, icount_shadow count_label,
                           icount_shadow *ret_label);

void icount_weak_hook_pread(const void *caller_pc, ssize_t ret, int fd,
                            void *buf, size_t count, off_t offset,
                            icount_shadow fd_label, icount_shadow buf_label,
                            icount_shadow count_label,
                            icount_shadow offset_label,
                            icount_shadow *ret_label);

void icount_weak_hook_fgetc(const void *caller_pc, int c, FILE *stream,
                            icount_shadow stream_label,
                            icount_shadow *ret_label);

void icount_weak_hook_fgetc_unlocked(const void *caller_pc, int res,
                                     FILE *stream, icount_shadow stream_label,
                                     icount_shadow *ret_label);

void icount_weak_hook_getc(const void *caller_pc, int c, FILE *stream,
                           icount_shadow stream_label,
                           icount_shadow *ret_label);

void icount_weak_hook_getc_unlocked(const void *caller_pc, int c, FILE *stream,
                                    icount_shadow stream_label,
                                    icount_shadow *ret_label);

void icount_weak_hook_getchar(const void *caller_pc, int c,
                              icount_shadow *ret_label);

void icount_weak_hook_getchar_unlocked(const void *caller_pc, int c,
                                       icount_shadow *ret_label);

void icount_weak_hook_fgets(const void *caller_pc, char *ret, char *str,
                            int count, FILE *stream, icount_shadow str_label,
                            icount_shadow count_label,
                            icount_shadow stream_label,
                            icount_shadow *ret_label);

void icount_weak_hook_fgets_unlocked(const void *caller_pc, char *ret,
                                     char *str, int count, FILE *fd,
                                     icount_shadow str_label,
                                     icount_shadow count_label,
                                     icount_shadow fd_label,
                                     icount_shadow *ret_label);

void icount_weak_hook_getline(const void *caller_pc, ssize_t ret,
                              char **lineptr, size_t *n, FILE *stream,
                              icount_shadow lineptr_label,
                              icount_shadow n_label, icount_shadow stream_label,
                              icount_shadow *ret_label);

void icount_weak_hook_getdelim(const void *caller_pc, ssize_t ret,
                               char **lineptr, size_t *n, int delim,
                               FILE *stream, icount_shadow lineptr_label,
                               icount_shadow n_label, icount_shadow delim_label,
                               icount_shadow stream_label,
                               icount_shadow *ret_label);

#ifdef __cplusplus
} // extern "C"

template <typename T> void icount_enable_shadow(T &data) { // NOLINT
  icount_enable_range_shadow(static_cast<void *>(&data), sizeof(T));
}

template <typename T> void icount_disable_shadow(T &data) { // NOLINT
  icount_disable_range_shadow(static_cast<void *>(&data), sizeof(T));
}

#endif

#endif // ICOUNT_INTERFACE_H
