use crate::parser::expression::FunctionDecl;

use super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::VariableInfo;

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_function_decl(&mut self, function: &FunctionDecl) {
        let Some(info) = self.functions.get(&function.name).cloned() else {
            self.semantic_error(format!(
                "Function '{}' has no inferred signature for code generation.",
                function.name
            ));
            return;
        };

        if info.param_types.len() != function.params.len() {
            self.semantic_error(format!(
                "Function '{}' has inconsistent parameter metadata.",
                function.name
            ));
            return;
        }

        let saved_body = std::mem::take(&mut self.body_lines);
        let saved_scopes = std::mem::take(&mut self.scopes);
        self.push_scope();

        let params = function
            .params
            .iter()
            .zip(info.param_types.iter().copied())
            .map(|(param, value_type)| format!("{} %{}", value_type.llvm_type(), param.name))
            .collect::<Vec<_>>();

        for (param, value_type) in function.params.iter().zip(info.param_types.iter().copied()) {
            let ptr_name = self.next_temp();
            let llvm_type = value_type.llvm_type();
            self.emit_body(format!("{ptr_name} = alloca {llvm_type}"));
            self.emit_body(format!(
                "store {llvm_type} %{}, {llvm_type}* {ptr_name}",
                param.name
            ));
            self.bind_current_scope(
                param.name.clone(),
                VariableInfo {
                    ptr_name,
                    value_type,
                },
            );
        }

        let Some(result) = self.emit_expr(&function.body) else {
            self.scopes = saved_scopes;
            self.body_lines = saved_body;
            return;
        };

        if result.value_type != info.return_type {
            self.semantic_error(format!(
                "Function '{}' returns {} but inferred signature expects {}.",
                function.name,
                result.value_type.display_name(),
                info.return_type.display_name()
            ));
            self.scopes = saved_scopes;
            self.body_lines = saved_body;
            return;
        }

        let function_body = std::mem::take(&mut self.body_lines);
        self.scopes = saved_scopes;
        self.body_lines = saved_body;

        let return_type = info.return_type.llvm_type();
        self.emit_function_line(String::new());
        self.emit_function_line(format!(
            "define {return_type} @{}({}) {{",
            info.llvm_name,
            params.join(", ")
        ));
        self.emit_function_line("entry:");
        for line in function_body {
            if line.ends_with(':') {
                self.emit_function_line(line);
            } else {
                self.emit_function_line(format!("  {line}"));
            }
        }
        self.emit_function_line(format!("  ret {return_type} {}", result.repr));
        self.emit_function_line("}");
    }
}
