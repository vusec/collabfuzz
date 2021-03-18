// RUN: %clang_icount %include_path %load_blacklist %s %runtime_lib -o %t
// RUN: %t

// Tests that labels are propagated through function calls.

#include <sanitizer/icount_interface.h>
#include <assert.h>

int f(int x) {
  int j = 2;
  icount_enable_range_shadow(&j, sizeof(j));
  return x + j;
}

int main(void) {
  int i = 42;
  icount_enable_range_shadow(&i, sizeof(i));

  icount_shadow ij_shadow = icount_get_shadow(f(i));
  assert(ij_shadow == 2);

  return 0;
}
