; RUN: rm -f %t.ll
; RUN: %opt_tracer -cond-tracer %s -S -o %t.ll
; RUN: FileCheck -input-file=%t.ll %s
; ModuleID = 'switch.c'
source_filename = "switch.c"
target datalayout = "e-m:e-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@.str = private unnamed_addr constant [2 x i8] c"1\00", align 1
@.str.1 = private unnamed_addr constant [2 x i8] c"2\00", align 1
@.str.2 = private unnamed_addr constant [2 x i8] c"3\00", align 1
@.str.3 = private unnamed_addr constant [2 x i8] c"0\00", align 1
; CHECK: @llvm.global_ctors = appending global [1 x { i32, void ()*, i8* }] [{ i32, void ()*, i8* } { i32 0, void ()* @__cond_tracer_create, i8* null }]
; CHECK: @llvm.global_dtors = appending global [1 x { i32, void ()*, i8* }] [{ i32, void ()*, i8* } { i32 0, void ()* @__cond_tracer_destroy, i8* null }]

; Function Attrs: nounwind uwtable
define dso_local i32 @main(i32, i8** nocapture readnone) local_unnamed_addr #0 {
  switch i32 %0, label %9 [
    i32 1, label %3
    i32 2, label %5
    i32 3, label %7
  ]

; <label>:3:                                      ; preds = %2
; CHECK: call void @__cond_tracer_trace(i64 5, i64 4, i64 1)
  %4 = tail call i32 @puts(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str, i64 0, i64 0))
  br label %11

; <label>:5:                                      ; preds = %2
; CHECK: call void @__cond_tracer_trace(i64 5, i64 4, i64 2)
  %6 = tail call i32 @puts(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.1, i64 0, i64 0))
  br label %11

; <label>:7:                                      ; preds = %2
; CHECK: call void @__cond_tracer_trace(i64 5, i64 4, i64 3)
  %8 = tail call i32 @puts(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.2, i64 0, i64 0))
  br label %11

; <label>:9:                                      ; preds = %2
; CHECK: call void @__cond_tracer_trace(i64 5, i64 4, i64 0)
  %10 = tail call i32 @puts(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.3, i64 0, i64 0))
  br label %11

; <label>:11:                                     ; preds = %9, %7, %5, %3
  ret i32 0
}

; Function Attrs: nounwind
declare dso_local i32 @puts(i8* nocapture readonly) local_unnamed_addr #1

; CHECK: declare void @__cond_tracer_create()

; CHECK: declare void @__cond_tracer_destroy()

; CHECK: declare void @__cond_tracer_trace(i64, i64, i64)

attributes #0 = { nounwind uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="false" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+fxsr,+mmx,+sse,+sse2,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #1 = { nounwind "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "no-frame-pointer-elim"="false" "no-infs-fp-math"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+fxsr,+mmx,+sse,+sse2,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }

!llvm.module.flags = !{!0}
!llvm.ident = !{!1}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{!"clang version 8.0.1 "}
