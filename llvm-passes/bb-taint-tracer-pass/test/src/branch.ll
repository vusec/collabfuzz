; RUN: rm -f %t.ll
; RUN: %opt_tracer -bb-taint-tracer %s -S -o %t.ll
; RUN: FileCheck -input-file=%t.ll %s
; ModuleID = 'branch.c'
source_filename = "branch.c"
target datalayout = "e-m:e-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

; Function Attrs: noinline nounwind optnone uwtable
define dso_local i32 @branch_func(i32 %input) #0 {
; CHECK-LABEL: entry
entry:
  %retval = alloca i32, align 4
  %input.addr = alloca i32, align 4
  store i32 %input, i32* %input.addr, align 4
  %0 = load i32, i32* %input.addr, align 4
  %cmp = icmp sgt i32 %0, 10
; CHECK: [[CAST:%[0-9]+]] = zext i1 [[COND:%[0-9a-z]+]] to i64
; CHECK: call void @__bb_taint_tracer_trace(i64 {{[0-9a-f]+}}, i64 {{[0-9a-f]+}}, i64 [[CAST]])
; CHECK-NEXT: br i1 [[COND]]
  br i1 %cmp, label %if.then, label %if.else

if.then:                                          ; preds = %entry
  store i32 11, i32* %retval, align 4
  br label %return

if.else:                                          ; preds = %entry
  store i32 7, i32* %retval, align 4
  br label %return

return:                                           ; preds = %if.else, %if.then
  %1 = load i32, i32* %retval, align 4
  ret i32 %1
}

attributes #0 = { noinline nounwind optnone uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }

!llvm.module.flags = !{!0}
!llvm.ident = !{!1}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{!"clang version 9.0.1 (git@github.com:llvm/llvm-project.git 2d75b245668a49815935687b9a70beddbc68f66c)"}
