// RUN: rm -f %t.ll
// RUN: %clang_tracer -O3 %s %dfsan_opts -c -S -o %t.ll
// RUN: FileCheck -input-file=%t.ll %s
void external1(int input);
void external2(int input);
void external3(int input);
void external4(int input);
void external5(int input);

void switch_func(int input) {
// CHECK-LABEL: entry
// CHECK: [[INPUT_LABEL:%[0-9]+]] = load i16
// CHECK: [[CAST:%[0-9]+]] = zext i32 [[COND:%[0-9a-z]+]] to i64
// CHECK-NEXT: call void @__dfsw___bb_taint_tracer_trace(i64 {{[0-9]+}}, i64 {{[0-9]+}}, i64 [[CAST]], i16 zeroext 0, i16 zeroext 0, i16 zeroext [[INPUT_LABEL]])
// CHECK-NEXT: switch i32 [[COND]]
  switch (input) {
  case 3:
    external1(input);
  case 4:
    external2(input);
  case 7:
    external3(input);
  case 5:
    external4(input);
  default:
    external5(input);
  }
}
