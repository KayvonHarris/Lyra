//! Compiler-owned intermediate representation for Lyra.
//!
//! Lyra IR sits between the validated AST and backend-specific IRs such as
//! LLVM IR. It preserves Lyra semantics without coupling the language to a
//! particular code-generation framework.

use lyra_span::Span;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Integer,
    Float,
    String,
    Boolean,
    Unit,
    Unknown,
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
    If {
        condition: Value,
        then_block: Block,
        else_block: Option<Block>,
        span: Span,
    },
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

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64, Span),
    Float(f64, Span),
    String(String, Span),
    Boolean(bool, Span),
    Local(String, Span),
    Call {
        callee: String,
        arguments: Vec<Value>,
        span: Span,
    },
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
            | Self::Call { span, .. }
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
        parameters: function
            .parameters
            .iter()
            .map(|parameter| Parameter {
                name: parameter.name.clone(),
                ty: parameter
                    .type_name
                    .as_ref()
                    .map_or(Type::Integer, lower_type_name),
                span: parameter.span,
            })
            .collect(),
        return_type: function
            .return_type
            .as_ref()
            .map_or(Type::Integer, lower_type_name),
        body: lower_block(&function.body),
        span: function.span,
    }
}

fn lower_block(block: &lyra_ast::Block) -> Block {
    Block {
        instructions: block.statements.iter().map(lower_statement).collect(),
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
        lyra_ast::Statement::If {
            condition,
            then_block,
            else_block,
            span,
        } => Instruction::If {
            condition: lower_expression(condition),
            then_block: lower_block(then_block),
            else_block: else_block.as_ref().map(lower_block),
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
        lyra_ast::Expression::Call {
            callee,
            arguments,
            span,
        } => Value::Call {
            callee: callee.clone(),
            arguments: arguments.iter().map(lower_expression).collect(),
            span: *span,
        },
        lyra_ast::Expression::Unary {
            operator,
            operand,
            span,
        } => Value::Unary {
            operator: lower_unary_operator(*operator),
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
            operator: lower_binary_operator(*operator),
            right: Box::new(lower_expression(right)),
            span: *span,
        },
    }
}

fn lower_type_name(type_name: &lyra_ast::TypeName) -> Type {
    match type_name.name.as_str() {
        "Int" => Type::Integer,
        "Float" => Type::Float,
        "String" => Type::String,
        "Bool" => Type::Boolean,
        "Unit" => Type::Unit,
        _ => Type::Unknown,
    }
}

fn lower_unary_operator(operator: lyra_ast::UnaryOperator) -> UnaryOperator {
    match operator {
        lyra_ast::UnaryOperator::Negate => UnaryOperator::Negate,
        lyra_ast::UnaryOperator::Not => UnaryOperator::Not,
    }
}

fn lower_binary_operator(operator: lyra_ast::BinaryOperator) -> BinaryOperator {
    match operator {
        lyra_ast::BinaryOperator::Add => BinaryOperator::Add,
        lyra_ast::BinaryOperator::Subtract => BinaryOperator::Subtract,
        lyra_ast::BinaryOperator::Multiply => BinaryOperator::Multiply,
        lyra_ast::BinaryOperator::Divide => BinaryOperator::Divide,
        lyra_ast::BinaryOperator::Remainder => BinaryOperator::Remainder,
        lyra_ast::BinaryOperator::Equal => BinaryOperator::Equal,
        lyra_ast::BinaryOperator::NotEqual => BinaryOperator::NotEqual,
        lyra_ast::BinaryOperator::Less => BinaryOperator::Less,
        lyra_ast::BinaryOperator::LessEqual => BinaryOperator::LessEqual,
        lyra_ast::BinaryOperator::Greater => BinaryOperator::Greater,
        lyra_ast::BinaryOperator::GreaterEqual => BinaryOperator::GreaterEqual,
        lyra_ast::BinaryOperator::And => BinaryOperator::And,
        lyra_ast::BinaryOperator::Or => BinaryOperator::Or,
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
    fn lowers_function_parameters_and_calls() {
        let module =
            lower_source("fn add(a, b) { return a + b; } fn main() { return add(20, 22); }");
        assert_eq!(module.functions[0].parameters.len(), 2);
        assert_eq!(module.functions[0].parameters[0].name, "a");
        assert_eq!(module.functions[0].parameters[1].name, "b");
        assert!(matches!(
            &module.functions[1].body.instructions[0],
            Instruction::Return {
                value: Some(Value::Call { callee, arguments, .. }),
                ..
            } if callee == "add" && arguments.len() == 2
        ));
    }

    #[test]
    fn lowers_typed_function_signature() {
        let module = lower_source(
            "fn add(a: Int, b: Int) -> Int { return a + b; } fn main() -> Int { return add(20, 22); }",
        );
        assert_eq!(module.functions[0].parameters[0].ty, Type::Integer);
        assert_eq!(module.functions[0].parameters[1].ty, Type::Integer);
        assert_eq!(module.functions[0].return_type, Type::Integer);
        assert_eq!(module.functions[1].return_type, Type::Integer);
    }

    #[test]
    fn lowers_if_else_control_flow() {
        let module = lower_source(
            "fn main() -> Int { let speed = 65; if speed >= 60 { return 42; } else { return 0; } }",
        );
        assert!(matches!(
            &module.functions[0].body.instructions[1],
            Instruction::If {
                condition: Value::Binary {
                    operator: BinaryOperator::GreaterEqual,
                    ..
                },
                then_block,
                else_block: Some(else_block),
                ..
            } if matches!(then_block.instructions[0], Instruction::Return { .. })
                && matches!(else_block.instructions[0], Instruction::Return { .. })
        ));
    }

    #[test]
    fn lowers_binary_expression_tree() {
        let module = lower_source("fn main() { return 60 + 5 * 2; }");
        let Instruction::Return {
            value: Some(value), ..
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
