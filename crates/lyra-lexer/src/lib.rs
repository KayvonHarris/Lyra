//! Lexical analysis for Lyra source code.

use lyra_diagnostics::{Diagnostic, Severity};
use lyra_span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Integer(i64),
    Float(f64),
    String(String),

    Fn,
    Let,
    Var,
    Struct,
    Enum,
    If,
    Else,
    For,
    While,
    Return,
    True,
    False,
    Import,
    As,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Colon,
    Semicolon,
    Arrow,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    EqualEqual,
    Bang,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn tokenize(source: &str) -> LexOutput {
    Lexer::new(source).tokenize()
}

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            pos: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn tokenize(mut self) -> LexOutput {
        while self.pos < self.source.len() {
            self.scan_token();
        }

        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.source.len(), self.source.len()),
        });

        LexOutput {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
        }
    }

    fn scan_token(&mut self) {
        let start = self.pos;
        let Some(ch) = self.advance() else {
            return;
        };

        match ch {
            c if c.is_whitespace() => {}
            '/' if self.peek() == Some('/') => {
                while !matches!(self.peek(), None | Some('\n')) {
                    self.advance();
                }
            }
            '(' => self.push(TokenKind::LParen, start),
            ')' => self.push(TokenKind::RParen, start),
            '{' => self.push(TokenKind::LBrace, start),
            '}' => self.push(TokenKind::RBrace, start),
            '[' => self.push(TokenKind::LBracket, start),
            ']' => self.push(TokenKind::RBracket, start),
            ',' => self.push(TokenKind::Comma, start),
            '.' => self.push(TokenKind::Dot, start),
            ':' => self.push(TokenKind::Colon, start),
            ';' => self.push(TokenKind::Semicolon, start),
            '+' => self.push(TokenKind::Plus, start),
            '*' => self.push(TokenKind::Star, start),
            '%' => self.push(TokenKind::Percent, start),
            '-' if self.match_char('>') => self.push(TokenKind::Arrow, start),
            '-' => self.push(TokenKind::Minus, start),
            '/' => self.push(TokenKind::Slash, start),
            '=' if self.match_char('=') => self.push(TokenKind::EqualEqual, start),
            '=' => self.push(TokenKind::Equal, start),
            '!' if self.match_char('=') => self.push(TokenKind::BangEqual, start),
            '!' => self.push(TokenKind::Bang, start),
            '<' if self.match_char('=') => self.push(TokenKind::LessEqual, start),
            '<' => self.push(TokenKind::Less, start),
            '>' if self.match_char('=') => self.push(TokenKind::GreaterEqual, start),
            '>' => self.push(TokenKind::Greater, start),
            '"' => self.string(start),
            c if c.is_ascii_digit() => self.number(start),
            c if is_identifier_start(c) => self.identifier(start),
            other => self.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: format!("unexpected character `{other}`"),
                span: Some(Span::new(start, self.pos)),
            }),
        }
    }

    fn identifier(&mut self, start: usize) {
        while self.peek().is_some_and(is_identifier_continue) {
            self.advance();
        }

        let text = &self.source[start..self.pos];
        let kind = match text {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "var" => TokenKind::Var,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            _ => TokenKind::Identifier(text.to_owned()),
        };

        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn number(&mut self, start: usize) {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }

        let is_float = self.peek() == Some('.')
            && self
                .peek_next()
                .is_some_and(|c| c.is_ascii_digit());

        if is_float {
            self.advance();
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.advance();
            }
        }

        let text = &self.source[start..self.pos];
        let kind = if is_float {
            TokenKind::Float(text.parse().expect("lexer validated float"))
        } else {
            TokenKind::Integer(text.parse().expect("lexer validated integer"))
        };

        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn string(&mut self, start: usize) {
        let content_start = self.pos;
        while !matches!(self.peek(), None | Some('"')) {
            self.advance();
        }

        if self.peek().is_none() {
            self.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: "unterminated string literal".to_owned(),
                span: Some(Span::new(start, self.pos)),
            });
            return;
        }

        let value = self.source[content_start..self.pos].to_owned();
        self.advance();
        self.tokens.push(Token {
            kind: TokenKind::String(value),
            span: Span::new(start, self.pos),
        });
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.source[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let mut chars = self.source[self.pos..].chars();
        chars.next()?;
        chars.next()
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        tokenize(source).tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn appends_eof() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn lexes_initial_lyra_function() {
        assert_eq!(
            kinds("fn main() { let speed = 65.0; return speed; }"),
            vec![
                TokenKind::Fn,
                TokenKind::Identifier("main".into()),
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::Let,
                TokenKind::Identifier("speed".into()),
                TokenKind::Equal,
                TokenKind::Float(65.0),
                TokenKind::Semicolon,
                TokenKind::Return,
                TokenKind::Identifier("speed".into()),
                TokenKind::Semicolon,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn skips_line_comments() {
        assert_eq!(
            kinds("let x = 1 // vehicle speed\nlet y = 2"),
            vec![
                TokenKind::Let,
                TokenKind::Identifier("x".into()),
                TokenKind::Equal,
                TokenKind::Integer(1),
                TokenKind::Let,
                TokenKind::Identifier("y".into()),
                TokenKind::Equal,
                TokenKind::Integer(2),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn reports_unterminated_string() {
        let output = tokenize("\"vehicle");
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.diagnostics[0].message, "unterminated string literal");
    }

    #[test]
    fn tracks_utf8_spans_in_bytes() {
        let output = tokenize("let café = 1");
        assert!(output.diagnostics.is_empty());
        assert_eq!(output.tokens[1].span, Span::new(4, 9));
    }
}
