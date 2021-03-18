// RUN: %clang_icount %include_path %s %runtime_lib %load_blacklist -o %t
// RUN: %t

// Tests that the API is performing set and get operations correctly.

#include <sanitizer/icount_interface.h>
#include <assert.h>

int main(void) {
  int i = 42;
  assert(icount_get_shadow(i) == 0);
  assert(icount_get_range_shadow(&i, sizeof(i)) == 0);

  icount_enable_range_shadow(&i, sizeof(i));
  assert(icount_get_shadow(i) == 1);
  assert(icount_get_range_shadow(&i, sizeof(i)) == 1);

  icount_set_range_shadow(7, &i, sizeof(i));
  assert(icount_get_shadow(i) == 7);
  assert(icount_get_range_shadow(&i, sizeof(i)) == 7);

  icount_disable_range_shadow(&i, sizeof(i));
  assert(icount_get_shadow(i) == 0);
  assert(icount_get_range_shadow(&i, sizeof(i)) == 0);

  return 0;
}
