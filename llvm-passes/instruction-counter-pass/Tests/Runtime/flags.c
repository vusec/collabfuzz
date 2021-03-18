// RUN: %clang_icount %include_path %s %runtime_lib %load_blacklist -mllvm -icount-abilist=%S/Inputs/flags_abilist.txt -mllvm -icount-debug-nonzero-shadows -o %t
// RUN: %t 2>&1 | FileCheck %s
// RUN: %clang_icount %include_path %s %runtime_lib %load_blacklist -mllvm -icount-abilist=%S/Inputs/flags_abilist.txt -mllvm -icount-debug-nonzero-shadows -o %t
// RUN: %clang_icount %include_path %s %runtime_lib %load_blacklist -mllvm -icount-abilist=%S/Inputs/flags_abilist.txt -mllvm -icount-debug-nonzero-shadows -o %t
// RUN: ICOUNT_OPTIONS=warn_nonzero_shadows=1 %t 2>&1 | FileCheck --check-prefix=CHECK-NONZERO %s

// Tests that flags work correctly.

#include <sanitizer/icount_interface.h>

int f(int i) {
  return i;
}

int main(void) {
  int i = 42;
  icount_enable_range_shadow(&i, sizeof(i));

  // CHECK: WARNING: InstructionCounter: call to uninstrumented function f
  // CHECK-NOT: WARNING: InstructionCounter: saw nonzero shadow
  // CHECK-NONZERO: WARNING: InstructionCounter: saw nonzero shadow
  f(i);

  return 0;
}
