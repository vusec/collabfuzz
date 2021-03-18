// RUN: %clang_tracer -g -mllvm -idassign-emit-info -mllvm -idassign-info-file -mllvm %t.id_info.csv %s %icount_abilist %rtlib_icount %rtlib_tracer %icount_rtdeps %rtdeps -o %t
// RUN: rm -f %t.csv
// RUN: %ld_path TRACER_ENABLE_FILE_OUTPUT=true TRACER_OUTPUT_FILE=%t.csv TRACER_INPUT_FILE=%S/number.txt %t %S/number.txt
// RUN: cat %t.csv | FileCheck %s
#include <stdio.h>
#include <stdlib.h>

// CHECK: condition_id,condition_count
int main(int argc, char *argv[argc + 1]) {
  if (argc != 2) {
    printf("usage: %s INPUT_PATH\n", argv[0]);
    exit(1);
  }

  FILE *input_stream = fopen(argv[1], "r");
  if (!input_stream) {
    perror("Could not open specified file");
    exit(1);
  }

  char input_buffer[10];
  char *res = fgets(input_buffer, sizeof(input_buffer), input_stream);
  if (!res) {
    perror("Could not read specified file");
    exit(1);
  }

  char *end_ptr = NULL;
  long int number = strtol(input_buffer, &end_ptr, 0);
  if (end_ptr == input_buffer) {
    printf("Could not parse number from: '%s'", input_buffer);
    exit(1);
  }

  long int power_2 = number * number;
  long int subtraction = power_2 - number;

  // CHECK: 0x{{[0-9a-f]+}},3
  if (subtraction == 1722) {
    puts("This is the answer!");
  } else {
    puts("Though luck...");
  }

  return EXIT_SUCCESS;
}
