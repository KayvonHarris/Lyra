//! Semantic analysis for Lyra.

use std::collections::{HashMap, HashSet};

use lyra_ast::{BinaryOperator, Expression, Item, Module, Statement, TypeName, UnaryOperator};
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

#[derive(Debug, Clone)]
struct FunctionSignature {
    parameters: Vec<Type>,
    return_type: Type,
}

#[derive(Default)]
struct Analyzer {
    diagnostics: Vec<Diagnostic>,
    scopes: Vec<HashMap<String, Type>>,
    functions: HashMap<String, FunctionSignature>,
    current_return_type: Type,
}

impl Analyzer {
    fn analyze(mut self, module: &Module) -> Analysis {
        let mut functions = HashSet::new();

        for item in &module.items {
            match item {
                Item::Function(function) if !functions.insert(function.name.clone()) => {
                    self.error(
                        format!("function `{}` is already defined", function.name),
                        function.span,
                    );
                }
                Item::Function(function) => {
                    if function.name == "main" && !function.parameters.is_empty() {
                        self.error("`main` cannot declare parameters yet", function.span);
                    }
                    if function.name == "main" {
                        let return_type = function
                            .return_type
                            .as_ref()
                            .map_or(Type::Integer, Self::type_from_name);
                        if return_type != Type::Integer {
                            self.error("`main` must return Int", function.span);
                        }
                    }
                    self.functions.insert(
                            function.name.clone(),
                            FunctionSignature {
                                parameters: function
                                    .parameters
                                    .iter()
                                    .map(|parameter| {
                                        parameter
                                            .type_name
                                            .as_ref()
                                            .map_or(Type::Integer, Self::type_from_name)
                                    })
                                    .collect(),
                                return_type: function
                                    .return_type
                                    .as_ref()
                                    .map_or(Type::Integer, Self::type_from_name),
                            },
                        );
                }
            }
        }

        for item in &module.items {
            match item {
                Item::Function(function) => {
                    self.current_return_type = function
                        .return_type
                        .as_ref()
                        .map_or(Type::Integer, Self::type_from_name);
                    self.push_scope();
                    for parameter in &function.parameters {
                        let duplicate = self
                            .scopes
                            .last()
                            .is_some_and(|scope| scope.contains_key(&parameter.name));
                        if duplicate {
                            self.error(
                                format!(
                                    "parameter `{}` is already defined in this function",
                                    parameter.name
                                ),
                                parameter.span,
                            );
                        } else if let Some(scope) = self.scopes.last_mut() {
                            let ty = parameter
                                .type_name
                                .as_ref()
                                .map_or(Type::Integer, Self::type_from_name);
                            scope.insert(parameter.name.clone(), ty);
                        }
                    }
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
            Statement::Return { value, span } => {
                let actual = value
                    .as_ref()
                    .map_or(Type::Unit, |value| self.check_expression(value));
                if actual != Type::Unknown && actual != self.current_return_type {
                    self.error(
                        format!(
                            "return type mismatch: expected {:?} but found {:?}",
                            self.current_return_type, actual
                        ),
                        *span,
                    );
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
            Expression::Call {
                callee,
                arguments,
                span,
            } => {
                if callee == "main" {
                    self.error(
                        "`main` is the program entry point and cannot be called",
                        *span,
                    );
                    return Type::Unknown;
                }
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expression(argument))
                    .collect::<Vec<_>>();
                match self.functions.get(callee).cloned() {
                    Some(signature) if signature.parameters.len() == arguments.len() => {
                        let mut valid = true;
                        for (index, (actual, expected)) in
                            argument_types.iter().zip(&signature.parameters).enumerate()
                        {
                            if *actual != Type::Unknown && actual != expected {
                                self.error(
                                    format!(
                                        "argument {} to `{callee}` expects {:?} but found {:?}",
                                        index + 1,
                                        expected,
                                        actual
                                    ),
                                    arguments[index].span(),
                                );
                                valid = false;
                            }
                        }
                        if valid {
                            signature.return_type
                        } else {
                            Type::Unknown
                        }
                    }
                    Some(signature) => {
                        self.error(
                            format!(
                                "function `{callee}` expects {} arguments but received {}",
                                signature.parameters.len(),
                                arguments.len()
                            ),
                            *span,
                        );
                        Type::Unknown
                    }
                    None => {
                        self.error(format!("unknown function `{callee}`"), *span);
                        Type::Unknown
                    }
                }
            }
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

    fn type_from_name(type_name: &TypeName) -> Type {
        match type_name.name.as_str() {
            "Int" => Type::Integer,
            "Float" => Type::Float,
            "String" => Type::String,
            "Bool" => Type::Boolean,
            "Unit" => Type::Unit,
            _ => Type::Unknown,
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
    fn accepts_function_parameters_and_calls() {
        let analysis =
            analyze_source("fn add(a, b) { return a + b; } fn main() { return add(20, 22); }");
        assert!(analysis.diagnostics.is_empty());
    }

    #[test]
    fn accepts_typed_function_call() {
        let analysis = analyze_source(
            "fn add(a: Int, b: Int) -> Int { return a + b; } fn main() -> Int { return add(20, 22); }",
        );
        assert!(analysis.diagnostics.is_empty());
    }

    #[test]
    fn rejects_typed_argument_mismatch() {
        let analysis = analyze_source(
            "fn add(a: Int, b: Int) -> Int { return a + b; } fn main() -> Int { return add(true, 22); }",
        );
        assert!(analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("argument 1") && diagnostic.message.contains("Integer")
        }));
    }

    #[test]
    fn rejects_typed_return_mismatch() {
        let analysis = analyze_source("fn answer() -> Bool { return 42; }");
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.message.contains("return type mismatch") })
        );
    }

    #[test]
    fn rejects_main_parameters() {
        let analysis = analyze_source("fn main(argc: Int) -> Int { return argc; }");
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.message.contains("cannot declare parameters") })
        );
    }

    #[test]
    fn rejects_calling_main() {
        let analysis =
            analyze_source("fn main() -> Int { return 0; } fn helper() -> Int { return main(); }");
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.message.contains("cannot be called") })
        );
    }

    #[test]
    fn rejects_non_integer_main_return_type() {
        let analysis = analyze_source("fn main() -> Bool { return true; }");
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.message.contains("must return Int") })
        );
    }

    #[test]
    fn reports_unknown_function() {
        let analysis = analyze_source("fn main() { return missing(42); }");
        assert!(analysis.diagnostics[0].message.contains("unknown function"));
    }

    #[test]
    fn reports_wrong_argument_count() {
        let analysis =
            analyze_source("fn add(a, b) { return a + b; } fn main() { return add(42); }");
        assert!(
            analysis.diagnostics[0]
                .message
                .contains("expects 2 arguments but received 1")
        );
    }

    #[test]
    fn reports_duplicate_parameter() {
        let analysis = analyze_source("fn add(a, a) { return a; }");
        assert!(
            analysis.diagnostics[0]
                .message
                .contains("parameter `a` is already defined")
        );
    }

    #[test]
    fn reports_unknown_identifier() {
        let analysis = analyze_source("fn main() { return speed; }");
        assert!(
            analysis.diagnostics[0]
                .message
                .contains("unknown identifier")
        );
    }

    #[test]
    fn reports_duplicate_function() {
        let analysis = analyze_source("fn main() {} fn main() {}");
        assert!(analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("function `main` is already defined")
        }));
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
        assert!(
            analysis.diagnostics[0]
                .message
                .contains("unknown identifier")
        );
    }
}
