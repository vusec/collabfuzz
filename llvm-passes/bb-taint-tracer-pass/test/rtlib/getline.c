// RUN: %clang_tracer -g -mllvm -idassign-emit-info -mllvm -idassign-info-file -mllvm %t.id_info.csv %s %dfsan_opts %ld_wrap %rtlib_tracer %rtdeps -o %t
// RUN: rm -f %t.csv
// RUN: %ld_path DFSAN_OPTIONS="strict_data_dependencies=0" TRACER_ENABLE_FILE_OUTPUT=true TRACER_OUTPUT_FILE=%t.csv TRACER_INPUT_FILE=%S/flag.txt %t %S/flag.txt
// RUN: cat %t.csv | FileCheck %s
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// CHECK: basic_block_id,terminator_id,terminator_tainted
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

  char* buffer = NULL;
  size_t buffer_size = 0;
  getline(&buffer, &buffer_size, stream);

// CHECK: 0x{{[0-9a-f]+}},0x{{[0-9a-f]+}},true
  if (!strcmp(buffer, "flag")) {
    puts("Flag found!");
  } else {
    puts("Flag not found!");
  }

  fclose(stream);

  return 0;
}
