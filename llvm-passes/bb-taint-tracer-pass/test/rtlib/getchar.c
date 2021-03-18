// RUN: %clang_tracer -g -mllvm -idassign-emit-info -mllvm -idassign-info-file -mllvm %t.id_info.csv %s %dfsan_opts %ld_wrap %rtlib_tracer %rtdeps -o %t
// RUN: rm -f %t.csv
// RUN: %ld_path DFSAN_OPTIONS="strict_data_dependencies=0" TRACER_ENABLE_FILE_OUTPUT=true TRACER_OUTPUT_FILE=%t.csv %t < %S/flag.txt
// RUN: cat %t.csv | FileCheck %s
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// CHECK: basic_block_id,terminator_id,terminator_tainted
int main(void) {
  char value = getchar();

// CHECK: 0x{{[0-9a-f]+}},0x{{[0-9a-f]+}},true
  if (value == 'f') {
    puts("Flag found!");
  } else {
    puts("Flag not found!");
  }

  return 0;
}
