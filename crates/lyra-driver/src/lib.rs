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
    let tokens = lyra_lexer::tokenize(source);
    let (module, diagnostics) = lyra_parser::parse(&tokens);
    CompileOutput {
        module,
        diagnostics,
    }
}
