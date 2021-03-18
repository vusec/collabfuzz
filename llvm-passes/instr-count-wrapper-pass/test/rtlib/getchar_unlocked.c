// RUN: %clang_tracer -g -mllvm -idassign-emit-info -mllvm -idassign-info-file -mllvm %t.id_info.csv %s %icount_abilist %rtlib_icount %rtlib_tracer %icount_rtdeps %rtdeps -o %t
// RUN: rm -f %t.csv
// RUN: %ld_path TRACER_ENABLE_FILE_OUTPUT=true TRACER_OUTPUT_FILE=%t.csv %t < %S/flag.txt
// RUN: cat %t.csv | FileCheck %s
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// CHECK: condition_id,condition_count
int main(void) {
  char value = getchar_unlocked();

// CHECK: 0x{{[0-9a-f]+}},1
  if (value == 'f') {
    puts("Flag found!");
  } else {
    puts("Flag not found!");
  }

  return 0;
}
