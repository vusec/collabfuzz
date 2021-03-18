// RUN: %clang_tracer -g -mllvm -idassign-emit-info -mllvm -idassign-info-file -mllvm %t.id_info.csv %s %icount_abilist %rtlib_icount %rtlib_tracer %icount_rtdeps %rtdeps -o %t
// RUN: rm -f %t.csv
// RUN: %ld_path TRACER_ENABLE_FILE_OUTPUT=true TRACER_OUTPUT_FILE=%t.csv TRACER_INPUT_FILE=%S/flag.txt %t %S/flag.txt
// RUN: cat %t.csv | FileCheck %s
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// CHECK: condition_id,condition_count
int main(int argc, char *argv[argc + 1]) {
  if (argc != 2) {
    printf("%s INPUT_PATH\n", argv[0]);
    exit(1);
  }

  FILE *stream = fopen(argv[1], "r");
  if (!stream) {
    perror("could not open file");
    exit(1);
  }

  char value = fgetc_unlocked(stream);

// CHECK: 0x{{[0-9]+}},1
  if (value == 'f') {
    puts("Flag found!");
  } else {
    puts("Flag not found!");
  }

  fclose(stream);

  return 0;
}
