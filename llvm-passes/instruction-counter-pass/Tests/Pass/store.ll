; RUN: %opt_icount %s -S | FileCheck %s
target datalayout = "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v64:64:64-v128:128:128-a0:0:64-s0:64:64-f80:128:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

define void @store0({} %v, {}* %p) {
  ; CHECK: @"icount$store0"
  ; CHECK: store
  ; CHECK-NOT: store

  store {} %v, {}* %p
  ret void
}

define void @store8(i8 %v, i8* %p) {
  ; CHECK: @"icount$store8"
  ; CHECK: load i16, i16* {{.*}} @__icount_arg_tls
  ; CHECK: ptrtoint i8* {{.*}} i64
  ; CHECK: and i64
  ; CHECK: mul i64
  ; CHECK: inttoptr i64 {{.*}} i16*
  ; CHECK: getelementptr i16, i16*
  ; CHECK: store i16
  ; CHECK: store i8

  store i8 %v, i8* %p
  ret void
}

define void @store16(i16 %v, i16* %p) {
  ; CHECK: @"icount$store16"
  ; CHECK: load i16, i16* {{.*}} @__icount_arg_tls
  ; CHECK: ptrtoint i16* {{.*}} i64
  ; CHECK: and i64
  ; CHECK: mul i64
  ; CHECK: inttoptr i64 {{.*}} i16*
  ; CHECK: getelementptr i16, i16*
  ; CHECK: store i16
  ; CHECK: getelementptr i16, i16*
  ; CHECK: store i16
  ; CHECK: store i16

  store i16 %v, i16* %p
  ret void
}

define void @store32(i32 %v, i32* %p) {
  ; CHECK: @"icount$store32"
  ; CHECK: load i16, i16* {{.*}} @__icount_arg_tls
  ; CHECK: ptrtoint i32* {{.*}} i64
  ; CHECK: and i64
  ; CHECK: mul i64
  ; CHECK: inttoptr i64 {{.*}} i16*
  ; CHECK: getelementptr i16, i16*
  ; CHECK: store i16
  ; CHECK: getelementptr i16, i16*
  ; CHECK: store i16
  ; CHECK: getelementptr i16, i16*
  ; CHECK: store i16
  ; CHECK: getelementptr i16, i16*
  ; CHECK: store i16
  ; CHECK: store i32

  store i32 %v, i32* %p
  ret void
}

define void @store64(i64 %v, i64* %p) {
  ; CHECK: @"icount$store64"
  ; CHECK: load i16, i16* {{.*}} @__icount_arg_tls
  ; CHECK: ptrtoint i64* {{.*}} i64
  ; CHECK: and i64
  ; CHECK: mul i64
  ; CHECK: inttoptr i64 {{.*}} i16*
  ; CHECK: insertelement {{.*}} i16
  ; CHECK: insertelement {{.*}} i16
  ; CHECK: insertelement {{.*}} i16
  ; CHECK: insertelement {{.*}} i16
  ; CHECK: insertelement {{.*}} i16
  ; CHECK: insertelement {{.*}} i16
  ; CHECK: insertelement {{.*}} i16
  ; CHECK: insertelement {{.*}} i16
  ; CHECK: bitcast i16* {{.*}} <8 x i16>*
  ; CHECK: store i64

  store i64 %v, i64* %p
  ret void
}
