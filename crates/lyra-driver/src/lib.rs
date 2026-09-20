//! Compiler pipeline orchestration.

use lyra_ast::Module;
use lyra_diagnostics::Diagnostic;

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

#[must_use]
pub fn compile(source: &str) -> CompileOutput {
    let lexed = lyra_lexer::tokenize(source);
    let (module, parser_diagnostics) = lyra_parser::parse(&lexed.tokens);

    let mut diagnostics = lexed.diagnostics;
    diagnostics.extend(parser_diagnostics);

    if diagnostics.is_empty() {
        diagnostics.extend(lyra_semantics::analyze(&module).diagnostics);
    }

    let ir = if diagnostics.is_empty() {
        Some(lyra_ir::lower(&module))
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
    if !output.diagnostics.is_empty() {
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
