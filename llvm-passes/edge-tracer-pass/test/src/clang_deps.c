// RUN: rm -f %t
// RUN: %clang_tracer -fno-discard-value-names -O3 -S -emit-llvm %s -o %t
// RUN: FileCheck -input-file=%t %s
#include <stdio.h>

int main(int argc, char* argv[argc + 1]) {
	puts("__FILE__:__LINE__");

// CHECK-LABEL: crit_edge:
// CHECK-NEXT: __edge_tracer_trace

	if (argc == 2) {
		// CHECK-LABEL: then:
		// CHECK-NEXT: __edge_tracer_trace
		// CHECK-NEXT: call
		puts("__FILE__:__LINE__");
		// CHECK-NEXT: __edge_tracer_trace
	}

	return 0;
}
