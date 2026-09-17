//! Compiler pipeline orchestration.

use lyra_ast::Module;
use lyra_diagnostics::Diagnostic;

#[derive(Debug)]
pub struct CompileOutput {
    pub module: Module,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn compile(source: &str) -> CompileOutput {
    let lexed = lyra_lexer::tokenize(source);
    let (module, parser_diagnostics) = lyra_parser::parse(&lexed.tokens);
    let semantic_analysis = lyra_semantics::analyze(&module);

    let mut diagnostics = lexed.diagnostics;
    diagnostics.extend(parser_diagnostics);
    diagnostics.extend(semantic_analysis.diagnostics);

    CompileOutput {
        module,
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
    }

    #[test]
    fn accepts_valid_program_through_full_frontend() {
        let output = compile("fn main() { let speed = 65.0; return speed >= 60; }");
        assert!(output.diagnostics.is_empty());
    }
}
