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

void check_char(FILE *stream, char target) {
  char value = fgetc(stream);

// CHECK:       "{{[0-9]+}}": {
// CHECK-NEXT:    "times_seen": 2,
// CHECK-NEXT:    "input_offsets": [
// CHECK-NEXT:      1
// CHECK-NEXT:    ],
// CHECK-NEXT:    "conditions_before_count": 2,
// CHECK-NEXT:    "tainted_conditions_before_count": 0
// CHECK-NEXT:  },
  if (value == target) {
    puts("Flag found!");
  } else {
    puts("Flag not found!");
  }
}

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

  char value = fgetc(stream);

// CHECK:       "{{[0-9]+}}": {
// CHECK-NEXT:    "times_seen": 1,
// CHECK-NEXT:    "input_offsets": [
// CHECK-NEXT:      0
// CHECK-NEXT:    ],
// CHECK-NEXT:    "conditions_before_count": 0,
// CHECK-NEXT:    "tainted_conditions_before_count": 0
// CHECK-NEXT:  },
  if (value == 'f') {
    puts("Flag found!");
  } else {
    puts("Flag not found!");
  }

// CHECK:       "{{[0-9]+}}": {
// CHECK-NEXT:    "times_seen": 1,
// CHECK-NEXT:    "input_offsets": [
// CHECK-NEXT:      0
// CHECK-NEXT:    ],
// CHECK-NEXT:    "conditions_before_count": 1,
// CHECK-NEXT:    "tainted_conditions_before_count": 1
// CHECK-NEXT:  }
  if (value == 'z') {
    puts("It is z");
  } else {
    puts("Not z");
  }

  check_char(stream, 'l');

  check_char(stream, 'a');

  fclose(stream);

  return 0;
}

// CHECK: }
