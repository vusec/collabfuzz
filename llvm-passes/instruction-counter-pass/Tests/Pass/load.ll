; RUN: %opt_icount %s -S | FileCheck %s
target datalayout = "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v64:64:64-v128:128:128-a0:0:64-s0:64:64-f80:128:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

define {} @load0({}* %p) {
  ; CHECK: @"icount$load0"
  ; CHECK: load
  ; CHECK-NOT: load
  %a = load {}, {}* %p
  ret {} %a
}

define i8 @load8(i8* %p) {
  ; CHECK: @"icount$load8"
  ; CHECK: ptrtoint i8*
  ; CHECK: and i64
  ; CHECK: mul i64
  ; CHECK: inttoptr i64 {{.*}} to i16*
  ; CHECK: load i16, i16*
  ; CHECK: load i8, i8*
  ; CHECK: store i16 {{.*}} @__icount_retval_tls
  ; CHECK: ret i8

  %a = load i8, i8* %p
  ret i8 %a
}

define i16 @load16(i16* %p) {
  ; CHECK: @"icount$load16"
  ; CHECK: ptrtoint i16*
  ; CHECK: and i64
  ; CHECK: mul i64
  ; CHECK: inttoptr i64 {{.*}} i16*
  ; CHECK: getelementptr i16, i16*
  ; CHECK: load i16, i16*
  ; CHECK: load i16, i16*
  ; CHECK: icmp uge i16
  ; CHECK: select i1
  ; CHECK: load i16, i16*
  ; CHECK: store i16 {{.*}} @__icount_retval_tls
  ; CHECK: ret i16

  %a = load i16, i16* %p
  ret i16 %a
}

define i32 @load32(i32* %p) {
  ; CHECK: @"icount$load32"
  ; CHECK: ptrtoint i32*
  ; CHECK: and i64
  ; CHECK: mul i64
  ; CHECK: inttoptr i64 {{.*}} i16*
  ; CHECK: bitcast i16* {{.*}} i64*
  ; CHECK: load i64, i64*
  ; CHECK: trunc i64 {{.*}} i16
  ; CHECK: shl i64
  ; CHECK: lshr i64
  ; CHECK: or i64
  ; CHECK: icmp eq i64
  ; CHECK: load i32, i32*
  ; CHECK: store i16 {{.*}} @__icount_retval_tls
  ; CHECK: ret i32
  ; CHECK: call {{.*}} @__icount_combine_on_load

  %a = load i32, i32* %p
  ret i32 %a
}

define i64 @load64(i64* %p) {
  ; CHECK: @"icount$load64"
  ; CHECK: ptrtoint i64*
  ; CHECK: and i64
  ; CHECK: mul i64
  ; CHECK: inttoptr i64 {{.*}} i16*
  ; CHECK: bitcast i16* {{.*}} i64*
  ; CHECK: load i64, i64*
  ; CHECK: trunc i64 {{.*}} i16
  ; CHECK: shl i64
  ; CHECK: lshr i64
  ; CHECK: or i64
  ; CHECK: icmp eq i64
  ; CHECK: load i64, i64*
  ; CHECK: store i16 {{.*}} @__icount_retval_tls
  ; CHECK: ret i64
  ; CHECK: call {{.*}} @__icount_combine_on_load
  ; CHECK: getelementptr i64, i64* {{.*}} i64
  ; CHECK: load i64, i64*
  ; CHECK: icmp eq i64

  %a = load i64, i64* %p
  ret i64 %a
}
