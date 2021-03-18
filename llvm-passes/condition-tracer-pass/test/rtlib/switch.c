// RUN: %clang_tracer %s %rtlib_tracer %rtdeps -o %t
// RUN: rm -f %t.csv
// RUN: TRACER_ENABLE_FILE_OUTPUT=1 TRACER_OUTPUT_FILE=%t.csv %t yolo
// RUN: cat %t.csv | FileCheck %s
#include <stdio.h>

int main(int argc, char *argv[argc + 1]) {
// CHECK: 0x{{[0-9a-f]+}},{{([0-1]{4})}}
  switch (argc) {
  case 1:
    puts("1");
  case 2:
    puts("2");
  case 3:
    puts("3");
  }

  return 0;
}
