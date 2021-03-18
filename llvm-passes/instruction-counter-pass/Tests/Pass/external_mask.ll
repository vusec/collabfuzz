; RUN: %opt_icount %s -S | FileCheck %s
target datalayout = "e-m:e-i64:64-i128:128-n32:64-S128"
target triple = "aarch64-unknown-linux-gnu"

define i32 @test(i32 %a, i32* nocapture readonly %b) #0 {
; CHECK: @"icount$test"
; CHECK: %[[RV:.*]] load{{.*}}__icount_shadow_ptr_mask
; CHECK: ptrtoint i32* {{.*}} to i64
; CHECK: and {{.*}}%[[RV:.*]]
; CHECK: mul i64
  %1 = load i32, i32* %b, align 4
  %2 = add nsw i32 %1, %a
  ret i32 %2
}
