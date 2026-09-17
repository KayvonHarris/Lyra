//! Abstract syntax tree definitions for Lyra.

use lyra_span::Span;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(Function),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        name: String,
        value: Expression,
        span: Span,
    },
    Return {
        value: Option<Expression>,
        span: Span,
    },
    Expression {
        expression: Expression,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Integer(i64, Span),
    Float(f64, Span),
    String(String, Span),
    Boolean(bool, Span),
    Identifier(String, Span),
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
        span: Span,
    },
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
        span: Span,
    },
}

impl Expression {
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Integer(_, span)
            | Self::Float(_, span)
            | Self::String(_, span)
            | Self::Boolean(_, span)
            | Self::Identifier(_, span)
            | Self::Unary { span, .. }
            | Self::Binary { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}
