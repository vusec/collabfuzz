//===-- icount_custom.cc --------------------------------------------------===//
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
// This file defines the custom functions listed in done_abilist.txt.
//===----------------------------------------------------------------------===//

#include <arpa/inet.h>
#include <assert.h>
#include <ctype.h>
#include <dlfcn.h>
#include <fcntl.h>
#include <link.h>
#include <poll.h>
#include <pthread.h>
#include <pwd.h>
#include <sched.h>
#include <signal.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/resource.h>
#include <sys/select.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#include "icount/icount.h"
#include "sanitizer_common/sanitizer_common.h"
#include "sanitizer_common/sanitizer_internal_defs.h"
#include "sanitizer_common/sanitizer_linux.h"

using namespace __icount;

#define CALL_WEAK_INTERCEPTOR_HOOK(f, ...) \
  do {                                     \
    if (f)                                 \
      f(__VA_ARGS__);                      \
  } while (false)
#define DECLARE_WEAK_INTERCEPTOR_HOOK(f, ...) \
  SANITIZER_INTERFACE_ATTRIBUTE SANITIZER_WEAK_ATTRIBUTE void f(__VA_ARGS__);

extern "C" {
DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_open, uptr caller_pc, int fd,
                              const char *path, int oflags,
                              icount_shadow path_shadow,
                              icount_shadow flag_shadow,
                              icount_shadow *va_shadows,
                              icount_shadow *ret_shadow, int mode);

// Same behavior as discard, just call the weak hook
SANITIZER_INTERFACE_ATTRIBUTE int __icountw_open(const char *path, int oflags,
                                                 icount_shadow path_shadow,
                                                 icount_shadow flag_shadow,
                                                 icount_shadow *va_shadows,
                                                 icount_shadow *ret_shadow,
                                                 ...) {
  *ret_shadow = 0;

  int mode = 0;
  if (__OPEN_NEEDS_MODE(oflags)) {
    va_list arg;
    va_start(arg, ret_shadow);
    mode = va_arg(arg, int);
    va_end(arg);
  }

  int fd = open(path, oflags, mode);

  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_open, GET_CALLER_PC(), fd, path,
                             oflags, path_shadow, flag_shadow, va_shadows,
                             ret_shadow, mode);

  return fd;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fopen, uptr caller_pc,
                              FILE *stream, const char *filename,
                              const char *mode, icount_shadow fn_shadow,
                              icount_shadow mode_shadow,
                              icount_shadow *ret_shadow)

// Same behavior as discard, just call the weak hook
SANITIZER_INTERFACE_ATTRIBUTE FILE *__icountw_fopen(const char *filename,
                                                    const char *mode,
                                                    icount_shadow fn_shadow,
                                                    icount_shadow mode_shadow,
                                                    icount_shadow *ret_shadow) {
  FILE *stream = fopen(filename, mode);

  *ret_shadow = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fopen, GET_CALLER_PC(), stream,
                             filename, mode, fn_shadow, mode_shadow,
                             ret_shadow);

  return stream;
}

SANITIZER_INTERFACE_ATTRIBUTE FILE *__icountw_fopen64(
    const char *filename, const char *mode, icount_shadow fn_shadow,
    icount_shadow mode_shadow, icount_shadow *ret_shadow) {
  return __icountw_fopen(filename, mode, fn_shadow, mode_shadow, ret_shadow);
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_close, uptr caller_pc, int res,
                              int fd, icount_shadow fd_shadow,
                              icount_shadow *ret_shadow)

// Same behavior as discard, just call the weak hook
SANITIZER_INTERFACE_ATTRIBUTE int __icountw_close(int fd,
                                                  icount_shadow fd_shadow,
                                                  icount_shadow *ret_shadow) {
  int res = close(fd);

  *ret_shadow = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_close, GET_CALLER_PC(), res, fd,
                             fd_shadow, ret_shadow);

  return res;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fclose, uptr caller_pc, int res,
                              FILE *stream, icount_shadow file_shadow,
                              icount_shadow *ret_shadow)

// Same behavior as discard, just call the weak hook
SANITIZER_INTERFACE_ATTRIBUTE int __icountw_fclose(FILE *stream,
                                                   icount_shadow file_shadow,
                                                   icount_shadow *ret_shadow) {
  int res = fclose(stream);

  *ret_shadow = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fclose, GET_CALLER_PC(), res,
                             stream, file_shadow, ret_shadow);

  return res;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(
    icount_weak_hook_mmap, uptr caller_pc, void *ret, void *addr, size_t length,
    int prot, int flags, int fd, off_t offset, icount_shadow start_shadow,
    icount_shadow len_shadow, icount_shadow prot_shadow,
    icount_shadow flags_shadow, icount_shadow fd_shadow,
    icount_shadow offset_shadow, icount_shadow *ret_shadow)

// XXX: May have interfere with interceptors
SANITIZER_INTERFACE_ATTRIBUTE void *__icountw_mmap(
    void *addr, size_t length, int prot, int flags, int fd, off_t offset,
    icount_shadow start_shadow, icount_shadow len_shadow,
    icount_shadow prot_shadow, icount_shadow flags_shadow,
    icount_shadow fd_shadow, icount_shadow offset_shadow,
    icount_shadow *ret_shadow) {
  void *ret = mmap(addr, length, prot, flags, fd, offset);

  *ret_shadow = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_mmap, GET_CALLER_PC(), ret, addr,
                             length, prot, flags, fd, offset, start_shadow,
                             len_shadow, prot_shadow, flags_shadow, fd_shadow,
                             offset_shadow, ret_shadow);

  return ret;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_munmap, uptr caller_pc, int res,
                              void *addr, size_t length,
                              icount_shadow addr_shadow,
                              icount_shadow length_shadow,
                              icount_shadow *ret_shadow)

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_munmap(void *addr, size_t length,
                                                   icount_shadow addr_shadow,
                                                   icount_shadow length_shadow,
                                                   icount_shadow *ret_shadow) {
  int res = munmap(addr, length);
  if (res == 0) {
    icount_disable_range_shadow(addr, length);
  }

  *ret_shadow = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_munmap, GET_CALLER_PC(), res,
                             addr, length, addr_shadow, length_shadow,
                             ret_shadow);

  return res;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fread, uptr caller_pc,
                              size_t ret, void *ptr, size_t size, size_t nmemb,
                              FILE *stream, icount_shadow ptr_label,
                              icount_shadow size_label,
                              icount_shadow nmemb_label,
                              icount_shadow stream_label,
                              icount_shadow *ret_label)

SANITIZER_INTERFACE_ATTRIBUTE size_t __icountw_fread(
    void *ptr, size_t size, size_t nmemb, FILE *stream, icount_shadow ptr_label,
    icount_shadow size_label, icount_shadow nmemb_label,
    icount_shadow stream_label, icount_shadow *ret_label) {
  size_t ret = fread(ptr, size, nmemb, stream);
  if (ret > 0)
    icount_disable_range_shadow(ptr, ret * size);

  *ret_label = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fread, GET_CALLER_PC(), ret, ptr,
                             size, nmemb, stream, ptr_label, size_label,
                             nmemb_label, stream_label, ret_label);

  return ret;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fread_unlocked, uptr caller_pc,
                              size_t ret, void *ptr, size_t size, size_t nmemb,
                              FILE *stream, icount_shadow ptr_label,
                              icount_shadow size_label,
                              icount_shadow nmemb_label,
                              icount_shadow stream_label,
                              icount_shadow *ret_label)

SANITIZER_INTERFACE_ATTRIBUTE size_t __icountw_fread_unlocked(
    void *ptr, size_t size, size_t nmemb, FILE *stream, icount_shadow ptr_label,
    icount_shadow size_label, icount_shadow nmemb_label,
    icount_shadow stream_label, icount_shadow *ret_label) {
  size_t ret = fread_unlocked(ptr, size, nmemb, stream);
  if (ret > 0)
    icount_disable_range_shadow(ptr, ret * size);

  *ret_label = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fread, GET_CALLER_PC(), ret, ptr,
                             size, nmemb, stream, ptr_label, size_label,
                             nmemb_label, stream_label, ret_label);
  return ret;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_read, uptr caller_pc,
                              ssize_t ret, int fd, void *buf, size_t count,
                              icount_shadow fd_label, icount_shadow buf_label,
                              icount_shadow count_label,
                              icount_shadow *ret_label)

SANITIZER_INTERFACE_ATTRIBUTE ssize_t __icountw_read(int fd, void *buf,
                                                     size_t count,
                                                     icount_shadow fd_label,
                                                     icount_shadow buf_label,
                                                     icount_shadow count_label,
                                                     icount_shadow *ret_label) {
  ssize_t ret = read(fd, buf, count);
  if (ret > 0)
    icount_disable_range_shadow(buf, ret);

  *ret_label = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_read, GET_CALLER_PC(), ret, fd,
                             buf, count, fd_label, buf_label, count_label,
                             ret_label);

  return ret;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_pread, uptr caller_pc,
                              ssize_t ret, int fd, void *buf, size_t count,
                              off_t offset, icount_shadow fd_label,
                              icount_shadow buf_label,
                              icount_shadow count_label,
                              icount_shadow offset_label,
                              icount_shadow *ret_label)

SANITIZER_INTERFACE_ATTRIBUTE ssize_t __icountw_pread(
    int fd, void *buf, size_t count, off_t offset, icount_shadow fd_label,
    icount_shadow buf_label, icount_shadow count_label,
    icount_shadow offset_label, icount_shadow *ret_label) {
  ssize_t ret = pread(fd, buf, count, offset);
  if (ret > 0)
    icount_disable_range_shadow(buf, ret);

  *ret_label = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_pread, GET_CALLER_PC(), ret, fd,
                             buf, count, offset, fd_label, buf_label,
                             count_label, offset_label, ret_label);

  return ret;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fgetc, uptr caller_pc, int c,
                              FILE *stream, icount_shadow fd_label,
                              icount_shadow *ret_label)

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_fgetc(FILE *stream,
                                                  icount_shadow fd_label,
                                                  icount_shadow *ret_label) {
  int c = fgetc(stream);

  *ret_label = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fgetc, GET_CALLER_PC(), c, stream,
                             fd_label, ret_label);

  return c;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fgetc_unlocked, uptr caller_pc,
                              int res, FILE *stream, icount_shadow fd_label,
                              icount_shadow *ret_label)

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_fgetc_unlocked(
    FILE *stream, icount_shadow fd_label, icount_shadow *ret_label) {
  int c = fgetc_unlocked(stream);

  *ret_label = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fgetc_unlocked, GET_CALLER_PC(),
                             c, stream, fd_label, ret_label);

  return c;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getc, uptr caller_pc, int c,
                              FILE *stream, icount_shadow stream_label,
                              icount_shadow *ret_label)

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_getc(FILE *stream,
                                                 icount_shadow stream_label,
                                                 icount_shadow *ret_label) {
  int c = getc(stream);

  *ret_label = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getc, GET_CALLER_PC(), c, stream,
                             stream_label, ret_label);

  return c;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getc_unlocked, uptr caller_pc,
                              int c, FILE *stream, icount_shadow stream_label,
                              icount_shadow *ret_label)

int __icountw_getc_unlocked(FILE *stream, icount_shadow stream_label,
                            icount_shadow *ret_label) {
  int c = getc_unlocked(stream);
  *ret_label = 0;

  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getc_unlocked, GET_CALLER_PC(), c,
                             stream, stream_label, ret_label);

  return c;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getchar, uptr caller_pc, int c,
                              icount_shadow *ret_label)

int __icountw_getchar(icount_shadow *ret_label) {
  int c = getchar();
  *ret_label = 0;

  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getchar, GET_CALLER_PC(), c,
                             ret_label);

  return c;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getchar_unlocked, uptr caller_pc,
                              int c, icount_shadow *ret_label)

int __icountw_getchar_unlocked(icount_shadow *ret_label) {
  int c = getchar_unlocked();
  *ret_label = 0;

  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getchar_unlocked, GET_CALLER_PC(),
                             c, ret_label);

  return c;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fgets, uptr caller_pc, char *ret,
                              char *str, int count, FILE *fd,
                              icount_shadow str_label,
                              icount_shadow count_label, icount_shadow fd_label,
                              icount_shadow *ret_label)

SANITIZER_INTERFACE_ATTRIBUTE char *__icountw_fgets(char *s, int size,
                                                    FILE *stream,
                                                    icount_shadow s_label,
                                                    icount_shadow size_label,
                                                    icount_shadow stream_label,
                                                    icount_shadow *ret_label) {
  char *ret = fgets(s, size, stream);
  if (ret) {
    icount_disable_range_shadow(ret, strlen(ret) + 1);
    *ret_label = s_label;
  } else {
    *ret_label = 0;
  }

  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fgets, GET_CALLER_PC(), ret, s,
                             size, stream, s_label, size_label, stream_label,
                             ret_label);

  return ret;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fgets_unlocked, uptr caller_pc,
                              char *ret, char *str, int count, FILE *fd,
                              icount_shadow str_label,
                              icount_shadow count_label, icount_shadow fd_label,
                              icount_shadow *ret_label)

SANITIZER_INTERFACE_ATTRIBUTE char *__icountw_fgets_unlocked(
    char *s, int size, FILE *stream, icount_shadow s_label,
    icount_shadow size_label, icount_shadow stream_label,
    icount_shadow *ret_label) {
  char *ret = fgets(s, size, stream);
  if (ret) {
    icount_disable_range_shadow(ret, strlen(ret) + 1);
    *ret_label = s_label;
  } else {
    *ret_label = 0;
  }

  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_fgets, GET_CALLER_PC(), ret, s,
                             size, stream, s_label, size_label, stream_label,
                             ret_label);

  return ret;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getline, uptr caller_pc,
                              ssize_t ret, char **lineptr, size_t *n, FILE *fd,
                              icount_shadow buf_label, icount_shadow size_label,
                              icount_shadow fd_label, icount_shadow *ret_label)

// ssize_t getline(char **lineptr, size_t *n, FILE *stream);
SANITIZER_INTERFACE_ATTRIBUTE ssize_t
__icountw_getline(char **lineptr, size_t *n, FILE *fd, icount_shadow buf_label,
                  icount_shadow size_label, icount_shadow fd_label,
                  icount_shadow *ret_label) {
  ssize_t ret = getline(lineptr, n, fd);
  if (ret > 0)
    icount_disable_range_shadow(*lineptr, ret + 1);

  *ret_label = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getline, GET_CALLER_PC(), ret,
                             lineptr, n, fd, buf_label, size_label, fd_label,
                             ret_label);

  return ret;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getdelim, uptr caller_pc,
                              ssize_t ret, char **lineptr, size_t *n, int delim,
                              FILE *fd, icount_shadow buf_label,
                              icount_shadow size_label,
                              icount_shadow delim_label, icount_shadow fd_label,
                              icount_shadow *ret_label)

// ssize_t getdelim(char **lineptr, size_t *n, int delim, FILE *stream);
SANITIZER_INTERFACE_ATTRIBUTE ssize_t __icountw_getdelim(
    char **lineptr, size_t *n, int delim, FILE *fd, icount_shadow buf_label,
    icount_shadow size_label, icount_shadow delim_label, icount_shadow fd_label,
    icount_shadow *ret_label) {
  ssize_t ret = getdelim(lineptr, n, delim, fd);
  if (ret > 0)
    icount_disable_range_shadow(*lineptr, ret + 1);

  *ret_label = 0;
  CALL_WEAK_INTERCEPTOR_HOOK(icount_weak_hook_getdelim, GET_CALLER_PC(), ret,
                             lineptr, n, delim, fd, buf_label, size_label,
                             delim_label, fd_label, ret_label);
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_stat(const char *path,
                                                 struct stat *buf,
                                                 icount_shadow path_label,
                                                 icount_shadow buf_label,
                                                 icount_shadow *ret_label) {
  int ret = stat(path, buf);
  if (ret == 0)
    icount_disable_range_shadow(buf, sizeof(struct stat));
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_fstat(int fd, struct stat *buf,
                                                  icount_shadow fd_label,
                                                  icount_shadow buf_label,
                                                  icount_shadow *ret_label) {
  int ret = fstat(fd, buf);
  if (ret == 0)
    icount_disable_range_shadow(buf, sizeof(struct stat));
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE char *__icountw_strchr(const char *s, int c,
                                                     icount_shadow s_label,
                                                     icount_shadow c_label,
                                                     icount_shadow *ret_label) {
  for (size_t i = 0;; ++i) {
    if (s[i] == c || s[i] == 0) {
      *ret_label =
          icount_combine_shadows(icount_get_range_shadow(s, i + 1),
                                 icount_combine_shadows(s_label, c_label));
      return s[i] == 0 ? nullptr : const_cast<char *>(s + i);
    }
  }
}

DECLARE_WEAK_INTERCEPTOR_HOOK(dfsan_weak_hook_memcmp, uptr caller_pc,
                              const void *s1, const void *s2, size_t n,
                              icount_shadow s1_label, icount_shadow s2_label,
                              icount_shadow n_label)

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_memcmp(
    const void *s1, const void *s2, size_t n, icount_shadow s1_label,
    icount_shadow s2_label, icount_shadow n_label, icount_shadow *ret_label) {
  CALL_WEAK_INTERCEPTOR_HOOK(dfsan_weak_hook_memcmp, GET_CALLER_PC(), s1, s2, n,
                             s1_label, s2_label, n_label);
  const char *cs1 = (const char *)s1, *cs2 = (const char *)s2;
  for (size_t i = 0; i != n; ++i) {
    if (cs1[i] != cs2[i]) {
      *ret_label = icount_combine_shadows(icount_get_range_shadow(cs1, i + 1),
                                          icount_get_range_shadow(cs2, i + 1));
      return cs1[i] - cs2[i];
    }
  }

  *ret_label = icount_combine_shadows(icount_get_range_shadow(cs1, n),
                                      icount_get_range_shadow(cs2, n));
  return 0;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(dfsan_weak_hook_strcmp, uptr caller_pc,
                              const char *s1, const char *s2,
                              icount_shadow s1_label, icount_shadow s2_label)

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_strcmp(const char *s1,
                                                   const char *s2,
                                                   icount_shadow s1_label,
                                                   icount_shadow s2_label,
                                                   icount_shadow *ret_label) {
  CALL_WEAK_INTERCEPTOR_HOOK(dfsan_weak_hook_strcmp, GET_CALLER_PC(), s1, s2,
                             s1_label, s2_label);
  for (size_t i = 0;; ++i) {
    if (s1[i] != s2[i] || s1[i] == 0 || s2[i] == 0) {
      *ret_label = icount_combine_shadows(icount_get_range_shadow(s1, i + 1),
                                          icount_get_range_shadow(s2, i + 1));
      return s1[i] - s2[i];
    }
  }
  return 0;
}

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_strcasecmp(
    const char *s1, const char *s2, icount_shadow s1_label,
    icount_shadow s2_label, icount_shadow *ret_label) {
  for (size_t i = 0;; ++i) {
    if (tolower(s1[i]) != tolower(s2[i]) || s1[i] == 0 || s2[i] == 0) {
      *ret_label = icount_combine_shadows(icount_get_range_shadow(s1, i + 1),
                                          icount_get_range_shadow(s2, i + 1));
      return s1[i] - s2[i];
    }
  }
  return 0;
}

DECLARE_WEAK_INTERCEPTOR_HOOK(dfsan_weak_hook_strncmp, uptr caller_pc,
                              const char *s1, const char *s2, size_t n,
                              icount_shadow s1_label, icount_shadow s2_label,
                              icount_shadow n_label)

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_strncmp(
    const char *s1, const char *s2, size_t n, icount_shadow s1_label,
    icount_shadow s2_label, icount_shadow n_label, icount_shadow *ret_label) {
  if (n == 0) {
    *ret_label = 0;
    return 0;
  }

  CALL_WEAK_INTERCEPTOR_HOOK(dfsan_weak_hook_strncmp, GET_CALLER_PC(), s1, s2,
                             n, s1_label, s2_label, n_label);

  for (size_t i = 0;; ++i) {
    if (s1[i] != s2[i] || s1[i] == 0 || s2[i] == 0 || i == n - 1) {
      *ret_label = icount_combine_shadows(icount_get_range_shadow(s1, i + 1),
                                          icount_get_range_shadow(s2, i + 1));
      return s1[i] - s2[i];
    }
  }
  return 0;
}

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_strncasecmp(
    const char *s1, const char *s2, size_t n, icount_shadow s1_label,
    icount_shadow s2_label, icount_shadow n_label, icount_shadow *ret_label) {
  if (n == 0) {
    *ret_label = 0;
    return 0;
  }

  for (size_t i = 0;; ++i) {
    if (tolower(s1[i]) != tolower(s2[i]) || s1[i] == 0 || s2[i] == 0 ||
        i == n - 1) {
      *ret_label = icount_combine_shadows(icount_get_range_shadow(s1, i + 1),
                                          icount_get_range_shadow(s2, i + 1));
      return s1[i] - s2[i];
    }
  }

  return 0;
}

SANITIZER_INTERFACE_ATTRIBUTE void *__icountw_calloc(size_t nmemb, size_t size,
                                                     icount_shadow nmemb_label,
                                                     icount_shadow size_label,
                                                     icount_shadow *ret_label) {
  void *p = calloc(nmemb, size);
  icount_disable_range_shadow(p, nmemb * size);
  *ret_label = 0;
  return p;
}

SANITIZER_INTERFACE_ATTRIBUTE size_t __icountw_strlen(
    const char *s, icount_shadow s_label, icount_shadow *ret_label) {
  size_t ret = strlen(s);
  *ret_label = icount_get_range_shadow(s, ret + 1);
  return ret;
}

static void *dfsan_memcpy(void *dest, const void *src, size_t n) {
  icount_shadow *sdest = shadow_for(dest);
  const icount_shadow *ssrc = shadow_for(src);
  internal_memcpy((void *)sdest, (const void *)ssrc, n * sizeof(icount_shadow));
  return internal_memcpy(dest, src, n);
}

static void dfsan_memset(void *s, int c, icount_shadow c_label, size_t n) {
  internal_memset(s, c, n);
  icount_set_range_shadow(c_label, s, n);
}

SANITIZER_INTERFACE_ATTRIBUTE
void *__icountw_memcpy(void *dest, const void *src, size_t n,
                       icount_shadow dest_label, icount_shadow src_label,
                       icount_shadow n_label, icount_shadow *ret_label) {
  *ret_label = dest_label;
  return dfsan_memcpy(dest, src, n);
}

SANITIZER_INTERFACE_ATTRIBUTE
void *__icountw_memset(void *s, int c, size_t n, icount_shadow s_label,
                       icount_shadow c_label, icount_shadow n_label,
                       icount_shadow *ret_label) {
  dfsan_memset(s, c, c_label, n);
  *ret_label = s_label;
  return s;
}

SANITIZER_INTERFACE_ATTRIBUTE char *__icountw_strdup(const char *s,
                                                     icount_shadow s_label,
                                                     icount_shadow *ret_label) {
  size_t len = strlen(s);
  void *p = malloc(len + 1);
  dfsan_memcpy(p, s, len + 1);
  *ret_label = 0;
  return static_cast<char *>(p);
}

SANITIZER_INTERFACE_ATTRIBUTE char *__icountw_strncpy(
    char *s1, const char *s2, size_t n, icount_shadow s1_label,
    icount_shadow s2_label, icount_shadow n_label, icount_shadow *ret_label) {
  size_t len = strlen(s2);
  if (len < n) {
    dfsan_memcpy(s1, s2, len + 1);
    dfsan_memset(s1 + len + 1, 0, 0, n - len - 1);
  } else {
    dfsan_memcpy(s1, s2, n);
  }

  *ret_label = s1_label;
  return s1;
}

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_clock_gettime(
    clockid_t clk_id, struct timespec *tp, icount_shadow clk_id_label,
    icount_shadow tp_label, icount_shadow *ret_label) {
  int ret = clock_gettime(clk_id, tp);
  if (ret == 0)
    icount_disable_range_shadow(tp, sizeof(struct timespec));
  *ret_label = 0;
  return ret;
}

static void unpoison(const void *ptr, uptr size) {
  icount_disable_range_shadow(const_cast<void *>(ptr), size);
}

// dlopen() ultimately calls mmap() down inside the loader, which generally
// doesn't participate in dynamic symbol resolution.  Therefore we won't
// intercept its calls to mmap, and we have to hook it here.
SANITIZER_INTERFACE_ATTRIBUTE void *__icountw_dlopen(
    const char *filename, int flag, icount_shadow filename_label,
    icount_shadow flag_label, icount_shadow *ret_label) {
  void *handle = dlopen(filename, flag);
  link_map *map = GET_LINK_MAP_BY_DLOPEN_HANDLE(handle);
  if (map)
    ForEachMappedRegion(map, unpoison);
  *ret_label = 0;
  return handle;
}

struct pthread_create_info {
  void *(*start_routine_trampoline)(void *, void *, icount_shadow,
                                    icount_shadow *);
  void *start_routine;
  void *arg;
};

static void *pthread_create_cb(void *p) {
  pthread_create_info pci(*(pthread_create_info *)p);
  free(p);
  icount_shadow ret_label;
  return pci.start_routine_trampoline(pci.start_routine, pci.arg, 0,
                                      &ret_label);
}

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_pthread_create(
    pthread_t *thread, const pthread_attr_t *attr,
    void *(*start_routine_trampoline)(void *, void *, icount_shadow,
                                      icount_shadow *),
    void *start_routine, void *arg, icount_shadow thread_label,
    icount_shadow attr_label, icount_shadow start_routine_label,
    icount_shadow arg_label, icount_shadow *ret_label) {
  pthread_create_info *pci =
      (pthread_create_info *)malloc(sizeof(pthread_create_info));
  pci->start_routine_trampoline = start_routine_trampoline;
  pci->start_routine = start_routine;
  pci->arg = arg;
  int rv = pthread_create(thread, attr, pthread_create_cb, (void *)pci);
  if (rv != 0)
    free(pci);
  *ret_label = 0;
  return rv;
}

struct dl_iterate_phdr_info {
  int (*callback_trampoline)(void *callback, struct dl_phdr_info *info,
                             size_t size, void *data, icount_shadow info_label,
                             icount_shadow size_label, icount_shadow data_label,
                             icount_shadow *ret_label);
  void *callback;
  void *data;
};

int dl_iterate_phdr_cb(struct dl_phdr_info *info, size_t size, void *data) {
  dl_iterate_phdr_info *dipi = (dl_iterate_phdr_info *)data;
  icount_disable_shadow(*info);
  icount_disable_range_shadow(const_cast<char *>(info->dlpi_name),
                              strlen(info->dlpi_name) + 1);
  icount_disable_range_shadow(
      const_cast<char *>(reinterpret_cast<const char *>(info->dlpi_phdr)),
      sizeof(*info->dlpi_phdr) * info->dlpi_phnum);
  icount_shadow ret_label;
  return dipi->callback_trampoline(dipi->callback, info, size, dipi->data, 0, 0,
                                   0, &ret_label);
}

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_dl_iterate_phdr(
    int (*callback_trampoline)(
        void *callback, struct dl_phdr_info *info, size_t size, void *data,
        icount_shadow info_label, icount_shadow size_label,
        icount_shadow data_label, icount_shadow *ret_label),
    void *callback, void *data, icount_shadow callback_label,
    icount_shadow data_label, icount_shadow *ret_label) {
  dl_iterate_phdr_info dipi = {callback_trampoline, callback, data};
  *ret_label = 0;
  return dl_iterate_phdr(dl_iterate_phdr_cb, &dipi);
}

SANITIZER_INTERFACE_ATTRIBUTE
char *__icountw_ctime_r(const time_t *timep, char *buf,
                        icount_shadow timep_label, icount_shadow buf_label,
                        icount_shadow *ret_label) {
  char *ret = ctime_r(timep, buf);
  if (ret) {
    icount_set_range_shadow(icount_get_range_shadow(timep, sizeof(time_t)), buf,
                            strlen(buf) + 1);
    *ret_label = buf_label;
  } else {
    *ret_label = 0;
  }
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
char *__icountw_getcwd(char *buf, size_t size, icount_shadow buf_label,
                       icount_shadow size_label, icount_shadow *ret_label) {
  char *ret = getcwd(buf, size);
  if (ret) {
    icount_disable_range_shadow(ret, strlen(ret) + 1);
    *ret_label = buf_label;
  } else {
    *ret_label = 0;
  }
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
char *__icountw_get_current_dir_name(icount_shadow *ret_label) {
  char *ret = get_current_dir_name();
  if (ret) {
    icount_disable_range_shadow(ret, strlen(ret) + 1);
  }
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_gethostname(char *name, size_t len, icount_shadow name_label,
                          icount_shadow len_label, icount_shadow *ret_label) {
  int ret = gethostname(name, len);
  if (ret == 0) {
    icount_disable_range_shadow(name, strlen(name) + 1);
  }
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_getrlimit(int resource, struct rlimit *rlim,
                        icount_shadow resource_label, icount_shadow rlim_label,
                        icount_shadow *ret_label) {
  int ret = getrlimit(resource, rlim);
  if (ret == 0) {
    icount_disable_range_shadow(rlim, sizeof(struct rlimit));
  }
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_getrusage(int who, struct rusage *usage, icount_shadow who_label,
                        icount_shadow usage_label, icount_shadow *ret_label) {
  int ret = getrusage(who, usage);
  if (ret == 0) {
    icount_disable_range_shadow(usage, sizeof(struct rusage));
  }
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
char *__icountw_strcpy(char *dest, const char *src, icount_shadow dst_label,
                       icount_shadow src_label, icount_shadow *ret_label) {
  char *ret = strcpy(dest, src);
  if (ret) {
    internal_memcpy(shadow_for(dest), shadow_for(src),
                    sizeof(icount_shadow) * (strlen(src) + 1));
  }
  *ret_label = dst_label;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
long int __icountw_strtol(const char *nptr, char **endptr, int base,
                          icount_shadow nptr_label, icount_shadow endptr_label,
                          icount_shadow base_label, icount_shadow *ret_label) {
  char *tmp_endptr;
  long int ret = strtol(nptr, &tmp_endptr, base);
  if (endptr) {
    *endptr = tmp_endptr;
  }
  if (tmp_endptr > nptr) {
    // If *tmp_endptr is '\0' include its label as well.
    *ret_label = icount_combine_shadows(
        base_label, icount_get_range_shadow(
                        nptr, tmp_endptr - nptr + (*tmp_endptr ? 0 : 1)));
  } else {
    *ret_label = 0;
  }
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
double __icountw_strtod(const char *nptr, char **endptr,
                        icount_shadow nptr_label, icount_shadow endptr_label,
                        icount_shadow *ret_label) {
  char *tmp_endptr;
  double ret = strtod(nptr, &tmp_endptr);
  if (endptr) {
    *endptr = tmp_endptr;
  }
  if (tmp_endptr > nptr) {
    // If *tmp_endptr is '\0' include its label as well.
    *ret_label = icount_get_range_shadow(
        nptr, tmp_endptr - nptr + (*tmp_endptr ? 0 : 1));
  } else {
    *ret_label = 0;
  }
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
long long int __icountw_strtoll(const char *nptr, char **endptr, int base,
                                icount_shadow nptr_label,
                                icount_shadow endptr_label,
                                icount_shadow base_label,
                                icount_shadow *ret_label) {
  char *tmp_endptr;
  long long int ret = strtoll(nptr, &tmp_endptr, base);
  if (endptr) {
    *endptr = tmp_endptr;
  }
  if (tmp_endptr > nptr) {
    // If *tmp_endptr is '\0' include its label as well.
    *ret_label = icount_combine_shadows(
        base_label, icount_get_range_shadow(
                        nptr, tmp_endptr - nptr + (*tmp_endptr ? 0 : 1)));
  } else {
    *ret_label = 0;
  }
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
unsigned long int __icountw_strtoul(const char *nptr, char **endptr, int base,
                                    icount_shadow nptr_label,
                                    icount_shadow endptr_label,
                                    icount_shadow base_label,
                                    icount_shadow *ret_label) {
  char *tmp_endptr;
  unsigned long int ret = strtoul(nptr, &tmp_endptr, base);
  if (endptr) {
    *endptr = tmp_endptr;
  }
  if (tmp_endptr > nptr) {
    // If *tmp_endptr is '\0' include its label as well.
    *ret_label = icount_combine_shadows(
        base_label, icount_get_range_shadow(
                        nptr, tmp_endptr - nptr + (*tmp_endptr ? 0 : 1)));
  } else {
    *ret_label = 0;
  }
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
long long unsigned int __icountw_strtoull(const char *nptr, char **endptr,
                                          icount_shadow nptr_label, int base,
                                          icount_shadow endptr_label,
                                          icount_shadow base_label,
                                          icount_shadow *ret_label) {
  char *tmp_endptr;
  long long unsigned int ret = strtoull(nptr, &tmp_endptr, base);
  if (endptr) {
    *endptr = tmp_endptr;
  }
  if (tmp_endptr > nptr) {
    // If *tmp_endptr is '\0' include its label as well.
    *ret_label = icount_combine_shadows(
        base_label, icount_get_range_shadow(
                        nptr, tmp_endptr - nptr + (*tmp_endptr ? 0 : 1)));
  } else {
    *ret_label = 0;
  }
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
time_t __icountw_time(time_t *t, icount_shadow t_label,
                      icount_shadow *ret_label) {
  time_t ret = time(t);
  if (ret != (time_t)-1 && t) {
    icount_disable_range_shadow(t, sizeof(time_t));
  }
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_inet_pton(int af, const char *src, void *dst,
                        icount_shadow af_label, icount_shadow src_label,
                        icount_shadow dst_label, icount_shadow *ret_label) {
  int ret = inet_pton(af, src, dst);
  if (ret == 1) {
    icount_set_range_shadow(
        icount_get_range_shadow(src, strlen(src) + 1), dst,
        af == AF_INET ? sizeof(struct in_addr) : sizeof(in6_addr));
  }
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
struct tm *__icountw_localtime_r(const time_t *timep, struct tm *result,
                                 icount_shadow timep_label,
                                 icount_shadow result_label,
                                 icount_shadow *ret_label) {
  struct tm *ret = localtime_r(timep, result);
  if (ret) {
    icount_set_range_shadow(icount_get_range_shadow(timep, sizeof(time_t)),
                            result, sizeof(struct tm));
    *ret_label = result_label;
  } else {
    *ret_label = 0;
  }
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_getpwuid_r(id_t uid, struct passwd *pwd, char *buf, size_t buflen,
                         struct passwd **result, icount_shadow uid_label,
                         icount_shadow pwd_label, icount_shadow buf_label,
                         icount_shadow buflen_label, icount_shadow result_label,
                         icount_shadow *ret_label) {
  // Store the data in pwd, the strings referenced from pwd in buf, and the
  // address of pwd in *result.  On failure, NULL is stored in *result.
  int ret = getpwuid_r(uid, pwd, buf, buflen, result);
  if (ret == 0) {
    icount_disable_range_shadow(pwd, sizeof(struct passwd));
    icount_disable_range_shadow(buf, strlen(buf) + 1);
  }
  *ret_label = 0;
  icount_disable_range_shadow(result, sizeof(struct passwd *));
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_poll(struct pollfd *fds, nfds_t nfds, int timeout,
                   icount_shadow dfs_label, icount_shadow nfds_label,
                   icount_shadow timeout_label, icount_shadow *ret_label) {
  int ret = poll(fds, nfds, timeout);
  if (ret >= 0) {
    for (; nfds > 0; --nfds) {
      icount_disable_range_shadow(&fds[nfds - 1].revents,
                                  sizeof(fds[nfds - 1].revents));
    }
  }
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_select(int nfds, fd_set *readfds, fd_set *writefds,
                     fd_set *exceptfds, struct timeval *timeout,
                     icount_shadow nfds_label, icount_shadow readfds_label,
                     icount_shadow writefds_label,
                     icount_shadow exceptfds_label, icount_shadow timeout_label,
                     icount_shadow *ret_label) {
  int ret = select(nfds, readfds, writefds, exceptfds, timeout);
  // Clear everything (also on error) since their content is either set or
  // undefined.
  if (readfds) {
    icount_disable_range_shadow(readfds, sizeof(fd_set));
  }
  if (writefds) {
    icount_disable_range_shadow(writefds, sizeof(fd_set));
  }
  if (exceptfds) {
    icount_disable_range_shadow(exceptfds, sizeof(fd_set));
  }
  icount_disable_range_shadow(timeout, sizeof(struct timeval));
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_sched_getaffinity(pid_t pid, size_t cpusetsize, cpu_set_t *mask,
                                icount_shadow pid_label,
                                icount_shadow cpusetsize_label,
                                icount_shadow mask_label,
                                icount_shadow *ret_label) {
  int ret = sched_getaffinity(pid, cpusetsize, mask);
  if (ret == 0) {
    icount_disable_range_shadow(mask, cpusetsize);
  }
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_sigemptyset(sigset_t *set, icount_shadow set_label,
                          icount_shadow *ret_label) {
  int ret = sigemptyset(set);
  icount_disable_range_shadow(set, sizeof(sigset_t));
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_sigaction(int signum, const struct sigaction *act,
                        struct sigaction *oldact, icount_shadow signum_label,
                        icount_shadow act_label, icount_shadow oldact_label,
                        icount_shadow *ret_label) {
  int ret = sigaction(signum, act, oldact);
  if (oldact) {
    icount_disable_range_shadow(oldact, sizeof(struct sigaction));
  }
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_gettimeofday(struct timeval *tv, struct timezone *tz,
                           icount_shadow tv_label, icount_shadow tz_label,
                           icount_shadow *ret_label) {
  int ret = gettimeofday(tv, tz);
  if (tv) {
    icount_disable_range_shadow(tv, sizeof(struct timeval));
  }
  if (tz) {
    icount_disable_range_shadow(tz, sizeof(struct timezone));
  }
  *ret_label = 0;
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE void *__icountw_memchr(void *s, int c, size_t n,
                                                     icount_shadow s_label,
                                                     icount_shadow c_label,
                                                     icount_shadow n_label,
                                                     icount_shadow *ret_label) {
  void *ret = memchr(s, c, n);
  size_t len =
      ret ? reinterpret_cast<char *>(ret) - reinterpret_cast<char *>(s) + 1 : n;
  *ret_label = icount_combine_shadows(icount_get_range_shadow(s, len),
                                      icount_combine_shadows(s_label, c_label));
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE char *__icountw_strrchr(
    char *s, int c, icount_shadow s_label, icount_shadow c_label,
    icount_shadow *ret_label) {
  char *ret = strrchr(s, c);
  *ret_label = icount_combine_shadows(icount_get_range_shadow(s, strlen(s) + 1),
                                      icount_combine_shadows(s_label, c_label));

  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE char *__icountw_strstr(
    char *haystack, char *needle, icount_shadow haystack_label,
    icount_shadow needle_label, icount_shadow *ret_label) {
  char *ret = strstr(haystack, needle);
  size_t len = ret ? ret + strlen(needle) - haystack : strlen(haystack) + 1;
  *ret_label = icount_combine_shadows(
      icount_get_range_shadow(haystack, len),
      icount_combine_shadows(
          icount_get_range_shadow(needle, strlen(needle) + 1),
          icount_combine_shadows(haystack_label, needle_label)));

  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_nanosleep(
    const struct timespec *req, struct timespec *rem, icount_shadow req_label,
    icount_shadow rem_label, icount_shadow *ret_label) {
  int ret = nanosleep(req, rem);
  *ret_label = 0;
  if (ret == -1) {
    // Interrupted by a signal, rem is filled with the remaining time.
    icount_disable_range_shadow(rem, sizeof(struct timespec));
  }
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_socketpair(
    int domain, int type, int protocol, int sv[2], icount_shadow domain_label,
    icount_shadow type_label, icount_shadow protocol_label,
    icount_shadow sv_label, icount_shadow *ret_label) {
  int ret = socketpair(domain, type, protocol, sv);
  *ret_label = 0;
  if (ret == 0) {
    icount_disable_range_shadow(sv, sizeof(*sv) * 2);
  }
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE int __icountw_write(int fd, const void *buf,
                                                  size_t count,
                                                  icount_shadow fd_label,
                                                  icount_shadow buf_label,
                                                  icount_shadow count_label,
                                                  icount_shadow *ret_label) {
  *ret_label = 0;
  return write(fd, buf, count);
}
}  // namespace __icount

// Type used to extract a icount_shadow with va_arg()
typedef int icount_shadow_va;

// Formats a chunk either a constant string or a single format directive (e.g.,
// '%.3f').
struct Formatter {
  Formatter(char *str_, const char *fmt_, size_t size_)
      : str(str_),
        str_off(0),
        size(size_),
        fmt_start(fmt_),
        fmt_cur(fmt_),
        width(-1) {}

  int format() {
    char *tmp_fmt = build_format_string();
    int retval = snprintf(str + str_off, str_off < size ? size - str_off : 0,
                          tmp_fmt, 0 /* used only to avoid warnings */);
    free(tmp_fmt);
    return retval;
  }

  template <typename T>
  int format(T arg) {
    char *tmp_fmt = build_format_string();
    int retval;
    if (width >= 0) {
      retval = snprintf(str + str_off, str_off < size ? size - str_off : 0,
                        tmp_fmt, width, arg);
    } else {
      retval = snprintf(str + str_off, str_off < size ? size - str_off : 0,
                        tmp_fmt, arg);
    }
    free(tmp_fmt);
    return retval;
  }

  char *build_format_string() {
    size_t fmt_size = fmt_cur - fmt_start + 1;
    char *new_fmt = (char *)malloc(fmt_size + 1);
    assert(new_fmt);
    internal_memcpy(new_fmt, fmt_start, fmt_size);
    new_fmt[fmt_size] = '\0';
    return new_fmt;
  }

  char *str_cur() { return str + str_off; }

  size_t num_written_bytes(int retval) {
    if (retval < 0) {
      return 0;
    }

    size_t num_avail = str_off < size ? size - str_off : 0;
    if (num_avail == 0) {
      return 0;
    }

    size_t num_written = retval;
    // A return value of {v,}snprintf of size or more means that the output was
    // truncated.
    if (num_written >= num_avail) {
      num_written -= num_avail;
    }

    return num_written;
  }

  char *str;
  size_t str_off;
  size_t size;
  const char *fmt_start;
  const char *fmt_cur;
  int width;
};

// Formats the input and propagates the input labels to the output. The output
// is stored in 'str'. 'size' bounds the number of output bytes. 'format' and
// 'ap' are the format string and the list of arguments for formatting. Returns
// the return value vsnprintf would return.
//
// The function tokenizes the format string in chunks representing either a
// constant string or a single format directive (e.g., '%.3f') and formats each
// chunk independently into the output string. This approach allows to figure
// out which bytes of the output string depends on which argument and thus to
// propagate labels more precisely.
//
// WARNING: This implementation does not support conversion specifiers with
// positional arguments.
static int format_buffer(char *str, size_t size, const char *fmt,
                         icount_shadow *va_labels, icount_shadow *ret_label,
                         va_list ap) {
  Formatter formatter(str, fmt, size);

  while (*formatter.fmt_cur) {
    formatter.fmt_start = formatter.fmt_cur;
    formatter.width = -1;
    int retval = 0;

    if (*formatter.fmt_cur != '%') {
      // Ordinary character. Consume all the characters until a '%' or the end
      // of the string.
      for (; *(formatter.fmt_cur + 1) && *(formatter.fmt_cur + 1) != '%';
           ++formatter.fmt_cur) {
      }
      retval = formatter.format();
      icount_disable_range_shadow(formatter.str_cur(),
                                  formatter.num_written_bytes(retval));
    } else {
      // Conversion directive. Consume all the characters until a conversion
      // specifier or the end of the string.
      bool end_fmt = false;
      for (; *formatter.fmt_cur && !end_fmt;) {
        switch (*++formatter.fmt_cur) {
          case 'd':
          case 'i':
          case 'o':
          case 'u':
          case 'x':
          case 'X':
            switch (*(formatter.fmt_cur - 1)) {
              case 'h':
                // Also covers the 'hh' case (since the size of the arg is still
                // an int).
                retval = formatter.format(va_arg(ap, int));
                break;
              case 'l':
                if (formatter.fmt_cur - formatter.fmt_start >= 2 &&
                    *(formatter.fmt_cur - 2) == 'l') {
                  retval = formatter.format(va_arg(ap, long long int));
                } else {
                  retval = formatter.format(va_arg(ap, long int));
                }
                break;
              case 'q':
                retval = formatter.format(va_arg(ap, long long int));
                break;
              case 'j':
                retval = formatter.format(va_arg(ap, intmax_t));
                break;
              case 'z':
              case 't':
                retval = formatter.format(va_arg(ap, size_t));
                break;
              default:
                retval = formatter.format(va_arg(ap, int));
            }
            icount_set_range_shadow(*va_labels++, formatter.str_cur(),
                                    formatter.num_written_bytes(retval));
            end_fmt = true;
            break;

          case 'a':
          case 'A':
          case 'e':
          case 'E':
          case 'f':
          case 'F':
          case 'g':
          case 'G':
            if (*(formatter.fmt_cur - 1) == 'L') {
              retval = formatter.format(va_arg(ap, long double));
            } else {
              retval = formatter.format(va_arg(ap, double));
            }
            icount_set_range_shadow(*va_labels++, formatter.str_cur(),
                                    formatter.num_written_bytes(retval));
            end_fmt = true;
            break;

          case 'c':
            retval = formatter.format(va_arg(ap, int));
            icount_set_range_shadow(*va_labels++, formatter.str_cur(),
                                    formatter.num_written_bytes(retval));
            end_fmt = true;
            break;

          case 's': {
            char *arg = va_arg(ap, char *);
            retval = formatter.format(arg);
            va_labels++;
            internal_memcpy(
                shadow_for(formatter.str_cur()), shadow_for(arg),
                sizeof(icount_shadow) * formatter.num_written_bytes(retval));
            end_fmt = true;
            break;
          }

          case 'p':
            retval = formatter.format(va_arg(ap, void *));
            icount_set_range_shadow(*va_labels++, formatter.str_cur(),
                                    formatter.num_written_bytes(retval));
            end_fmt = true;
            break;

          case 'n': {
            int *ptr = va_arg(ap, int *);
            *ptr = (int)formatter.str_off;
            va_labels++;
            icount_disable_range_shadow(ptr, sizeof(ptr));
            end_fmt = true;
            break;
          }

          case '%':
            retval = formatter.format();
            icount_disable_range_shadow(formatter.str_cur(),
                                        formatter.num_written_bytes(retval));
            end_fmt = true;
            break;

          case '*':
            formatter.width = va_arg(ap, int);
            va_labels++;
            break;

          default:
            break;
        }
      }
    }

    if (retval < 0) {
      return retval;
    }

    formatter.fmt_cur++;
    formatter.str_off += retval;
  }

  *ret_label = 0;

  // Number of bytes written in total.
  return formatter.str_off;
}

extern "C" {
SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_sprintf(char *str, const char *format, icount_shadow str_label,
                      icount_shadow format_label, icount_shadow *va_labels,
                      icount_shadow *ret_label, ...) {
  va_list ap;
  va_start(ap, ret_label);
  int ret = format_buffer(str, ~0ul, format, va_labels, ret_label, ap);
  va_end(ap);
  return ret;
}

SANITIZER_INTERFACE_ATTRIBUTE
int __icountw_snprintf(char *str, size_t size, const char *format,
                       icount_shadow str_label, icount_shadow size_label,
                       icount_shadow format_label, icount_shadow *va_labels,
                       icount_shadow *ret_label, ...) {
  va_list ap;
  va_start(ap, ret_label);
  int ret = format_buffer(str, size, format, va_labels, ret_label, ap);
  va_end(ap);
  return ret;
}
}  // extern "C"
