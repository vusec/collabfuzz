; RUN: rm -f %t.ll
; RUN: %opt_tracer -edge-tracer %s -S -o %t.ll
; RUN: FileCheck -input-file=%t.ll %s
; ModuleID = 'hello.c'
source_filename = "hello.c"
target datalayout = "e-m:e-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@.str = private unnamed_addr constant [21 x i8] c"Lots of arguments...\00", align 1
@.str.1 = private unnamed_addr constant [10 x i8] c"Only two!\00", align 1

; Function Attrs: nounwind uwtable
define dso_local i32 @main(i32 %argc, i8** nocapture readnone %argv) local_unnamed_addr #0 {
entry:
  %call = tail call i32 @puts(i8* getelementptr inbounds ([21 x i8], [21 x i8]* @.str, i64 0, i64 0))
  %cmp = icmp eq i32 %argc, 2
  br i1 %cmp, label %if.then, label %if.end

; CHECK-LABEL: entry.if.end_crit_edge:
; CHECK-NEXT: call void @__edge_tracer_trace(i64 {{[0-9]+}}, i64 {{[0-9]+}})
; CHECK-NEXT: br label %if.end

; CHECK-LABEL: if.then:
; CHECK-NEXT: call void @__edge_tracer_trace(i64 {{[0-9]+}}, i64 {{[0-9]+}})
; CHECK-NEXT: call i32 @puts
; CHECK-NEXT: call void @__edge_tracer_trace(i64 {{[0-9]+}}, i64 {{[0-9]+}})
if.then:                                          ; preds = %entry
  %call1 = tail call i32 @puts(i8* getelementptr inbounds ([10 x i8], [10 x i8]* @.str.1, i64 0, i64 0))
  br label %if.end

if.end:                                           ; preds = %if.then, %entry
  ret i32 0
}

; Function Attrs: nounwind
declare dso_local i32 @puts(i8* nocapture readonly) local_unnamed_addr #1

attributes #0 = { nounwind uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "no-frame-pointer-elim"="false" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+fxsr,+mmx,+sse,+sse2,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #1 = { nounwind "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "no-frame-pointer-elim"="false" "no-infs-fp-math"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+fxsr,+mmx,+sse,+sse2,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }

!llvm.module.flags = !{!0}
!llvm.ident = !{!1}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{!"clang version 7.1.0 "}
