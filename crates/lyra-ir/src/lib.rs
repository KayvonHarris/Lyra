//! Compiler-owned intermediate representation for Lyra.
//!
//! Lyra IR sits between the validated AST and backend-specific IRs such as
//! LLVM IR. It preserves Lyra semantics without coupling the language to a
//! particular code-generation framework.

use lyra_ast::{BinaryOperator, UnaryOperator};
use lyra_span::Span;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Bind {
        name: String,
        value: Value,
        span: Span,
    },
    Evaluate {
        value: Value,
        span: Span,
    },
    Return {
        value: Option<Value>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64, Span),
    Float(f64, Span),
    String(String, Span),
    Boolean(bool, Span),
    Local(String, Span),
    Unary {
        operator: UnaryOperator,
        operand: Box<Value>,
        span: Span,
    },
    Binary {
        left: Box<Value>,
        operator: BinaryOperator,
        right: Box<Value>,
        span: Span,
    },
}

impl Value {
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Integer(_, span)
            | Self::Float(_, span)
            | Self::String(_, span)
            | Self::Boolean(_, span)
            | Self::Local(_, span)
            | Self::Unary { span, .. }
            | Self::Binary { span, .. } => *span,
        }
    }
}

#[must_use]
pub fn lower(module: &lyra_ast::Module) -> Module {
    Module {
        functions: module
            .items
            .iter()
            .map(|item| match item {
                lyra_ast::Item::Function(function) => lower_function(function),
            })
            .collect(),
    }
}

fn lower_function(function: &lyra_ast::Function) -> Function {
    Function {
        name: function.name.clone(),
        body: Block {
            instructions: function
                .body
                .statements
                .iter()
                .map(lower_statement)
                .collect(),
        },
        span: function.span,
    }
}

fn lower_statement(statement: &lyra_ast::Statement) -> Instruction {
    match statement {
        lyra_ast::Statement::Let { name, value, span } => Instruction::Bind {
            name: name.clone(),
            value: lower_expression(value),
            span: *span,
        },
        lyra_ast::Statement::Return { value, span } => Instruction::Return {
            value: value.as_ref().map(lower_expression),
            span: *span,
        },
        lyra_ast::Statement::Expression { expression, span } => Instruction::Evaluate {
            value: lower_expression(expression),
            span: *span,
        },
    }
}

fn lower_expression(expression: &lyra_ast::Expression) -> Value {
    match expression {
        lyra_ast::Expression::Integer(value, span) => Value::Integer(*value, *span),
        lyra_ast::Expression::Float(value, span) => Value::Float(*value, *span),
        lyra_ast::Expression::String(value, span) => Value::String(value.clone(), *span),
        lyra_ast::Expression::Boolean(value, span) => Value::Boolean(*value, *span),
        lyra_ast::Expression::Identifier(name, span) => Value::Local(name.clone(), *span),
        lyra_ast::Expression::Unary {
            operator,
            operand,
            span,
        } => Value::Unary {
            operator: *operator,
            operand: Box::new(lower_expression(operand)),
            span: *span,
        },
        lyra_ast::Expression::Binary {
            left,
            operator,
            right,
            span,
        } => Value::Binary {
            left: Box::new(lower_expression(left)),
            operator: *operator,
            right: Box::new(lower_expression(right)),
            span: *span,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lower_source(source: &str) -> Module {
        let lexed = lyra_lexer::tokenize(source);
        assert!(lexed.diagnostics.is_empty());
        let (ast, parser_diagnostics) = lyra_parser::parse(&lexed.tokens);
        assert!(parser_diagnostics.is_empty());
        let analysis = lyra_semantics::analyze(&ast);
        assert!(analysis.diagnostics.is_empty());
        lower(&ast)
    }

    #[test]
    fn lowers_function_and_bindings() {
        let module = lower_source("fn main() { let speed = 65; return speed; }");
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "main");
        assert_eq!(module.functions[0].body.instructions.len(), 2);
        assert!(matches!(
            &module.functions[0].body.instructions[0],
            Instruction::Bind { name, .. } if name == "speed"
        ));
        assert!(matches!(
            &module.functions[0].body.instructions[1],
            Instruction::Return {
                value: Some(Value::Local(name, _)),
                ..
            } if name == "speed"
        ));
    }

    #[test]
    fn lowers_binary_expression_tree() {
        let module = lower_source("fn main() { return 60 + 5 * 2; }");
        let Instruction::Return {
            value: Some(value),
            ..
        } = &module.functions[0].body.instructions[0]
        else {
            panic!("expected return instruction");
        };

        assert!(matches!(
            value,
            Value::Binary {
                operator: BinaryOperator::Add,
                right,
                ..
            } if matches!(
                right.as_ref(),
                Value::Binary {
                    operator: BinaryOperator::Multiply,
                    ..
                }
            )
        ));
    }
}
