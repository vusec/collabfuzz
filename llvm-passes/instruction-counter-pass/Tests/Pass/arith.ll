; RUN: %opt_icount %s -S | FileCheck %s
target datalayout = "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v64:64:64-v128:128:128-a0:0:64-s0:64:64-f80:128:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

; CHECK: @"icount$add"
define i8 @add(i8 %a, i8 %b) {
  ; CHECK-DAG: %[[ALABEL:.*]] = load{{.*}}__icount_arg_tls, i64 0, i64 0
  ; CHECK-DAG: %[[BLABEL:.*]] = load{{.*}}__icount_arg_tls, i64 0, i64 1
  ; CHECK: %[[GREATER:.*]] = icmp uge i16 %[[ALABEL]], %[[BLABEL]]
  ; CHECK: %[[COMB:.*]] = select i1 %[[GREATER]], i16 %[[ALABEL]], i16 %[[BLABEL]]
  ; CHECK: %[[ZERO:.*]] = icmp ne i16 %[[COMB]], 0
  ; CHECK: br i1 %[[ZERO]]

  ; CHECK: %[[TRAS:.*]] = add i16 %[[COMB]], 1
  ; CHECK: br label

  ; CHECK: %[[ADDLABEL:.*]] = phi i16 [ %[[TRAS]], {{.*}} ], [ %[[COMB]], {{.*}} ]
  ; CHECK: add i8
  %c = add i8 %a, %b
  ; CHECK: store i16 %[[ADDLABEL]], i16* @__icount_retval_tls
  ; CHECK: ret i8
  ret i8 %c
}

; CHECK: @"icount$sub"
define i8 @sub(i8 %a, i8 %b) {
  ; CHECK: load{{.*}}__icount_arg_tls
  ; CHECK: load{{.*}}__icount_arg_tls
  ; CHECK: icmp uge i16
  ; CHECK: select
  ; CHECK: icmp ne i16 {{.*}}, 0
  ; CHECK: br i1

  ; CHECK: add i16 {{.*}}, 1
  ; CHECK: br label

  ; CHECK: phi i16
  ; CHECK: sub i8
  %c = sub i8 %a, %b
  ; CHECK: store{{.*}}__icount_retval_tls
  ; CHECK: ret i8
  ret i8 %c
}

; CHECK: @"icount$mul"
define i8 @mul(i8 %a, i8 %b) {
  ; CHECK: load{{.*}}__icount_arg_tls
  ; CHECK: load{{.*}}__icount_arg_tls
  ; CHECK: icmp uge i16
  ; CHECK: select
  ; CHECK: icmp ne i16 {{.*}}, 0
  ; CHECK: br i1

  ; CHECK: add i16 {{.*}}, 1
  ; CHECK: br label

  ; CHECK: phi i16
  ; CHECK: mul i8
  %c = mul i8 %a, %b
  ; CHECK: store{{.*}}__icount_retval_tls
  ; CHECK: ret i8
  ret i8 %c
}

; CHECK: @"icount$sdiv"
define i8 @sdiv(i8 %a, i8 %b) {
  ; CHECK: load{{.*}}__icount_arg_tls
  ; CHECK: load{{.*}}__icount_arg_tls
  ; CHECK: icmp uge i16
  ; CHECK: select
  ; CHECK: icmp ne i16 {{.*}}, 0
  ; CHECK: br i1

  ; CHECK: add i16 {{.*}}, 1
  ; CHECK: br label

  ; CHECK: phi i16
  ; CHECK: sdiv i8
  %c = sdiv i8 %a, %b
  ; CHECK: store{{.*}}__icount_retval_tls
  ; CHECK: ret i8
  ret i8 %c
}

; CHECK: @"icount$udiv"
define i8 @udiv(i8 %a, i8 %b) {
  ; CHECK: load{{.*}}__icount_arg_tls
  ; CHECK: load{{.*}}__icount_arg_tls
  ; CHECK: icmp uge i16
  ; CHECK: select
  ; CHECK: icmp ne i16 {{.*}}, 0
  ; CHECK: br i1

  ; CHECK: add i16 {{.*}}, 1
  ; CHECK: br label

  ; CHECK: phi i16
  ; CHECK: udiv i8
  %c = udiv i8 %a, %b
  ; CHECK: store{{.*}}__icount_retval_tls
  ; CHECK: ret i8
  ret i8 %c
}
