; Hulk LLVM IR (intermediate code)
declare i32 @printf(i8*, ...)
@.fmt.number = private unnamed_addr constant [4 x i8] c"%g\0A\00"
@.fmt.string = private unnamed_addr constant [4 x i8] c"%s\0A\00"
@.fmt.bool = private unnamed_addr constant [4 x i8] c"%d\0A\00"

define i32 @main() {
entry:
  %t0 = alloca double
  store double 10.0, double* %t0
  %t1 = alloca double
  store double 20.0, double* %t1
  %t2 = load double, double* %t0
  %t3 = load double, double* %t1
  %t4 = fadd double %t2, %t3
  %t5 = alloca double
  store double %t4, double* %t5
  %t6 = load double, double* %t5
  %t7 = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.fmt.number, i64 0, i64 0), double %t6)
  ret i32 0
}