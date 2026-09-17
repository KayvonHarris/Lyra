//! Parser for the Lyra language.

use lyra_ast::{
    BinaryOperator, Block, Expression, Function, Item, Module, Statement, UnaryOperator,
};
use lyra_diagnostics::{Diagnostic, Severity};
use lyra_lexer::{Token, TokenKind};
use lyra_span::Span;

#[must_use]
pub fn parse(tokens: &[Token]) -> (Module, Vec<Diagnostic>) {
    Parser::new(tokens).parse_module()
}

struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            current: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_module(mut self) -> (Module, Vec<Diagnostic>) {
        let mut items = Vec::new();

        while !self.at_end() {
            if let Some(item) = self.parse_item() {
                items.push(item);
            } else {
                self.synchronize_item();
            }
        }

        (Module { items }, self.diagnostics)
    }

    fn parse_item(&mut self) -> Option<Item> {
        if self.matches(|kind| matches!(kind, TokenKind::Fn)) {
            self.parse_function().map(Item::Function)
        } else {
            self.report_current("expected an item such as `fn`");
            None
        }
    }

    fn parse_function(&mut self) -> Option<Function> {
        let start = self.previous().span.start;
        let name = self.expect_identifier("expected function name after `fn`")?;
        self.expect(
            |kind| matches!(kind, TokenKind::LParen),
            "expected `(` after function name",
        )?;
        self.expect(
            |kind| matches!(kind, TokenKind::RParen),
            "expected `)` after function parameters",
        )?;
        let body = self.parse_block()?;
        let span = Span::new(start, body.span.end);

        Some(Function { name, body, span })
    }

    fn parse_block(&mut self) -> Option<Block> {
        let open = self.expect(
            |kind| matches!(kind, TokenKind::LBrace),
            "expected `{` to start block",
        )?;
        let mut statements = Vec::new();

        while !self.check(|kind| matches!(kind, TokenKind::RBrace)) && !self.at_end() {
            if let Some(statement) = self.parse_statement() {
                statements.push(statement);
            } else {
                self.synchronize_statement();
            }
        }

        let close = self.expect(
            |kind| matches!(kind, TokenKind::RBrace),
            "expected `}` to close block",
        )?;
        Some(Block {
            statements,
            span: Span::new(open.span.start, close.span.end),
        })
    }

    fn parse_statement(&mut self) -> Option<Statement> {
        if self.matches(|kind| matches!(kind, TokenKind::Let)) {
            return self.parse_let_statement();
        }
        if self.matches(|kind| matches!(kind, TokenKind::Return)) {
            return self.parse_return_statement();
        }

        let expression = self.parse_expression()?;
        let start = expression.span().start;
        let semicolon = self.expect(
            |kind| matches!(kind, TokenKind::Semicolon),
            "expected `;` after expression",
        )?;
        Some(Statement::Expression {
            expression,
            span: Span::new(start, semicolon.span.end),
        })
    }

    fn parse_let_statement(&mut self) -> Option<Statement> {
        let start = self.previous().span.start;
        let name = self.expect_identifier("expected variable name after `let`")?;
        self.expect(
            |kind| matches!(kind, TokenKind::Equal),
            "expected `=` after variable name",
        )?;
        let value = self.parse_expression()?;
        let semicolon = self.expect(
            |kind| matches!(kind, TokenKind::Semicolon),
            "expected `;` after let statement",
        )?;

        Some(Statement::Let {
            name,
            value,
            span: Span::new(start, semicolon.span.end),
        })
    }

    fn parse_return_statement(&mut self) -> Option<Statement> {
        let start = self.previous().span.start;
        let value = if self.check(|kind| matches!(kind, TokenKind::Semicolon)) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        let semicolon = self.expect(
            |kind| matches!(kind, TokenKind::Semicolon),
            "expected `;` after return statement",
        )?;

        Some(Statement::Return {
            value,
            span: Span::new(start, semicolon.span.end),
        })
    }

    fn parse_expression(&mut self) -> Option<Expression> {
        self.parse_binary(1)
    }

    fn parse_binary(&mut self, minimum_precedence: u8) -> Option<Expression> {
        let mut left = self.parse_unary()?;

        while let Some((operator, precedence)) = self.current_binary_operator() {
            if precedence < minimum_precedence {
                break;
            }

            self.advance();
            let right = self.parse_binary(precedence + 1)?;
            let span = Span::new(left.span().start, right.span().end);
            left = Expression::Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
                span,
            };
        }

        Some(left)
    }

    fn parse_unary(&mut self) -> Option<Expression> {
        if self.matches(|kind| matches!(kind, TokenKind::Minus)) {
            let start = self.previous().span.start;
            let operand = self.parse_unary()?;
            let span = Span::new(start, operand.span().end);
            return Some(Expression::Unary {
                operator: UnaryOperator::Negate,
                operand: Box::new(operand),
                span,
            });
        }
        if self.matches(|kind| matches!(kind, TokenKind::Bang)) {
            let start = self.previous().span.start;
            let operand = self.parse_unary()?;
            let span = Span::new(start, operand.span().end);
            return Some(Expression::Unary {
                operator: UnaryOperator::Not,
                operand: Box::new(operand),
                span,
            });
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<Expression> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Integer(value) => Some(Expression::Integer(value, token.span)),
            TokenKind::Float(value) => Some(Expression::Float(value, token.span)),
            TokenKind::String(value) => Some(Expression::String(value, token.span)),
            TokenKind::True => Some(Expression::Boolean(true, token.span)),
            TokenKind::False => Some(Expression::Boolean(false, token.span)),
            TokenKind::Identifier(name) => Some(Expression::Identifier(name, token.span)),
            TokenKind::LParen => {
                let expression = self.parse_expression()?;
                self.expect(
                    |kind| matches!(kind, TokenKind::RParen),
                    "expected `)` after expression",
                )?;
                Some(expression)
            }
            _ => {
                self.report("expected expression", token.span);
                None
            }
        }
    }

    fn current_binary_operator(&self) -> Option<(BinaryOperator, u8)> {
        match &self.peek().kind {
            TokenKind::OrOr => Some((BinaryOperator::Or, 1)),
            TokenKind::AndAnd => Some((BinaryOperator::And, 2)),
            TokenKind::EqualEqual => Some((BinaryOperator::Equal, 3)),
            TokenKind::BangEqual => Some((BinaryOperator::NotEqual, 3)),
            TokenKind::Less => Some((BinaryOperator::Less, 4)),
            TokenKind::LessEqual => Some((BinaryOperator::LessEqual, 4)),
            TokenKind::Greater => Some((BinaryOperator::Greater, 4)),
            TokenKind::GreaterEqual => Some((BinaryOperator::GreaterEqual, 4)),
            TokenKind::Plus => Some((BinaryOperator::Add, 5)),
            TokenKind::Minus => Some((BinaryOperator::Subtract, 5)),
            TokenKind::Star => Some((BinaryOperator::Multiply, 6)),
            TokenKind::Slash => Some((BinaryOperator::Divide, 6)),
            TokenKind::Percent => Some((BinaryOperator::Remainder, 6)),
            _ => None,
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Option<String> {
        match self.peek().kind.clone() {
            TokenKind::Identifier(name) => {
                self.advance();
                Some(name)
            }
            _ => {
                self.report_current(message);
                None
            }
        }
    }

    fn expect<F>(&mut self, predicate: F, message: &str) -> Option<Token>
    where
        F: FnOnce(&TokenKind) -> bool,
    {
        if predicate(&self.peek().kind) {
            Some(self.advance().clone())
        } else {
            self.report_current(message);
            None
        }
    }

    fn matches<F>(&mut self, predicate: F) -> bool
    where
        F: FnOnce(&TokenKind) -> bool,
    {
        if predicate(&self.peek().kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check<F>(&self, predicate: F) -> bool
    where
        F: FnOnce(&TokenKind) -> bool,
    {
        predicate(&self.peek().kind)
    }

    fn at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.current)
            .or_else(|| self.tokens.last())
            .expect("parser requires an EOF token")
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn advance(&mut self) -> &Token {
        if !self.at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn report_current(&mut self, message: &str) {
        self.report(message, self.peek().span);
    }

    fn report(&mut self, message: &str, span: Span) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: message.to_owned(),
            span: Some(span),
        });
    }

    fn synchronize_statement(&mut self) {
        while !self.at_end() {
            if self.matches(|kind| matches!(kind, TokenKind::Semicolon)) {
                return;
            }
            if self.check(|kind| matches!(kind, TokenKind::RBrace)) {
                return;
            }
            self.advance();
        }
    }

    fn synchronize_item(&mut self) {
        while !self.at_end() {
            if self.check(|kind| matches!(kind, TokenKind::Fn)) {
                return;
            }
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lyra_lexer::tokenize;

    fn parse_source(source: &str) -> (Module, Vec<Diagnostic>) {
        let lexed = tokenize(source);
        assert!(lexed.diagnostics.is_empty());
        parse(&lexed.tokens)
    }

    #[test]
    fn parses_empty_module() {
        let (module, diagnostics) = parse_source("");
        assert!(diagnostics.is_empty());
        assert!(module.items.is_empty());
    }

    #[test]
    fn parses_function_with_let_and_return() {
        let (module, diagnostics) =
            parse_source("fn main() { let speed = 60 + 5 * 2; return speed; }");
        assert!(diagnostics.is_empty());
        assert_eq!(module.items.len(), 1);

        let Item::Function(function) = &module.items[0];
        assert_eq!(function.name, "main");
        assert_eq!(function.body.statements.len(), 2);
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        let (module, diagnostics) = parse_source("fn main() { return 1 + 2 * 3; }");
        assert!(diagnostics.is_empty());
        let Item::Function(function) = &module.items[0];
        let Statement::Return {
            value: Some(expression),
            ..
        } = &function.body.statements[0]
        else {
            panic!("expected return expression");
        };

        let Expression::Binary {
            operator: BinaryOperator::Add,
            right,
            ..
        } = expression
        else {
            panic!("expected addition at expression root");
        };
        assert!(matches!(
            right.as_ref(),
            Expression::Binary {
                operator: BinaryOperator::Multiply,
                ..
            }
        ));
    }

    #[test]
    fn parses_boolean_logic_and_comparisons() {
        let (_, diagnostics) = parse_source("fn main() { return speed >= 65 && true; }");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_missing_semicolon() {
        let (_, diagnostics) = parse_source("fn main() { let speed = 65 }");
        assert!(!diagnostics.is_empty());
        assert!(diagnostics[0].message.contains("`;`"));
    }

    #[test]
    fn recovers_to_parse_later_function() {
        let (module, diagnostics) = parse_source("oops fn second() { return 2; }");
        assert!(!diagnostics.is_empty());
        assert_eq!(module.items.len(), 1);
        let Item::Function(function) = &module.items[0];
        assert_eq!(function.name, "second");
    }
}
