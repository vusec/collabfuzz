#include <fcntl.h>
#include <sanitizer/icount_interface.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#include "tainter.h"

#define UNUSED(x) __attribute__((unused)) x
typedef unsigned long uptr;

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_open(const void UNUSED(*caller_pc), int fd,
                           const char* path, int UNUSED(oflags),
                           icount_shadow UNUSED(path_shadow),
                           icount_shadow UNUSED(flag_shadow),
                           icount_shadow UNUSED(*va_shadows),
                           icount_shadow UNUSED(*ret_shadow),
                           int UNUSED(mode)) {
  if (fd == -1) {
    return;
  }

  tainter_trace_open(fd, path);
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_fopen(const void UNUSED(*caller_pc), FILE* stream,
                            const char* filename, const char UNUSED(*mode),
                            icount_shadow UNUSED(fn_shadow),
                            icount_shadow UNUSED(mode_shadow),
                            icount_shadow UNUSED(*ret_shadow)) {
  if (!stream) {
    return;
  }

  tainter_trace_open(fileno(stream), filename);
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_close(const void UNUSED(*caller_pc), int res, int fd,
                            icount_shadow UNUSED(fd_shadow),
                            icount_shadow UNUSED(*ret_shadow)) {
  if (res == -1) {
    return;
  }

  tainter_trace_close(fd);
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_fclose(const void UNUSED(*caller_pc), int res,
                             FILE* stream, icount_shadow UNUSED(file_shadow),
                             icount_shadow UNUSED(*ret_shadow)) {
  if (res == EOF) {
    return;
  }

  tainter_trace_close(fileno(stream));
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_mmap(
    const void UNUSED(*caller_pc), void* ret, void UNUSED(*addr),
    size_t UNUSED(length), int UNUSED(prot), int UNUSED(flags), int fd,
    off_t UNUSED(offset), icount_shadow UNUSED(addr_shadow),
    icount_shadow UNUSED(length_shadow), icount_shadow UNUSED(prot_shadow),
    icount_shadow UNUSED(flags_shadow), icount_shadow UNUSED(fd_shadow),
    icount_shadow UNUSED(offset_shadow), icount_shadow UNUSED(*ret_shadow)) {
  if (ret == MAP_FAILED) {
    return;
  }

  if (tainter_is_input_fd(fd)) {
    icount_enable_range_shadow(ret, length);
  }
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_read(const void UNUSED(*caller_pc), ssize_t ret, int fd,
                           void* buf, size_t UNUSED(count),
                           icount_shadow UNUSED(fd_label),
                           icount_shadow UNUSED(buf_label),
                           icount_shadow UNUSED(count_label),
                           icount_shadow UNUSED(*ret_label)) {
  if (ret <= 0) {
    return;
  }

  if (tainter_is_input_fd(fd)) {
    icount_enable_range_shadow(buf, ret);
  }
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_pread(
    const void UNUSED(*caller_pc), ssize_t ret, int fd, void* buf,
    size_t UNUSED(count), off_t UNUSED(offset), icount_shadow UNUSED(fd_label),
    icount_shadow UNUSED(buf_label), icount_shadow UNUSED(count_label),
    icount_shadow UNUSED(offset_label), icount_shadow UNUSED(*ret_label)) {
  if (ret <= 0) {
    return;
  }

  if (tainter_is_input_fd(fd)) {
    icount_enable_range_shadow(buf, ret);
  }
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_fread(
    const void UNUSED(*caller_pc), size_t ret, void* ptr, size_t UNUSED(size),
    size_t UNUSED(nmemb), FILE* stream, icount_shadow UNUSED(ptr_label),
    icount_shadow UNUSED(size_label), icount_shadow UNUSED(nmemb_label),
    icount_shadow UNUSED(stream_label), icount_shadow UNUSED(*ret_label)) {
  if (ret == 0) {
    return;
  }

  if (tainter_is_input_fd(fileno(stream))) {
    icount_enable_range_shadow(ptr, ret * size);
  }
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_fread_unlocked(const void* caller_pc, size_t ret,
                                     void* ptr, size_t size, size_t nmemb,
                                     FILE* stream, icount_shadow ptr_label,
                                     icount_shadow size_label,
                                     icount_shadow nmemb_label,
                                     icount_shadow stream_label,
                                     icount_shadow* ret_label) {
  icount_weak_hook_fread(caller_pc, ret, ptr, size, nmemb, stream, ptr_label,
                         size_label, nmemb_label, stream_label, ret_label);
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_fgetc(const void UNUSED(*caller_pc), int c, FILE* stream,
                            icount_shadow UNUSED(stream_label),
                            icount_shadow* ret_label) {
  if (c == EOF) {
    return;
  }

  if (tainter_is_input_fd(fileno(stream))) {
    *ret_label = tainter_get_init_count();
  }
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_fgetc_unlocked(const void* caller_pc, int res,
                                     FILE* stream, icount_shadow stream_label,
                                     icount_shadow* ret_label) {
  icount_weak_hook_fgetc(caller_pc, res, stream, stream_label, ret_label);
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_getc(const void* caller_pc, int c, FILE* stream,
                           icount_shadow stream_label,
                           icount_shadow* ret_label) {
  icount_weak_hook_fgetc(caller_pc, c, stream, stream_label, ret_label);
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_getc_unlocked(const void* caller_pc, int c, FILE* stream,
                                    icount_shadow UNUSED(stream_label),
                                    icount_shadow* ret_label) {
  icount_weak_hook_fgetc(caller_pc, c, stream, stream_label, ret_label);
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_getchar(const void UNUSED(*caller_pc), int c,
                              icount_shadow* ret_label) {
  if (c == EOF) {
    return;
  }

  if (tainter_is_input_fd(fileno(stdin))) {
    *ret_label = tainter_get_init_count();
    printf("ret_label: %d\n", *ret_label);
  }
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_getchar_unlocked(const void* caller_pc, int c,
                                       icount_shadow* ret_label) {
  icount_weak_hook_getchar(caller_pc, c, ret_label);
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_fgets(const void UNUSED(*caller_pc), char* ret,
                            char UNUSED(*str), int UNUSED(count), FILE* stream,
                            icount_shadow UNUSED(str_label),
                            icount_shadow UNUSED(count_label),
                            icount_shadow UNUSED(stream_label),
                            icount_shadow UNUSED(*ret_label)) {
  if (!ret) {
    return;
  }

  if (tainter_is_input_fd(fileno(stream))) {
    icount_enable_range_shadow(ret, strlen(ret) + 1);
  }
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_getline(const void UNUSED(*caller_pc), ssize_t ret,
                              char** lineptr, size_t* UNUSED(n), FILE* stream,
                              icount_shadow UNUSED(lineptr_label),
                              icount_shadow UNUSED(n_label),
                              icount_shadow UNUSED(stream_label),
                              icount_shadow UNUSED(*ret_label)) {
  if (ret <= 0) {
    return;
  }

  if (tainter_is_input_fd(fileno(stream))) {
    icount_enable_range_shadow(*lineptr, *n);
  }
}

// NOLINTNEXTLINE(readability-identifier-naming)
void icount_weak_hook_getdelim(const void UNUSED(*caller_pc), ssize_t ret,
                               char** lineptr, size_t* UNUSED(n),
                               int UNUSED(delim), FILE* stream,
                               icount_shadow UNUSED(lineptr_label),
                               icount_shadow UNUSED(n_label),
                               icount_shadow UNUSED(delim_label),
                               icount_shadow UNUSED(stream_label),
                               icount_shadow UNUSED(*ret_label)) {
  if (ret <= 0) {
    return;
  }

  if (tainter_is_input_fd(fileno(stream))) {
    icount_enable_range_shadow(*lineptr, *n);
  }
}
