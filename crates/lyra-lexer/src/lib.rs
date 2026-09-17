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
    AndAnd,
    OrOr,

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
            '/' if self.peek() == Some('/') => self.line_comment(),
            '/' if self.peek() == Some('*') => self.block_comment(start),
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
            '&' if self.match_char('&') => self.push(TokenKind::AndAnd, start),
            '|' if self.match_char('|') => self.push(TokenKind::OrOr, start),
            '"' => self.string(start),
            c if c.is_ascii_digit() => self.number(start),
            c if is_identifier_start(c) => self.identifier(start),
            other => self.error(
                format!("unexpected character `{other}`"),
                Span::new(start, self.pos),
            ),
        }
    }

    fn line_comment(&mut self) {
        self.advance();
        while !matches!(self.peek(), None | Some('\n')) {
            self.advance();
        }
    }

    fn block_comment(&mut self, start: usize) {
        self.advance();
        let mut depth = 1usize;

        while self.pos < self.source.len() {
            if self.peek() == Some('/') && self.peek_next() == Some('*') {
                self.advance();
                self.advance();
                depth += 1;
            } else if self.peek() == Some('*') && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                depth -= 1;
                if depth == 0 {
                    return;
                }
            } else {
                self.advance();
            }
        }

        self.error(
            "unterminated block comment".to_owned(),
            Span::new(start, self.pos),
        );
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
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
            self.advance();
        }

        let is_float = self.peek() == Some('.')
            && self.peek_next().is_some_and(|c| c.is_ascii_digit());

        if is_float {
            self.advance();
            while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
                self.advance();
            }
        }

        let text = &self.source[start..self.pos];
        let normalized = text.replace('_', "");

        let kind = if is_float {
            match normalized.parse::<f64>() {
                Ok(value) if value.is_finite() => TokenKind::Float(value),
                _ => {
                    self.error(
                        format!("invalid floating-point literal `{text}`"),
                        Span::new(start, self.pos),
                    );
                    return;
                }
            }
        } else {
            match normalized.parse::<i64>() {
                Ok(value) => TokenKind::Integer(value),
                Err(_) => {
                    self.error(
                        format!("integer literal out of range `{text}`"),
                        Span::new(start, self.pos),
                    );
                    return;
                }
            }
        };

        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn string(&mut self, start: usize) {
        let mut value = String::new();

        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    self.advance();
                    self.tokens.push(Token {
                        kind: TokenKind::String(value),
                        span: Span::new(start, self.pos),
                    });
                    return;
                }
                '\\' => {
                    let escape_start = self.pos;
                    self.advance();
                    let Some(escaped) = self.advance() else {
                        break;
                    };
                    match escaped {
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        '0' => value.push('\0'),
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        other => self.error(
                            format!("unknown escape sequence `\\{other}`"),
                            Span::new(escape_start, self.pos),
                        ),
                    }
                }
                _ => {
                    value.push(ch);
                    self.advance();
                }
            }
        }

        self.error(
            "unterminated string literal".to_owned(),
            Span::new(start, self.pos),
        );
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn error(&mut self, message: String, span: Span) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message,
            span: Some(span),
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
    fn skips_line_and_nested_block_comments() {
        assert_eq!(
            kinds("let x = 1 // speed\n/* outer /* nested */ done */ let y = 2"),
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
    fn lexes_string_escapes() {
        assert_eq!(
            kinds("\"Lyra\\nedge\\t\\\"AI\\\"\\\\\""),
            vec![TokenKind::String("Lyra\nedge\t\"AI\"\\".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn lexes_numeric_separators() {
        assert_eq!(
            kinds("1_000 65.5_0"),
            vec![TokenKind::Integer(1000), TokenKind::Float(65.50), TokenKind::Eof]
        );
    }

    #[test]
    fn reports_integer_overflow_without_panicking() {
        let output = tokenize("999999999999999999999999999999999999");
        assert_eq!(output.diagnostics.len(), 1);
        assert!(output.diagnostics[0].message.contains("out of range"));
    }

    #[test]
    fn reports_unknown_escape() {
        let output = tokenize("\"bad\\qescape\"");
        assert_eq!(output.diagnostics.len(), 1);
        assert!(output.diagnostics[0].message.contains("unknown escape"));
    }

    #[test]
    fn reports_unterminated_string() {
        let output = tokenize("\"vehicle");
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.diagnostics[0].message, "unterminated string literal");
    }

    #[test]
    fn reports_unterminated_block_comment() {
        let output = tokenize("/* never closes");
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.diagnostics[0].message, "unterminated block comment");
    }

    #[test]
    fn lexes_logical_and_comparison_operators() {
        assert_eq!(
            kinds("a >= b && b != c || c <= d"),
            vec![
                TokenKind::Identifier("a".into()),
                TokenKind::GreaterEqual,
                TokenKind::Identifier("b".into()),
                TokenKind::AndAnd,
                TokenKind::Identifier("b".into()),
                TokenKind::BangEqual,
                TokenKind::Identifier("c".into()),
                TokenKind::OrOr,
                TokenKind::Identifier("c".into()),
                TokenKind::LessEqual,
                TokenKind::Identifier("d".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn tracks_utf8_spans_in_bytes() {
        let output = tokenize("let café = 1");
        assert!(output.diagnostics.is_empty());
        assert_eq!(output.tokens[1].span, Span::new(4, 9));
    }
}
