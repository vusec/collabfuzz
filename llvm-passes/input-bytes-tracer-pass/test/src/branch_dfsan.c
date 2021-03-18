// RUN: rm -f %t.ll
// RUN: %clang_tracer -O3 %s %dfsan_opts -c -S -emit-llvm -o %t.ll
// RUN: FileCheck -input-file=%t.ll %s
void external1(int input);
void external2(int input);

void branch_func(int input) {
// CHECK-LABEL: entry
// CHECK: [[INPUT_LABEL:%[0-9]+]] = load i16
// CHECK: [[CAST:%[0-9]+]] = zext i1 [[COND:%[0-9a-z]+]] to i64
// CHECK-NEXT: call void @__dfsw___bb_taint_tracer_trace(i64 {{[0-9]+}}, i64 [[CAST]], i16 zeroext 0, i16 zeroext [[INPUT_LABEL]])
// CHECK-NEXT: br i1 [[COND]]
  if (input > 10) {
    external1(input);
  } else {
    external2(input);
  }
}
