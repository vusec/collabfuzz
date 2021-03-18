// RUN: %clang_tracer %s %rtlib_tracer %rtdeps -o %t
// RUN: TRACER_ENABLE_FILE_OUTPUT=yolo not %t 2>&1 | FileCheck %s

// CHECK: tracer error: the argument ('yolo') for option 'enable_file_output' is invalid.
int main(void) { return 0; }
