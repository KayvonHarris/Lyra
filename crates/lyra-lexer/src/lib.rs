//! Lexical analysis for Lyra source code.

use lyra_span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Identifier,
    Integer,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[must_use]
pub fn tokenize(source: &str) -> Vec<Token> {
    vec![Token {
        kind: TokenKind::Eof,
        span: Span::new(source.len(), source.len()),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_eof() {
        let tokens = tokenize("");
        assert_eq!(tokens.last().map(|t| &t.kind), Some(&TokenKind::Eof));
    }
}
