// RUN: %clang_tracer -g \
// RUN:         -mllvm -idassign-emit-info \
// RUN:         -mllvm -idassign-info-file -mllvm %t.id_info.csv \
// RUN:         %s %dfsan_opts %ld_wrap %rtlib_tracer %rtdeps -o %t
// RUN: rm -f %t.json
// RUN: %ld_path \
// RUN:         DFSAN_OPTIONS="strict_data_dependencies=0" \
// RUN:         RUST_LOG=trace \
// RUN:         TRACER_OUTPUT_FILE=%t.json \
// RUN:         TRACER_INPUT_FILE=%S/flag.txt \
// RUN:         %t %S/flag.txt
// RUN: cat %t.json | FileCheck %s

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// CHECK: {
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

  char buffer[10];
  memset(buffer, 0, sizeof(buffer));
  fgets(buffer, sizeof(buffer), stream);

// CHECK:       "{{[0-9]+}}": {
// CHECK:         "input_offsets": [
// CHECK-DAG:        0{{,?}}
// CHECK-DAG:        1{{,?}}
// CHECK-DAG:        2{{,?}}
// CHECK-DAG:        3{{,?}}
// CHECK-NEXT:    ]
// CHECK:       }
  if (!strcmp(buffer, "flag")) {
    puts("Flag found!");
  } else {
    puts("Flag not found!");
  }

  fclose(stream);

  return 0;
}
// CHECK: }
