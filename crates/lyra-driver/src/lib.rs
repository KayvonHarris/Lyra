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
    let (module, mut diagnostics) = lyra_parser::parse(&lexed.tokens);
    diagnostics.splice(0..0, lexed.diagnostics);
    CompileOutput {
        module,
        diagnostics,
    }
}
