//! Compiler pipeline orchestration.

use lyra_ast::Module;
use lyra_diagnostics::Diagnostic;

#[derive(Debug)]
pub struct CompileOutput {
    pub module: Module,
    pub ir: Option<lyra_ir::Module>,
    pub diagnostics: Vec<Diagnostic>,
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
        let output = compile("fn main() { let speed = 65.0; return speed >= 60; }");
        assert!(output.diagnostics.is_empty());
        assert!(output.ir.is_some());
    }

    #[test]
    fn does_not_lower_invalid_syntax_to_ir() {
        let output = compile("fn main() { let speed = ; }");
        assert!(!output.diagnostics.is_empty());
        assert!(output.ir.is_none());
    }
}
