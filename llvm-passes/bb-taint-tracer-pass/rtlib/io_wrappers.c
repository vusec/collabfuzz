#define _GNU_SOURCE

#include <fcntl.h>
#include <sanitizer/dfsan_interface.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#include "tainter.h"

#define UNUSED(x) __attribute__((unused)) x

// NOLINTNEXTLINE(readability-identifier-naming)
int __dfsw_open(const char* pathname, int flags,
                dfsan_label UNUSED(pathname_label),
                dfsan_label UNUSED(flags_label), dfsan_label* ret_label, ...) {
  *ret_label = 0;

  int mode = 0;
  if (__OPEN_NEEDS_MODE(flags)) {
    va_list arg;
    va_start(arg, ret_label);
    mode = va_arg(arg, int);
    va_end(arg);
  }

  int fd = open(pathname, flags, mode);
  if (fd == -1) {
    return fd;
  }

  tainter_trace_open(fd, pathname);

  return fd;
}

// NOLINTNEXTLINE(readability-identifier-naming)
FILE* __dfsw_fopen(const char* pathname, const char* mode,
                   dfsan_label UNUSED(pathname_label),
                   dfsan_label UNUSED(mode_label), dfsan_label* ret_label) {
  *ret_label = 0;

  FILE* stream = fopen(pathname, mode);
  if (!stream) {
    return stream;
  }

  tainter_trace_open(fileno(stream), pathname);

  return stream;
}

// NOLINTNEXTLINE(readability-identifier-naming)
FILE* __dfsw_fopen64(const char* pathname, const char* mode,
                     dfsan_label pathname_label, dfsan_label mode_label,
                     dfsan_label* ret_label) {
  return __dfsw_fopen(pathname, mode, pathname_label, mode_label, ret_label);
}

// NOLINTNEXTLINE(readability-identifier-naming)
int __dfsw_close(int fd, dfsan_label UNUSED(fd_label), dfsan_label* ret_label) {
  *ret_label = 0;

  int ret = close(fd);
  if (ret == -1) {
    return ret;
  }

  tainter_trace_close(fd);

  return ret;
}

// NOLINTNEXTLINE(readability-identifier-naming)
int __dfsw_fclose(FILE* stream, dfsan_label UNUSED(stream_label),
                  dfsan_label* ret_label) {
  *ret_label = 0;

  int ret = fclose(stream);
  if (ret == EOF) {
    return ret;
  }

  tainter_trace_close(fileno(stream));

  return ret;
}

// NOLINTNEXTLINE(readability-identifier-naming)
void* __dfsw_mmap(void* addr, size_t length, int prot, int flags, int fd,
                  off_t offset, dfsan_label UNUSED(addr_label),
                  dfsan_label UNUSED(length_label),
                  dfsan_label UNUSED(prot_label),
                  dfsan_label UNUSED(flags_label), dfsan_label UNUSED(fd_label),
                  dfsan_label UNUSED(offset_label), dfsan_label* ret_label) {
  *ret_label = 0;

  void* ret = mmap(addr, length, prot, flags, fd, offset);
  if (ret == MAP_FAILED) {
    return ret;
  }

  int input_label = tainter_is_input_fd(fd) ? tainter_get_input_label() : 0;
  dfsan_set_label(input_label, ret, length);

  return ret;
}

// NOLINTNEXTLINE(readability-identifier-naming)
int __dfsw_munmap(void* addr, size_t length, dfsan_label UNUSED(addr_label),
                  dfsan_label UNUSED(length_label), dfsan_label* ret_label) {
  *ret_label = 0;

  int ret = munmap(addr, length);
  if (ret < 0) {
    return ret;
  }

  dfsan_set_label(0, addr, length);

  return ret;
}

// NOLINTNEXTLINE(readability-identifier-naming)
ssize_t __wrap___dfsw_read(int fd, void* buf, size_t count,
                           dfsan_label UNUSED(fd_label),
                           dfsan_label UNUSED(buf_label),
                           dfsan_label UNUSED(count_label),
                           dfsan_label* ret_label) {
  *ret_label = 0;

  ssize_t ret = read(fd, buf, count);
  if (ret <= 0) {
    return ret;
  }

  int input_label = tainter_is_input_fd(fd) ? tainter_get_input_label() : 0;
  dfsan_set_label(input_label, buf, ret);
  *ret_label = input_label;  // Output may depend on the size of the file

  return ret;
}

// NOLINTNEXTLINE(readability-identifier-naming)
ssize_t __wrap___dfsw_pread(int fd, void* buf, size_t count, off_t offset,
                            dfsan_label UNUSED(fd_label),
                            dfsan_label UNUSED(buf_label),
                            dfsan_label UNUSED(count_label),
                            dfsan_label UNUSED(offset_label),
                            dfsan_label* ret_label) {
  *ret_label = 0;

  ssize_t ret = pread(fd, buf, count, offset);
  if (ret <= 0) {
    return ret;
  }

  int input_label = tainter_is_input_fd(fd) ? tainter_get_input_label() : 0;
  dfsan_set_label(input_label, buf, ret);
  *ret_label = input_label;  // Output may depend on the size of the file

  return ret;
}

// NOLINTNEXTLINE(readability-identifier-naming)
size_t __dfsw_fread(void* ptr, size_t size, size_t nmemb, FILE* stream,
                    dfsan_label UNUSED(ptr_label),
                    dfsan_label UNUSED(size_label),
                    dfsan_label UNUSED(nmemb_label),
                    dfsan_label UNUSED(stream_label), dfsan_label* ret_label) {
  *ret_label = 0;

  size_t ret = fread(ptr, size, nmemb, stream);
  if (ret == 0) {
    return ret;
  }

  int input_label =
      tainter_is_input_fd(fileno(stream)) ? tainter_get_input_label() : 0;
  dfsan_set_label(input_label, ptr, ret * size);
  *ret_label = input_label;  // Output may depend on the size of the file

  return ret;
}

// NOLINTNEXTLINE(readability-identifier-naming)
size_t __dfsw_fread_unlocked(void* ptr, size_t size, size_t nmemb, FILE* stream,
                             dfsan_label UNUSED(ptr_label),
                             dfsan_label UNUSED(size_label),
                             dfsan_label UNUSED(nmemb_label),
                             dfsan_label UNUSED(stream_label),
                             dfsan_label* ret_label) {
  *ret_label = 0;

  size_t ret = fread_unlocked(ptr, size, nmemb, stream);
  if (ret == 0) {
    return ret;
  }

  int input_label =
      tainter_is_input_fd(fileno(stream)) ? tainter_get_input_label() : 0;
  dfsan_set_label(input_label, ptr, ret * size);
  *ret_label = input_label;  // Output may depend on the size of the file

  return ret;
}

// NOLINTNEXTLINE(readability-identifier-naming)
int __dfsw_fgetc(FILE* stream, dfsan_label UNUSED(stream_label),
                 dfsan_label* ret_label) {
  *ret_label = 0;

  int c = fgetc(stream);
  if (c == EOF) {
    return c;
  }

  *ret_label =
      tainter_is_input_fd(fileno(stream)) ? tainter_get_input_label() : 0;

  return c;
}

// NOLINTNEXTLINE(readability-identifier-naming)
int __dfsw_fgetc_unlocked(FILE* stream, dfsan_label UNUSED(stream_label),
                          dfsan_label* ret_label) {
  *ret_label = 0;

  int c = fgetc_unlocked(stream);
  if (c == EOF) {
    return c;
  }

  *ret_label =
      tainter_is_input_fd(fileno(stream)) ? tainter_get_input_label() : 0;

  return c;
}

// NOLINTNEXTLINE(readability-identifier-naming)
int __dfsw_getc(FILE* stream, dfsan_label UNUSED(stream_label),
                dfsan_label* ret_label) {
  *ret_label = 0;

  int c = getc(stream);
  if (c == EOF) {
    return c;
  }

  *ret_label =
      tainter_is_input_fd(fileno(stream)) ? tainter_get_input_label() : 0;

  return c;
}

// NOLINTNEXTLINE(readability-identifier-naming)
int __dfsw_getc_unlocked(FILE* stream, dfsan_label UNUSED(stream_label),
                         dfsan_label* ret_label) {
  *ret_label = 0;

  int c = getc_unlocked(stream);
  if (c == EOF) {
    return c;
  }

  *ret_label =
      tainter_is_input_fd(fileno(stream)) ? tainter_get_input_label() : 0;

  return c;
}

// NOLINTNEXTLINE(readability-identifier-naming)
int __dfsw__IO_getc(FILE* stream, dfsan_label stream_label,
                    dfsan_label* ret_label) {
  return __dfsw_getc(stream, stream_label, ret_label);
}

// NOLINTNEXTLINE(readability-identifier-naming)
int __dfsw_getchar(dfsan_label* ret_label) {
  *ret_label = 0;

  int c = getchar();
  if (c == EOF) {
    return c;
  }

  *ret_label =
      tainter_is_input_fd(fileno(stdin)) ? tainter_get_input_label() : 0;

  return c;
}

// NOLINTNEXTLINE(readability-identifier-naming)
int __dfsw_getchar_unlocked(dfsan_label* ret_label) {
  *ret_label = 0;

  int c = getchar_unlocked();
  if (c == EOF) {
    return c;
  }

  *ret_label =
      tainter_is_input_fd(fileno(stdin)) ? tainter_get_input_label() : 0;

  return c;
}

// NOLINTNEXTLINE(readability-identifier-naming)
char* __wrap___dfsw_fgets(char* s, int size, FILE* stream, dfsan_label s_label,
                          dfsan_label UNUSED(size_label),
                          dfsan_label UNUSED(stream_label),
                          dfsan_label* ret_label) {
  *ret_label = 0;

  char* ret = fgets(s, size, stream);
  if (!ret) {
    return ret;
  }

  int input_label =
      tainter_is_input_fd(fileno(stream)) ? tainter_get_input_label() : 0;
  dfsan_set_label(input_label, ret, strlen(ret) + 1);
  *ret_label = s_label;

  return ret;
}

// NOLINTNEXTLINE(readability-identifier-naming)
char* __dfsw_fgets_unlocked(char* s, int size, FILE* stream,
                            dfsan_label s_label, dfsan_label UNUSED(size_label),
                            dfsan_label UNUSED(stream_label),
                            dfsan_label* ret_label) {
  *ret_label = 0;

  char* ret = fgets_unlocked(s, size, stream);
  if (!ret) {
    return ret;
  }

  int input_label =
      tainter_is_input_fd(fileno(stream)) ? tainter_get_input_label() : 0;
  dfsan_set_label(input_label, ret, strlen(ret) + 1);
  *ret_label = s_label;

  return ret;
}

// NOLINTNEXTLINE(readability-identifier-naming)
ssize_t __dfsw_getline(char** lineptr, size_t* n, FILE* stream,
                       dfsan_label UNUSED(lineptr_label),
                       dfsan_label UNUSED(n_label),
                       dfsan_label UNUSED(stream_label),
                       dfsan_label* ret_label) {
  *ret_label = 0;

  ssize_t ret = getline(lineptr, n, stream);
  if (ret <= 0) {
    return ret;
  }

  int input_label =
      tainter_is_input_fd(fileno(stream)) ? tainter_get_input_label() : 0;

  dfsan_set_label(input_label, *lineptr, ret);
  *ret_label = input_label;  // Output may depend on the size of the file

  return ret;
}

// NOLINTNEXTLINE(readability-identifier-naming)
ssize_t __dfsw_getdelim(char** lineptr, size_t* n, int delim, FILE* stream,
                        dfsan_label UNUSED(lineptr_label),
                        dfsan_label UNUSED(n_label),
                        dfsan_label UNUSED(delim_label),
                        dfsan_label UNUSED(stream_label),
                        dfsan_label* ret_label) {
  *ret_label = 0;

  ssize_t ret = getdelim(lineptr, n, delim, stream);
  if (ret <= 0) {
    return ret;
  }

  int input_label =
      tainter_is_input_fd(fileno(stream)) ? tainter_get_input_label() : 0;

  dfsan_set_label(input_label, *lineptr, ret);
  *ret_label = input_label;  // Output may depend on the size of the file

  return ret;
}
