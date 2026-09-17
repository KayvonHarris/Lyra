//! Semantic analysis for Lyra.

use std::collections::HashMap;

use lyra_ast::{BinaryOperator, Expression, Item, Module, Statement, UnaryOperator};
use lyra_diagnostics::{Diagnostic, Severity};
use lyra_span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Integer,
    Float,
    String,
    Boolean,
    Unit,
    Unknown,
}

#[derive(Debug, Default)]
pub struct Analysis {
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn analyze(module: &Module) -> Analysis {
    Analyzer::default().analyze(module)
}

#[derive(Default)]
struct Analyzer {
    diagnostics: Vec<Diagnostic>,
    scopes: Vec<HashMap<String, Type>>,
}

impl Analyzer {
    fn analyze(mut self, module: &Module) -> Analysis {
        for item in &module.items {
            match item {
                Item::Function(function) => {
                    self.push_scope();
                    for statement in &function.body.statements {
                        self.check_statement(statement);
                    }
                    self.pop_scope();
                }
            }
        }

        Analysis {
            diagnostics: self.diagnostics,
        }
    }

    fn check_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Let {
                name, value, span, ..
            } => {
                let ty = self.check_expression(value);
                let duplicate = self
                    .scopes
                    .last()
                    .is_some_and(|scope| scope.contains_key(name));

                if duplicate {
                    self.error(
                        format!("variable `{name}` is already defined in this scope"),
                        *span,
                    );
                } else if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(name.clone(), ty);
                } else {
                    self.error("internal semantic error: no active scope", *span);
                }
            }
            Statement::Return { value, .. } => {
                if let Some(value) = value {
                    self.check_expression(value);
                }
            }
            Statement::Expression { expression, .. } => {
                self.check_expression(expression);
            }
        }
    }

    fn check_expression(&mut self, expression: &Expression) -> Type {
        match expression {
            Expression::Integer(_, _) => Type::Integer,
            Expression::Float(_, _) => Type::Float,
            Expression::String(_, _) => Type::String,
            Expression::Boolean(_, _) => Type::Boolean,
            Expression::Identifier(name, span) => self.resolve(name, *span),
            Expression::Unary {
                operator,
                operand,
                span,
            } => {
                let operand_type = self.check_expression(operand);
                self.check_unary(*operator, operand_type, *span)
            }
            Expression::Binary {
                left,
                operator,
                right,
                span,
            } => {
                let left_type = self.check_expression(left);
                let right_type = self.check_expression(right);
                self.check_binary(left_type, *operator, right_type, *span)
            }
        }
    }

    fn check_unary(&mut self, operator: UnaryOperator, operand: Type, span: Span) -> Type {
        match operator {
            UnaryOperator::Negate if matches!(operand, Type::Integer | Type::Float) => operand,
            UnaryOperator::Not if operand == Type::Boolean => Type::Boolean,
            _ if operand == Type::Unknown => Type::Unknown,
            UnaryOperator::Negate => {
                self.error("unary `-` requires a numeric operand", span);
                Type::Unknown
            }
            UnaryOperator::Not => {
                self.error("unary `!` requires a boolean operand", span);
                Type::Unknown
            }
        }
    }

    fn check_binary(
        &mut self,
        left: Type,
        operator: BinaryOperator,
        right: Type,
        span: Span,
    ) -> Type {
        if left == Type::Unknown || right == Type::Unknown {
            return Type::Unknown;
        }

        match operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder => self.check_arithmetic(left, right, span),
            BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => {
                if Self::numeric_pair(left, right) {
                    Type::Boolean
                } else {
                    self.error("comparison operators require numeric operands", span);
                    Type::Unknown
                }
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                if left == right || Self::numeric_pair(left, right) {
                    Type::Boolean
                } else {
                    self.error("equality operands must have compatible types", span);
                    Type::Unknown
                }
            }
            BinaryOperator::And | BinaryOperator::Or => {
                if left == Type::Boolean && right == Type::Boolean {
                    Type::Boolean
                } else {
                    self.error("logical operators require boolean operands", span);
                    Type::Unknown
                }
            }
        }
    }

    fn check_arithmetic(&mut self, left: Type, right: Type, span: Span) -> Type {
        if !Self::numeric_pair(left, right) {
            self.error("arithmetic operators require numeric operands", span);
            return Type::Unknown;
        }

        if left == Type::Float || right == Type::Float {
            Type::Float
        } else {
            Type::Integer
        }
    }

    fn numeric_pair(left: Type, right: Type) -> bool {
        matches!(left, Type::Integer | Type::Float) && matches!(right, Type::Integer | Type::Float)
    }

    fn resolve(&mut self, name: &str, span: Span) -> Type {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return *ty;
            }
        }

        self.error(format!("unknown identifier `{name}`"), span);
        Type::Unknown
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            span: Some(span),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze_source(source: &str) -> Analysis {
        let lexed = lyra_lexer::tokenize(source);
        assert!(lexed.diagnostics.is_empty());
        let (module, parser_diagnostics) = lyra_parser::parse(&lexed.tokens);
        assert!(parser_diagnostics.is_empty());
        analyze(&module)
    }

    #[test]
    fn accepts_well_typed_program() {
        let analysis = analyze_source("fn main() { let speed = 65.0; return speed >= 60; }");
        assert!(analysis.diagnostics.is_empty());
    }

    #[test]
    fn reports_unknown_identifier() {
        let analysis = analyze_source("fn main() { return speed; }");
        assert!(analysis.diagnostics[0].message.contains("unknown identifier"));
    }

    #[test]
    fn reports_duplicate_variable() {
        let analysis = analyze_source("fn main() { let speed = 1; let speed = 2; }");
        assert!(analysis.diagnostics[0].message.contains("already defined"));
    }

    #[test]
    fn rejects_invalid_arithmetic() {
        let analysis = analyze_source("fn main() { return true + 1; }");
        assert!(analysis.diagnostics[0].message.contains("numeric operands"));
    }

    #[test]
    fn promotes_mixed_numeric_arithmetic_to_float() {
        let analysis = analyze_source("fn main() { let speed = 60 + 5.5; return speed; }");
        assert!(analysis.diagnostics.is_empty());
    }

    #[test]
    fn rejects_boolean_negation_of_number() {
        let analysis = analyze_source("fn main() { return !42; }");
        assert!(analysis.diagnostics[0].message.contains("boolean operand"));
    }

    #[test]
    fn rejects_numeric_negation_of_boolean() {
        let analysis = analyze_source("fn main() { return -true; }");
        assert!(analysis.diagnostics[0].message.contains("numeric operand"));
    }

    #[test]
    fn rejects_logical_operator_on_numbers() {
        let analysis = analyze_source("fn main() { return 1 && 2; }");
        assert!(analysis.diagnostics[0].message.contains("boolean operands"));
    }

    #[test]
    fn rejects_comparison_of_non_numeric_values() {
        let analysis = analyze_source("fn main() { return \"a\" < \"b\"; }");
        assert!(analysis.diagnostics[0].message.contains("numeric operands"));
    }

    #[test]
    fn rejects_incompatible_equality() {
        let analysis = analyze_source("fn main() { return true == 1; }");
        assert!(analysis.diagnostics[0].message.contains("compatible types"));
    }

    #[test]
    fn accepts_numeric_equality_across_integer_and_float() {
        let analysis = analyze_source("fn main() { return 1 == 1.0; }");
        assert!(analysis.diagnostics.is_empty());
    }

    #[test]
    fn variables_do_not_leak_between_functions() {
        let analysis = analyze_source(
            "fn first() { let speed = 65; return speed; } fn second() { return speed; }",
        );
        assert_eq!(analysis.diagnostics.len(), 1);
        assert!(analysis.diagnostics[0].message.contains("unknown identifier"));
    }
}
