; RUN: %opt_icount %s -S | FileCheck %s
; RUN: %opt_icount %s -icount-args-abi -S | FileCheck %s
target datalayout = "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v64:64:64-v128:128:128-a0:0:64-s0:64:64-f80:128:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

; CHECK: module asm ".symver icount$f1,icount$f@@version1"
module asm ".symver f1,f@@version1"

; CHECK: @"icount$f2" = alias {{.*}} @"icount$f1"
@f2 = alias void (), void ()* @f1

; CHECK: @"icount$g2" = alias {{.*}} @"icount$g1"
@g2 = alias void (i16*), bitcast (void (i8*)* @g1 to void (i16*)*)

; CHECK: define void @"icount$f1"
define void @f1() {
  ret void
}

; CHECK: define void @"icount$g1"
define void @g1(i8*) {
  ret void
}
