//! Compiler pipeline orchestration.

use std::collections::HashMap;

use lyra_ast::Module;
use lyra_diagnostics::{Diagnostic, Severity};

#[derive(Debug)]
pub struct CompileOutput {
    pub module: Module,
    pub ir: Option<lyra_ir::Module>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub enum BackendError {
    Frontend(Vec<Diagnostic>),
    Codegen(lyra_codegen_llvm::CodegenError),
}

fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

#[must_use]
pub fn compile(source: &str) -> CompileOutput {
    let lexed = lyra_lexer::tokenize(source);
    let (module, parser_diagnostics) = lyra_parser::parse(&lexed.tokens);

    let mut diagnostics = lexed.diagnostics;
    diagnostics.extend(parser_diagnostics);

    let mut function_signatures = HashMap::new();
    if !has_errors(&diagnostics) {
        let analysis = lyra_semantics::analyze(&module);
        function_signatures = analysis
            .function_signatures
            .iter()
            .map(|(name, signature)| {
                let convert_type = |ty| match ty {
                    lyra_semantics::Type::Integer => lyra_ir::Type::Integer,
                    lyra_semantics::Type::Float => lyra_ir::Type::Float,
                    lyra_semantics::Type::String => lyra_ir::Type::String,
                    lyra_semantics::Type::Boolean => lyra_ir::Type::Boolean,
                    lyra_semantics::Type::Unit => lyra_ir::Type::Unit,
                    lyra_semantics::Type::Unknown => lyra_ir::Type::Unknown,
                };
                (
                    name.clone(),
                    (
                        signature.parameters.iter().copied().map(convert_type).collect(),
                        convert_type(signature.return_type),
                    ),
                )
            })
            .collect();
        diagnostics.extend(analysis.diagnostics);
    }

    let ir = if !has_errors(&diagnostics) {
        Some(lyra_ir::lower_with_signatures(
            &module,
            &function_signatures,
        ))
    } else {
        None
    };

    CompileOutput {
        module,
        ir,
        diagnostics,
    }
}

pub fn compile_to_llvm(source: &str) -> Result<String, BackendError> {
    let output = compile(source);
    if has_errors(&output.diagnostics) {
        return Err(BackendError::Frontend(output.diagnostics));
    }

    let ir = output
        .ir
        .expect("validated compilation must produce Lyra IR");
    lyra_codegen_llvm::emit_llvm_ir(&ir).map_err(BackendError::Codegen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_does_not_count_as_frontend_error() {
        let diagnostics = vec![Diagnostic {
            severity: Severity::Warning,
            message: "warning".to_owned(),
            span: None,
        }];
        assert!(!has_errors(&diagnostics));
    }

    #[test]
    fn error_counts_as_frontend_error() {
        let diagnostics = vec![Diagnostic {
            severity: Severity::Error,
            message: "error".to_owned(),
            span: None,
        }];
        assert!(has_errors(&diagnostics));
    }

    #[test]
    fn reports_semantic_errors_through_driver() {
        let output = compile("fn main() { return missing; }");
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("unknown identifier"))
        );
        assert!(output.ir.is_none());
    }

    #[test]
    fn accepts_valid_program_through_full_frontend() {
        let output = compile("fn is_fast() -> Bool { let speed = 65.0; return speed >= 60; }");
        assert!(output.diagnostics.is_empty());
        assert!(output.ir.is_some());
    }

    #[test]
    fn emits_llvm_for_integer_program() {
        let llvm = compile_to_llvm("fn main() { return 40 + 2; }")
            .expect("valid integer program should lower to LLVM IR");
        assert!(llvm.contains("add i64 40, 2"));
        assert!(llvm.contains("trunc i64 %1 to i32"));
        assert!(llvm.contains("ret i32 %lyra.main.exit"));
    }

    #[test]
    fn does_not_lower_invalid_syntax_to_ir() {
        let output = compile("fn main() { let speed = ; }");
        assert!(!output.diagnostics.is_empty());
        assert!(output.ir.is_none());
    }
}
