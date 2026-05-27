use super::super::backend::LlvmBackend;

pub(in crate::codegen::llvm) fn format_ptr_global(name: &str, bytes: usize) -> String {
    format!(
        "getelementptr inbounds ([{} x i8], [{} x i8]* {}, i64 0, i64 0)",
        bytes, bytes, name
    )
}

pub(in crate::codegen::llvm) fn escape_llvm_string(value: &str) -> String {
    let mut escaped = String::new();

    for byte in value.as_bytes() {
        match *byte {
            b'\\' => escaped.push_str("\\5C"),
            b'"' => escaped.push_str("\\22"),
            32..=126 => escaped.push(*byte as char),
            _ => escaped.push_str(&format!("\\{:02X}", byte)),
        }
    }

    escaped.push_str("\\00");
    escaped
}

pub(in crate::codegen::llvm) fn format_double(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        let text = format!("{value:.10}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn compose_module(&self) -> String {
        let mut out = vec![
            "; Hulk LLVM IR (intermediate code)".to_string(),
            "declare i32 @printf(i8*, ...)".to_string(),
            "declare i32 @asprintf(i8**, i8*, ...)".to_string(),
            "declare i32 @strcmp(i8*, i8*)".to_string(),
            "declare i32 @rand()".to_string(),
            "declare i64 @time(i64*)".to_string(),
            "declare void @srand(i32)".to_string(),
            "declare i8* @malloc(i64)".to_string(),
            "declare double @llvm.sin.f64(double)".to_string(),
            "declare double @llvm.cos.f64(double)".to_string(),
            "declare double @llvm.sqrt.f64(double)".to_string(),
            "declare double @llvm.exp.f64(double)".to_string(),
            "declare double @llvm.log.f64(double)".to_string(),
            "declare double @llvm.pow.f64(double, double)".to_string(),
            "@.fmt.number = private unnamed_addr constant [4 x i8] c\"%g\\0A\\00\"".to_string(),
            "@.fmt.string = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\"".to_string(),
            "@.fmt.bool = private unnamed_addr constant [4 x i8] c\"%d\\0A\\00\"".to_string(),
            "@.bool.true = private unnamed_addr constant [5 x i8] c\"true\\00\"".to_string(),
            "@.bool.false = private unnamed_addr constant [6 x i8] c\"false\\00\"".to_string(),
            "@.fmt.concat.ss = private unnamed_addr constant [5 x i8] c\"%s%s\\00\"".to_string(),
            "@.fmt.concat.sn = private unnamed_addr constant [5 x i8] c\"%s%g\\00\"".to_string(),
            "@.fmt.concat.ns = private unnamed_addr constant [5 x i8] c\"%g%s\\00\"".to_string(),
            "@.fmt.concat.bs = private unnamed_addr constant [5 x i8] c\"%s%d\\00\"".to_string(),
            "@.fmt.concat.sb = private unnamed_addr constant [5 x i8] c\"%d%s\\00\"".to_string(),
            "@.str.space = private unnamed_addr constant [2 x i8] c\" \\00\"".to_string(),
        ];

        out.extend(self.global_lines.clone());
        out.extend(self.function_lines.clone());
        out.push(String::new());
        out.push("define i32 @main() {".to_string());
        out.push("entry:".to_string());
        out.push("  %t_seed_raw = call i64 @time(i64* null)".to_string());
        out.push("  %t_seed_i32 = trunc i64 %t_seed_raw to i32".to_string());
        out.push("  call void @srand(i32 %t_seed_i32)".to_string());

        for line in &self.body_lines {
            if line.ends_with(':') {
                out.push(line.clone());
            } else {
                out.push(format!("  {line}"));
            }
        }

        out.push("  ret i32 0".to_string());
        out.push("}".to_string());

        out.join("\n")
    }
}
