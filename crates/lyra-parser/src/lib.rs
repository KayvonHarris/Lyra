//! Parser for the Lyra language.

use lyra_ast::Module;
use lyra_diagnostics::Diagnostic;
use lyra_lexer::Token;

#[must_use]
pub fn parse(_tokens: &[Token]) -> (Module, Vec<Diagnostic>) {
    (Module::default(), Vec::new())
}
