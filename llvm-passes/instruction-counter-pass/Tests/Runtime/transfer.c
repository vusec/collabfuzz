// RUN: %clang_icount %include_path %s %runtime_lib %load_blacklist -o %t
// RUN: %t

// Tests that the combination and the transfer functions behave consistently and
// as expected.

#include <sanitizer/icount_interface.h>
#include <assert.h>

int main(void) {
  int i = 42;
  icount_enable_range_shadow(&i, sizeof(i));
  icount_shadow i_shadow = icount_get_shadow(i);
  assert(i_shadow == 1);

  int j = 42;
  icount_enable_range_shadow(&j, sizeof(j));
  icount_shadow j_shadow = icount_get_shadow(j);
  assert(j_shadow == 1);

  int k = 42;
  icount_enable_range_shadow(&k, sizeof(k));
  icount_shadow k_shadow = icount_get_shadow(k);
  assert(k_shadow == 1);

  int ij = i + j;
  icount_shadow ij_shadow = icount_get_shadow(ij);
  assert(ij_shadow == 2);

  int ik = i + k;
  icount_shadow ik_shadow = icount_get_shadow(ik);
  assert(ik_shadow == 2);

  int ijk = ij + ik;
  icount_shadow ijk_shadow = icount_get_shadow(ijk);
  assert(ijk_shadow == 3);

  icount_shadow user_shadow = icount_transfer_shadow(icount_combine_shadows(
      icount_transfer_shadow(icount_combine_shadows(i_shadow, j_shadow)),
      icount_transfer_shadow(icount_combine_shadows(i_shadow, k_shadow))));
  assert(ijk_shadow == user_shadow);

  return 0;
}
