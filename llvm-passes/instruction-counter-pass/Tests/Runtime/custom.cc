// RUN: %clangxx_icount %include_path %s %runtime_lib %load_blacklist -o %t
// RUN: %t
// RUN: %clangxx_icount -mllvm -dfsan-args-abi %include_path %s %runtime_lib %load_blacklist -o %t
// RUN: %t

// Tests custom implementations of various glibc functions.

#include <sanitizer/icount_interface.h>

#include <arpa/inet.h>
#include <assert.h>
#include <fcntl.h>
#include <link.h>
#include <poll.h>
#include <pthread.h>
#include <pwd.h>
#include <sched.h>
#include <signal.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/select.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#define ASSERT_DISABLED_SHADOW(data) \
  assert(0 == icount_get_shadow((long) (data)))

#define ASSERT_DISABLED_RANGE_SHADOW(ptr, size) \
  assert(0 == icount_get_range_shadow(ptr, size))

#define ASSERT_SHADOW(data, label) \
  assert(label == icount_get_shadow((long) (data)))

#define ASSERT_RANGE_SHADOW(ptr, size, label) \
  assert(label == icount_get_range_shadow(ptr, size))

void test_open() {
  int fd = 0;
  icount_enable_shadow(fd);

  fd = open("/etc/passwd", O_RDONLY);
  assert(fd > 0);

  ASSERT_DISABLED_SHADOW(fd);
  close(fd);
}

void test_fopen() {
  FILE* stream = NULL;
  icount_enable_shadow(stream);

  stream = fopen("/etc/passwd", "r");
  assert(stream);

  ASSERT_DISABLED_SHADOW(stream);
  fclose(stream);
}

void test_close() {
  int fd = open("/etc/passwd", O_RDONLY);
  assert(fd > 0);

  int ret = 0;
  icount_enable_shadow(ret);

  ret = close(fd);

  ASSERT_DISABLED_SHADOW(ret);
}

void test_fclose() {
  FILE *stream = fopen("/etc/passwd", "r");
  assert(stream);

  int ret = 0;
  icount_enable_shadow(ret);

  ret = fclose(stream);
  ASSERT_DISABLED_SHADOW(ret);
}

void test_mmap() {
  void *addr = mmap(NULL, getpagesize(), PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_ANONYMOUS, 0, 0);
  assert(addr != MAP_FAILED);
  int res = munmap(addr, 16);
  assert(res == 0);
  icount_enable_range_shadow(addr, 16);

  void *mapped_addr = NULL;
  icount_enable_shadow(mapped_addr);

  mapped_addr = mmap(addr, 16, PROT_READ | PROT_WRITE,
                     MAP_PRIVATE | MAP_FIXED | MAP_ANONYMOUS, -1, 0);
  assert(mapped_addr == addr);

  ASSERT_DISABLED_RANGE_SHADOW(mapped_addr, 16);
  ASSERT_DISABLED_SHADOW(mapped_addr);
  munmap(mapped_addr, 16);
}

void test_munmap() {
  void *addr = mmap(NULL, getpagesize(), PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_ANONYMOUS, 0, 0);
  assert(addr != MAP_FAILED);
  icount_enable_range_shadow(addr, 16);

  int res = 42;
  icount_enable_shadow(res);

  res = munmap(addr, getpagesize());
  assert(res == 0);

  ASSERT_DISABLED_RANGE_SHADOW(addr, 16);
  ASSERT_DISABLED_SHADOW(res);
}

void test_fread() {
  FILE *stream = fopen("/etc/passwd", "r");
  assert(stream);

  char buffer[16];
  icount_enable_range_shadow(buffer, 16);
  size_t res = 0;
  icount_enable_shadow(res);

  res = fread(buffer, 1, 16, stream);
  assert(res > 0);

  ASSERT_DISABLED_RANGE_SHADOW(buffer, 16);
  ASSERT_DISABLED_SHADOW(res);
  fclose(stream);
}

void test_fread_unlocked() {
  FILE *stream = fopen("/etc/passwd", "r");
  assert(stream);

  char buffer[16];
  icount_enable_range_shadow(buffer, 16);
  size_t res = 0;
  icount_enable_shadow(res);

  res = fread_unlocked(buffer, 1, 16, stream);
  assert(res > 0);

  ASSERT_DISABLED_RANGE_SHADOW(buffer, 16);
  ASSERT_DISABLED_SHADOW(res);
  fclose(stream);
}

void test_read() {
  int fd = open("/etc/passwd", O_RDONLY);
  assert(fd > 0);
  
  char buffer[16];
  icount_enable_range_shadow(buffer, 16);
  ssize_t res = 0;
  icount_enable_shadow(res);

  res = read(fd, buffer, 16);
  assert(res > 0);

  ASSERT_DISABLED_RANGE_SHADOW(buffer, 16);
  ASSERT_DISABLED_SHADOW(res);
  close(fd);
}

void test_pread() {
  int fd = open("/etc/passwd", O_RDONLY);
  assert(fd > 0);
  
  char buffer[16];
  icount_enable_range_shadow(buffer, 16);
  ssize_t res = 0;
  icount_enable_shadow(res);

  res = pread(fd, buffer, 16, 2);
  assert(res > 0);

  ASSERT_DISABLED_RANGE_SHADOW(buffer, 16);
  ASSERT_DISABLED_SHADOW(res);
  close(fd);
}

void test_fgetc() {
  FILE *stream = fopen("/etc/passwd", "r");
  assert(stream);

  int res = 0;
  icount_enable_shadow(res);

  res = fgetc(stream);

  ASSERT_DISABLED_SHADOW(res);
  fclose(stream);
}

void test_fgetc_unlocked() {
  FILE *stream = fopen("/etc/passwd", "r");
  assert(stream);

  int res = 0;
  icount_enable_shadow(res);

  res = fgetc_unlocked(stream);

  ASSERT_DISABLED_SHADOW(res);
  fclose(stream);
}

void test_fgets() {
  FILE *stream = fopen("/etc/passwd", "r");
  assert(stream);

  char* buffer = static_cast<char*>(malloc(16));
  icount_enable_range_shadow(buffer, 16);
  icount_enable_shadow(buffer);
  char *res = NULL;

  res = fgets(buffer, 16, stream);
  assert(res);

  ASSERT_DISABLED_RANGE_SHADOW(buffer, 16);
  ASSERT_SHADOW(res, 1);
  fclose(stream);
}

void test_fgets_unlocked() {
  FILE *stream = fopen("/etc/passwd", "r");
  assert(stream);

  char* buffer = static_cast<char*>(malloc(16));
  icount_enable_range_shadow(buffer, 16);
  icount_enable_shadow(buffer);
  char *res = NULL;

  res = fgets(buffer, 16, stream);
  assert(res);

  ASSERT_DISABLED_RANGE_SHADOW(buffer, 16);
  ASSERT_SHADOW(res, 1);
  fclose(stream);
}

void test_getline() {
  FILE *stream = fopen("/etc/passwd", "r");
  assert(stream);

  size_t size = 128;
  char* buffer = static_cast<char*>(malloc(size));
  icount_enable_range_shadow(buffer, size);
  ssize_t res = 0;

  res = getline(&buffer, &size, stream);
  assert(res > 0);

  ASSERT_DISABLED_RANGE_SHADOW(buffer, res);
  ASSERT_RANGE_SHADOW(buffer + res, size - res, 1);
  ASSERT_DISABLED_SHADOW(res);
  free(buffer);
  fclose(stream);
}

void test_getdelim() {
  FILE *stream = fopen("/etc/passwd", "r");
  assert(stream);

  size_t size = 128;
  char* buffer = static_cast<char*>(malloc(size));
  icount_enable_range_shadow(buffer, size);
  ssize_t res = 0;

  res = getdelim(&buffer, &size, '/', stream);
  assert(res > 0);

  ASSERT_DISABLED_RANGE_SHADOW(buffer, res);
  ASSERT_RANGE_SHADOW(buffer + res, size - res, 1);
  ASSERT_DISABLED_SHADOW(res);
  free(buffer);
  fclose(stream);
}

void test_stat() {
  int i = 1;
  icount_enable_range_shadow(&i, sizeof(i));

  struct stat s;
  s.st_dev = i;
  assert(0 == stat("/", &s));
  ASSERT_DISABLED_SHADOW(s.st_dev);

  s.st_dev = i;
  assert(-1 == stat("/nonexistent", &s));
  ASSERT_SHADOW(s.st_dev, 1);
}

void test_fstat() {
  int i = 1;
  icount_enable_range_shadow(&i, sizeof(i));

  struct stat s;
  int fd = open("/dev/zero", O_RDONLY);
  s.st_dev = i;
  int rv = fstat(fd, &s);
  assert(0 == rv);
  ASSERT_DISABLED_SHADOW(s.st_dev);
}

void test_memcmp() {
  char str1[] = "str1", str2[] = "str2";
  icount_enable_range_shadow(&str1[3], 1);
  icount_enable_range_shadow(&str2[3], 1);

  int rv = memcmp(str1, str2, sizeof(str1));
  assert(rv < 0);
  ASSERT_SHADOW(rv, 1);
}

void test_memcpy() {
  char str1[] = "str1";
  char str2[sizeof(str1)];
  icount_enable_range_shadow(&str1[3], 1);

  ASSERT_DISABLED_SHADOW(memcpy(str2, str1, sizeof(str1)));
  assert(0 == memcmp(str2, str1, sizeof(str1)));
  ASSERT_DISABLED_SHADOW(str2[0]);
  ASSERT_SHADOW(str2[3], 1);
}

void test_memset() {
  char buf[8];
  int j = 'a';
  icount_enable_range_shadow(&j, sizeof(j));

  ASSERT_DISABLED_SHADOW(memset(&buf, j, sizeof(buf)));
  for (int i = 0; i < 8; ++i) {
    ASSERT_SHADOW(buf[i], 1);
    assert(buf[i] == 'a');
  }
}

void test_strcmp() {
  char str1[] = "str1", str2[] = "str2";
  icount_enable_range_shadow(&str1[3], 1);
  icount_enable_range_shadow(&str2[3], 1);

  int rv = strcmp(str1, str2);
  assert(rv < 0);
  ASSERT_SHADOW(rv, 1);
}

void test_strlen() {
  char str1[] = "str1";
  icount_enable_range_shadow(&str1[3], 1);

  int rv = strlen(str1);
  assert(rv == 4);
  ASSERT_SHADOW(rv, 1);
}

void test_strdup() {
  char str1[] = "str1";
  icount_enable_range_shadow(&str1[3], 1);

  char *strd = strdup(str1);
  ASSERT_DISABLED_SHADOW(strd[0]);
  ASSERT_SHADOW(strd[3], 1);
  free(strd);
}

void test_strncpy() {
  char str1[] = "str1";
  char str2[sizeof(str1)];
  icount_enable_range_shadow(&str1[3], 1);

  char *strd = strncpy(str2, str1, 5);
  assert(strd == str2);
  assert(strcmp(str1, str2) == 0);
  ASSERT_DISABLED_SHADOW(strd);
  ASSERT_DISABLED_SHADOW(strd[0]);
  ASSERT_DISABLED_SHADOW(strd[1]);
  ASSERT_DISABLED_SHADOW(strd[2]);
  ASSERT_SHADOW(strd[3], 1);

  strd = strncpy(str2, str1, 3);
  assert(strd == str2);
  assert(strncmp(str1, str2, 3) == 0);
  ASSERT_DISABLED_SHADOW(strd);
  ASSERT_DISABLED_SHADOW(strd[0]);
  ASSERT_DISABLED_SHADOW(strd[1]);
  ASSERT_DISABLED_SHADOW(strd[2]);
}

void test_strncmp() {
  char str1[] = "str1", str2[] = "str2";
  icount_enable_range_shadow(&str1[3], 1);
  icount_enable_range_shadow(&str2[3], 1);

  int rv = strncmp(str1, str2, sizeof(str1));
  assert(rv < 0);
  ASSERT_SHADOW(rv, 1);

  rv = strncmp(str1, str2, 3);
  assert(rv == 0);
  ASSERT_DISABLED_SHADOW(rv);
}

void test_strcasecmp() {
  char str1[] = "str1", str2[] = "str2", str3[] = "Str1";
  icount_enable_range_shadow(&str1[3], 1);
  icount_enable_range_shadow(&str2[3], 1);
  icount_enable_range_shadow(&str3[2], 1);

  int rv = strcasecmp(str1, str2);
  assert(rv < 0);
  ASSERT_SHADOW(rv, 1);

  rv = strcasecmp(str1, str3);
  assert(rv == 0);
  ASSERT_SHADOW(rv, 1);
}

void test_strncasecmp() {
  char str1[] = "Str1", str2[] = "str2";
  icount_enable_range_shadow(&str1[3], 1);
  icount_enable_range_shadow(&str2[3], 1);

  int rv = strncasecmp(str1, str2, sizeof(str1));
  assert(rv < 0);
  ASSERT_SHADOW(rv, 1);

  rv = strncasecmp(str1, str2, 3);
  assert(rv == 0);
  ASSERT_DISABLED_SHADOW(rv);
}

void test_strchr() {
  char str1[] = "str1";
  icount_enable_range_shadow(&str1[3], 1);

  char *crv = strchr(str1, 'r');
  assert(crv == &str1[2]);
  ASSERT_DISABLED_SHADOW(crv);

  crv = strchr(str1, '1');
  assert(crv == &str1[3]);
  ASSERT_SHADOW(crv, 1);

  crv = strchr(str1, 'x');
  assert(!crv);
  ASSERT_SHADOW(crv, 1);
}

void test_calloc() {
  // With any luck this sequence of calls will cause calloc to return the same
  // pointer both times.  This is probably the best we can do to test this
  // function.
  char *crv = (char *) calloc(4096, 1);
  ASSERT_DISABLED_SHADOW(crv[0]);
  icount_enable_range_shadow(crv, 100);
  free(crv);

  crv = (char *) calloc(4096, 1);
  ASSERT_DISABLED_SHADOW(crv[0]);
  free(crv);
}

void test_dlopen() {
  void *map = dlopen(NULL, RTLD_NOW);
  assert(map);
  ASSERT_DISABLED_SHADOW(map);
  dlclose(map);
  map = dlopen("/nonexistent", RTLD_NOW);
  assert(!map);
  ASSERT_DISABLED_SHADOW(map);
}

void test_clock_gettime() {
  struct timespec tp;
  icount_enable_range_shadow(((char *)&tp) + 3, 1);
  int t = clock_gettime(CLOCK_REALTIME, &tp);
  assert(t == 0);
  ASSERT_DISABLED_SHADOW(t);
  ASSERT_DISABLED_SHADOW(((char *)&tp)[3]);
}

void test_ctime_r() {
  char *buf = (char*) malloc(64);
  time_t t = 0;

  char *ret = ctime_r(&t, buf);
  ASSERT_DISABLED_SHADOW(ret);
  assert(buf == ret);
  ASSERT_DISABLED_RANGE_SHADOW(buf, strlen(buf) + 1);

  icount_enable_range_shadow(&t, sizeof(t));
  ret = ctime_r(&t, buf);
  ASSERT_DISABLED_SHADOW(ret);
  ASSERT_RANGE_SHADOW(buf, strlen(buf) + 1, 1);

  t = 0;
  icount_enable_range_shadow(&buf, sizeof(&buf));
  ret = ctime_r(&t, buf);
  ASSERT_SHADOW(ret, 1);
  ASSERT_DISABLED_RANGE_SHADOW(buf, strlen(buf) + 1);
}

void test_getcwd() {
  char buf[1024];
  char *ptr = buf;
  icount_enable_range_shadow(buf + 2, 2);
  char* ret = getcwd(buf, sizeof(buf));
  assert(ret == buf);
  assert(ret[0] == '/');
  ASSERT_DISABLED_RANGE_SHADOW(buf + 2, 2);
  icount_enable_range_shadow(&ptr, sizeof(ptr));
  ret = getcwd(ptr, sizeof(buf));
  ASSERT_SHADOW(ret, 1);
}

void test_get_current_dir_name() {
  char* ret = get_current_dir_name();
  assert(ret);
  assert(ret[0] == '/');
  ASSERT_DISABLED_RANGE_SHADOW(ret, strlen(ret) + 1);
}

void test_gethostname() {
  char buf[1024];
  icount_enable_range_shadow(buf + 2, 2);
  assert(gethostname(buf, sizeof(buf)) == 0);
  ASSERT_DISABLED_RANGE_SHADOW(buf + 2, 2);
}

void test_getrlimit() {
  struct rlimit rlim;
  icount_enable_range_shadow(&rlim, sizeof(rlim));
  assert(getrlimit(RLIMIT_CPU, &rlim) == 0);
  ASSERT_DISABLED_RANGE_SHADOW(&rlim, sizeof(rlim));
}

void test_getrusage() {
  struct rusage usage;
  icount_enable_range_shadow(&usage, sizeof(usage));
  assert(getrusage(RUSAGE_SELF, &usage) == 0);
  ASSERT_DISABLED_RANGE_SHADOW(&usage, sizeof(usage));
}

void test_strcpy() {
  char src[] = "hello world";
  char dst[sizeof(src) + 2];
  icount_disable_range_shadow(src, sizeof(src));
  icount_disable_range_shadow(dst, sizeof(dst));
  icount_enable_range_shadow(src + 2, 1);
  icount_enable_range_shadow(src + 3, 1);
  icount_enable_range_shadow(dst + 4, 1);
  icount_enable_range_shadow(dst + 12, 1);
  char *ret = strcpy(dst, src);
  assert(ret == dst);
  assert(strcmp(src, dst) == 0);
  for (int i = 0; i < strlen(src) + 1; ++i) {
    assert(icount_get_shadow(dst[i]) == icount_get_shadow(src[i]));
  }
  // Note: if strlen(src) + 1 were used instead to compute the first untouched
  // byte of dest, the label would be I|J. This is because strlen() might
  // return a non-zero label, and because by default pointer labels are not
  // ignored on loads.
  ASSERT_SHADOW(dst[12], 1);
}

void test_strtol() {
  char buf[] = "1234578910";
  char *endptr = NULL;
  icount_enable_range_shadow(buf + 1, 1);
  icount_set_range_shadow(2, buf + 10, 1);
  long int ret = strtol(buf, &endptr, 10);
  assert(ret == 1234578910);
  assert(endptr == buf + 10);
  ASSERT_SHADOW(ret, 2);
}

void test_strtoll() {
  char buf[] = "1234578910 ";
  char *endptr = NULL;
  icount_enable_range_shadow(buf + 1, 1);
  icount_set_range_shadow(2, buf + 2, 1);
  long long int ret = strtoll(buf, &endptr, 10);
  assert(ret == 1234578910);
  assert(endptr == buf + 10);
  ASSERT_SHADOW(ret, 2);
}

void test_strtoul() {
  char buf[] = "0xffffffffffffaa";
  char *endptr = NULL;
  icount_enable_range_shadow(buf + 1, 1);
  icount_set_range_shadow(2, buf + 2, 1);
  long unsigned int ret = strtol(buf, &endptr, 16);
  assert(ret == 72057594037927850);
  assert(endptr == buf + 16);
  ASSERT_SHADOW(ret, 2);
}

void test_strtoull() {
  char buf[] = "0xffffffffffffffaa";
  char *endptr = NULL;
  icount_enable_range_shadow(buf + 1, 1);
  icount_set_range_shadow(2, buf + 2, 1);
  long long unsigned int ret = strtoull(buf, &endptr, 16);
  assert(ret == 0xffffffffffffffaa);
  assert(endptr == buf + 18);
  ASSERT_SHADOW(ret, 2);
}

void test_strtod() {
  char buf[] = "12345.76 foo";
  char *endptr = NULL;
  icount_enable_range_shadow(buf + 1, 1);
  icount_set_range_shadow(2, buf + 2, 1);
  double ret = strtod(buf, &endptr);
  assert(ret == 12345.76);
  assert(endptr == buf + 8);
  ASSERT_SHADOW(ret, 2);
}

void test_time() {
  time_t t = 0;
  icount_enable_range_shadow(&t, 1);
  time_t ret = time(&t);
  assert(ret == t);
  assert(ret > 0);
  ASSERT_DISABLED_SHADOW(t);
}

void test_inet_pton() {
  char addr4[] = "127.0.0.1";
  icount_enable_range_shadow(addr4 + 3, 1);
  struct in_addr in4;
  int ret4 = inet_pton(AF_INET, addr4, &in4);
  assert(ret4 == 1);
  ASSERT_RANGE_SHADOW(&in4, sizeof(in4), 1);
  assert(in4.s_addr == htonl(0x7f000001));

  char addr6[] = "::1";
  icount_enable_range_shadow(addr6 + 3, 1);
  struct in6_addr in6;
  int ret6 = inet_pton(AF_INET6, addr6, &in6);
  assert(ret6 == 1);
  ASSERT_RANGE_SHADOW(((char *) &in6) + sizeof(in6) - 1, 1, 1);
}

void test_localtime_r() {
  time_t t0 = 1384800998;
  struct tm t1;
  icount_enable_range_shadow(&t0, sizeof(t0));
  struct tm* ret = localtime_r(&t0, &t1);
  assert(ret == &t1);
  assert(t1.tm_min == 56);
  ASSERT_SHADOW(t1.tm_mon, 1);
}

void test_getpwuid_r() {
  struct passwd pwd;
  char buf[1024];
  struct passwd *result;

  icount_enable_range_shadow(&pwd, 4);
  int ret = getpwuid_r(0, &pwd, buf, sizeof(buf), &result);
  assert(ret == 0);
  assert(strcmp(pwd.pw_name, "root") == 0);
  assert(result == &pwd);
  ASSERT_DISABLED_RANGE_SHADOW(&pwd, 4);
}

void test_poll() {
  struct pollfd fd;
  fd.fd = 0;
  fd.events = POLLIN;
  icount_enable_range_shadow(&fd.revents, sizeof(fd.revents));
  int ret = poll(&fd, 1, 1);
  ASSERT_DISABLED_SHADOW(fd.revents);
  assert(ret >= 0);
}

void test_select() {
  struct timeval t;
  fd_set fds;
  t.tv_sec = 2;
  FD_SET(0, &fds);
  icount_enable_range_shadow(&fds, sizeof(fds));
  icount_enable_range_shadow(&t, sizeof(t));
  int ret = select(1, &fds, NULL, NULL, &t);
  assert(ret >= 0);
  ASSERT_DISABLED_SHADOW(t.tv_sec);
  ASSERT_DISABLED_RANGE_SHADOW(&fds, sizeof(fds));
}

void test_sched_getaffinity() {
  cpu_set_t mask;
  icount_enable_range_shadow(&mask, 1);
  int ret = sched_getaffinity(0, sizeof(mask), &mask);
  assert(ret == 0);
  ASSERT_DISABLED_RANGE_SHADOW(&mask, sizeof(mask));
}

void test_sigemptyset() {
  sigset_t set;
  icount_enable_range_shadow(&set, 1);
  int ret = sigemptyset(&set);
  assert(ret == 0);
  ASSERT_DISABLED_RANGE_SHADOW(&set, sizeof(set));
}

void test_sigaction() {
  struct sigaction oldact;
  icount_enable_range_shadow(&oldact, 1);
  int ret = sigaction(SIGUSR1, NULL, &oldact);
  assert(ret == 0);
  ASSERT_DISABLED_RANGE_SHADOW(&oldact, sizeof(oldact));
}

void test_gettimeofday() {
  struct timeval tv;
  struct timezone tz;
  icount_enable_range_shadow(&tv, sizeof(tv));
  icount_enable_range_shadow(&tz, sizeof(tz));
  int ret = gettimeofday(&tv, &tz);
  assert(ret == 0);
  ASSERT_DISABLED_RANGE_SHADOW(&tv, sizeof(tv));
  ASSERT_DISABLED_RANGE_SHADOW(&tz, sizeof(tz));
}

void *pthread_create_test_cb(void *p) {
  assert(p == (void *)1);
  ASSERT_DISABLED_SHADOW(p);
  return (void *)2;
}

void test_pthread_create() {
  pthread_t pt;
  pthread_create(&pt, 0, pthread_create_test_cb, (void *)1);
  void *cbrv;
  pthread_join(pt, &cbrv);
  assert(cbrv == (void *)2);
}

int dl_iterate_phdr_test_cb(struct dl_phdr_info *info, size_t size,
                            void *data) {
  assert(data == (void *)3);
  ASSERT_DISABLED_SHADOW(info);
  ASSERT_DISABLED_SHADOW(size);
  ASSERT_DISABLED_SHADOW(data);
  return 0;
}

void test_dl_iterate_phdr() {
  dl_iterate_phdr(dl_iterate_phdr_test_cb, (void *)3);
}

void test_strrchr() {
  char str1[] = "str1str1";
  icount_enable_range_shadow(&str1[7], 1);

  char *rv = strrchr(str1, 'r');
  assert(rv == &str1[6]);
  ASSERT_SHADOW(rv, 1);
}

void test_strstr() {
  char str1[] = "str1str1";
  icount_enable_range_shadow(&str1[3], 1);

  char *rv = strstr(str1, "1s");
  assert(rv == &str1[3]);
  ASSERT_SHADOW(rv, 1);

  rv = strstr(str1, "2s");
  assert(rv == NULL);
  ASSERT_SHADOW(rv, 1);
}

void test_memchr() {
  char str1[] = "str1";
  icount_enable_range_shadow(&str1[3], 1);

  char *crv = (char *) memchr(str1, 'r', sizeof(str1));
  assert(crv == &str1[2]);
  ASSERT_DISABLED_SHADOW(crv);

  crv = (char *) memchr(str1, '1', sizeof(str1));
  assert(crv == &str1[3]);
  ASSERT_SHADOW(crv, 1);

  crv = (char *) memchr(str1, 'x', sizeof(str1));
  assert(!crv);
  ASSERT_SHADOW(crv, 1);
}

void alarm_handler(int unused) {
  ;
}

void test_nanosleep() {
  struct timespec req, rem;
  req.tv_sec = 1;
  req.tv_nsec = 0;
  icount_enable_range_shadow(&rem, sizeof(rem));

  // non interrupted
  int rv = nanosleep(&req, &rem);
  assert(rv == 0);
  ASSERT_DISABLED_SHADOW(rv);
  ASSERT_RANGE_SHADOW(&rem, 1, 1);

  // interrupted by an alarm
  signal(SIGALRM, alarm_handler);
  req.tv_sec = 3;
  alarm(1);
  rv = nanosleep(&req, &rem);
  assert(rv == -1);
  ASSERT_DISABLED_SHADOW(rv);
  ASSERT_DISABLED_RANGE_SHADOW(&rem, sizeof(rem));
}

void test_socketpair() {
  int fd[2];

  icount_enable_range_shadow(fd, sizeof(fd));
  int rv = socketpair(PF_LOCAL, SOCK_STREAM, 0, fd);
  assert(rv == 0);
  ASSERT_DISABLED_SHADOW(rv);
  ASSERT_DISABLED_RANGE_SHADOW(fd, sizeof(fd));
}

void test_write() {
  int fd = open("/dev/null", O_WRONLY);

  char buf[] = "a string";
  int len = strlen(buf);

  // The result of a write always unlabeled.
  int res = write(fd, buf, len);
  assert(res > 0);
  ASSERT_DISABLED_SHADOW(res);

  // Label all arguments to write().
  icount_enable_range_shadow(&(buf[3]), 1);
  icount_enable_range_shadow(&fd, sizeof(fd));
  icount_enable_range_shadow(&len, sizeof(len));

  // The value returned by write() should have no label.
  res = write(fd, buf, len);
  ASSERT_DISABLED_SHADOW(res);

  close(fd);
}

template <class T>
void test_sprintf_chunk(const char* expected, const char* format, T arg) {
  char buf[512];
  memset(buf, 'a', sizeof(buf));

  char padded_expected[512];
  strcpy(padded_expected, "foo ");
  strcat(padded_expected, expected);
  strcat(padded_expected, " bar");

  char padded_format[512];
  strcpy(padded_format, "foo ");
  strcat(padded_format, format);
  strcat(padded_format, " bar");

  // Non labelled arg.
  assert(sprintf(buf, padded_format,  arg) == strlen(padded_expected));
  assert(strcmp(buf, padded_expected) == 0);
  ASSERT_RANGE_SHADOW(buf, strlen(padded_expected), 0);
  memset(buf, 'a', sizeof(buf));

  // Labelled arg.
  icount_enable_range_shadow(&arg, sizeof(arg));
  assert(sprintf(buf, padded_format,  arg) == strlen(padded_expected));
  assert(strcmp(buf, padded_expected) == 0);
  ASSERT_RANGE_SHADOW(buf, 4, 0);
  ASSERT_RANGE_SHADOW(buf + 4, strlen(padded_expected) - 8, 1);
  ASSERT_RANGE_SHADOW(buf + (strlen(padded_expected) - 4), 4, 0);
}

void test_sprintf() {
  char buf[2048];
  memset(buf, 'a', sizeof(buf));

  // Test formatting (no conversion specifier).
  assert(sprintf(buf, "Hello world!") == 12);
  assert(strcmp(buf, "Hello world!") == 0);
  ASSERT_RANGE_SHADOW(buf, sizeof(buf), 0);

  // Test for extra arguments.
  assert(sprintf(buf, "Hello world!", 42, "hello") == 12);
  assert(strcmp(buf, "Hello world!") == 0);
  ASSERT_RANGE_SHADOW(buf, sizeof(buf), 0);

  // Test formatting & label propagation (multiple conversion specifiers): %s,
  // %d, %n, %f, and %%.
  const char* s = "world";
  int m = 8;
  int d = 27;
  icount_enable_range_shadow((void *) (s + 1), 2);
  icount_enable_range_shadow(&m, sizeof(m));
  icount_enable_range_shadow(&d, sizeof(d));
  int n;
  int r = sprintf(buf, "hello %s, %-d/%d/%d %f %% %n%d", s, 2014, m, d,
                  12345.6781234, &n, 1000);
  assert(r == 42);
  assert(strcmp(buf, "hello world, 2014/8/27 12345.678123 % 1000") == 0);
  ASSERT_RANGE_SHADOW(buf, 7, 0);
  ASSERT_RANGE_SHADOW(buf + 7, 2, 1);
  ASSERT_RANGE_SHADOW(buf + 9, 9, 0);
  ASSERT_RANGE_SHADOW(buf + 18, 1, 1);
  ASSERT_RANGE_SHADOW(buf + 19, 1, 0);
  ASSERT_RANGE_SHADOW(buf + 20, 2, 1);
  ASSERT_RANGE_SHADOW(buf + 22, 15, 0);
  ASSERT_SHADOW(r, 0);
  assert(n == 38);

  // Test formatting & label propagation (single conversion specifier, with
  // additional length and precision modifiers).
  test_sprintf_chunk("-559038737", "%d", 0xdeadbeef);
  test_sprintf_chunk("3735928559", "%u", 0xdeadbeef);
  test_sprintf_chunk("12345", "%i", 12345);
  test_sprintf_chunk("751", "%o", 0751);
  test_sprintf_chunk("babe", "%x", 0xbabe);
  test_sprintf_chunk("0000BABE", "%.8X", 0xbabe);
  test_sprintf_chunk("-17", "%hhd", 0xdeadbeef);
  test_sprintf_chunk("-16657", "%hd", 0xdeadbeef);
  test_sprintf_chunk("deadbeefdeadbeef", "%lx", 0xdeadbeefdeadbeef);
  test_sprintf_chunk("0xdeadbeefdeadbeef", "%p",
                 (void *)  0xdeadbeefdeadbeef);
  test_sprintf_chunk("18446744073709551615", "%ju", (intmax_t) -1);
  test_sprintf_chunk("18446744073709551615", "%zu", (size_t) -1);
  test_sprintf_chunk("18446744073709551615", "%tu", (size_t) -1);

  test_sprintf_chunk("0x1.f9acffa7eb6bfp-4", "%a", 0.123456);
  test_sprintf_chunk("0X1.F9ACFFA7EB6BFP-4", "%A", 0.123456);
  test_sprintf_chunk("0.12346", "%.5f", 0.123456);
  test_sprintf_chunk("0.123456", "%g", 0.123456);
  test_sprintf_chunk("1.234560e-01", "%e", 0.123456);
  test_sprintf_chunk("1.234560E-01", "%E", 0.123456);
  test_sprintf_chunk("0.1234567891234560", "%.16Lf",
                     (long double) 0.123456789123456);

  test_sprintf_chunk("z", "%c", 'z');

  // %n, %s, %d, %f, and %% already tested

  // Test formatting with width passed as an argument.
  r = sprintf(buf, "hi %*d my %*s friend %.*f", 3, 1, 6, "dear", 4, 3.14159265359);
  assert(r == 30);
  assert(strcmp(buf, "hi   1 my   dear friend 3.1416") == 0);
}

void test_snprintf() {
  char buf[2048];
  memset(buf, 'a', sizeof(buf));
  const char* s = "world";
  int y = 2014;
  int m = 8;
  int d = 27;
  icount_enable_range_shadow((void *) (s + 1), 2);
  icount_enable_range_shadow(&y, sizeof(y));
  icount_enable_range_shadow(&m, sizeof(m));
  int r = snprintf(buf, 19, "hello %s, %-d/%d/%d %f", s, y, m, d,
                   12345.6781234);
  // The return value is the number of bytes that would have been written to
  // the final string if enough space had been available.
  assert(r == 35);
  assert(memcmp(buf, "hello world, 2014/", 19) == 0);
  ASSERT_RANGE_SHADOW(buf, 7, 0);
  ASSERT_RANGE_SHADOW(buf + 7, 2, 1);
  ASSERT_RANGE_SHADOW(buf + 9, 4, 0);
  ASSERT_RANGE_SHADOW(buf + 13, 4, 1);
  ASSERT_RANGE_SHADOW(buf + 17, 2, 0);
  ASSERT_SHADOW(r, 0);
}

int main(void) {
  test_open();
  test_fopen();
  test_close();
  test_fclose();
  test_mmap();
  test_munmap();
  test_fread();
  test_fread_unlocked();
  test_read();
  test_pread();
  test_fgetc();
  test_fgetc_unlocked();
  test_fgets();
  test_fgets_unlocked();
  test_getline();
  test_getdelim();
  test_calloc();
  test_clock_gettime();
  test_ctime_r();
  test_dl_iterate_phdr();
  test_dlopen();
  test_fstat();
  test_get_current_dir_name();
  test_getcwd();
  test_gethostname();
  test_getpwuid_r();
  test_getrlimit();
  test_getrusage();
  test_gettimeofday();
  test_inet_pton();
  test_localtime_r();
  test_memchr();
  test_memcmp();
  test_memcpy();
  test_memset();
  test_nanosleep();
  test_poll();
  test_pthread_create();
  test_sched_getaffinity();
  test_select();
  test_sigaction();
  test_sigemptyset();
  test_snprintf();
  test_socketpair();
  test_sprintf();
  test_stat();
  test_strcasecmp();
  test_strchr();
  test_strcmp();
  test_strcpy();
  test_strdup();
  test_strlen();
  test_strncasecmp();
  test_strncmp();
  test_strncpy();
  test_strrchr();
  test_strstr();
  test_strtod();
  test_strtol();
  test_strtoll();
  test_strtoul();
  test_strtoull();
  test_time();
  test_write();
}
