// RUN: %clang_tracer -g -mllvm -idassign-emit-info -mllvm -idassign-info-file -mllvm %t.id_info.csv %s %icount_abilist %rtlib_icount %rtlib_tracer %icount_rtdeps %rtdeps -o %t
// RUN: rm -f %t.csv
// RUN: %ld_path TRACER_ENABLE_FILE_OUTPUT=true TRACER_OUTPUT_FILE=%t.csv TRACER_INPUT_FILE=%S/flag.txt %t %S/flag.txt
// RUN: cat %t.csv | FileCheck %s
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

// CHECK: condition_id,condition_count
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
  read(fd, buffer, sizeof(buffer));

// CHECK: 0x{{[0-9]+}},1
  if (!strcmp(buffer, "flag")) {
    puts("Flag found!");
  } else {
    puts("Flag not found!");
  }

  close(fd);

  return 0;
}
