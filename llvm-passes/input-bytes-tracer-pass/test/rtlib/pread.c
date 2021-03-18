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

#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

// CHECK: {
int main(int argc, char *argv[argc + 1]) {
  if (argc != 2) {
    printf("%s INPUT_PATH\n", argv[0]);
    exit(1);
  }

  int fd = open(argv[1], O_RDONLY);
  if (fd < 0) {
    perror("could not open file");
    exit(1);
  }

  char buffer[10];
  memset(buffer, 0, sizeof(buffer));
  pread(fd, buffer, sizeof(buffer), 0);

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

  memset(buffer, 0, sizeof(buffer));
  pread(fd, buffer, sizeof(buffer), 1);

// CHECK:       "{{[0-9]+}}": {
// CHECK:         "input_offsets": [
// CHECK-DAG:        1{{,?}}
// CHECK-DAG:        2{{,?}}
// CHECK-DAG:        3{{,?}}
// CHECK-NEXT:    ]
// CHECK:       }
  if (!strcmp(buffer, "lag")) {
    puts("Flag found!");
  } else {
    puts("Flag not found!");
  }

  close(fd);

  return 0;
}
// CHECK: }
