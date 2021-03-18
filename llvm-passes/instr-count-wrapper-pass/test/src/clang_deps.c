// RUN: rm -f %t
// RUN: %clang_tracer %icount_abilist -fno-discard-value-names -O3 -S -emit-llvm %s -o %t
// RUN: FileCheck -input-file=%t %s
#include <stdio.h>

int main(int argc, char *argv[argc + 1]) {
  // CHECK: @__icountw___inst_count_trace
  if (argc == 2) {
    puts("case 0");
  } else {
    puts("case 1");
  }

  return 0;
}

// CHECK: @__inst_count_create()
// CHECK: @__inst_count_destroy()
