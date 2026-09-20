//! LLVM text-IR backend for Lyra.
//!
//! This first backend milestone deliberately emits a small, verifiable subset
//! of LLVM IR without binding the compiler to LLVM's C API. Native object
//! emission can be layered on after the Lyra IR/backend contract stabilizes.

use std::collections::HashMap;

use lyra_ir::{BinaryOperator, Instruction, Module, Type, UnaryOperator, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    Unsupported(&'static str),
    UnknownLocal(String),
    UnknownFunction(String),
}

pub fn emit_llvm_ir(module: &Module) -> Result<String, CodegenError> {
    let signatures = module
        .functions
        .iter()
        .map(|function| (function.name.clone(), function.return_type))
        .collect::<HashMap<_, _>>();
    let mut output = String::from("; ModuleID = 'lyra'\nsource_filename = \"lyra\"\n\n");

    for function in &module.functions {
        let mut emitter = FunctionEmitter::new(&signatures);
        let mut body = String::new();
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| Ok(format!("{} %{}", llvm_type(parameter.ty)?, parameter.name)))
            .collect::<Result<Vec<_>, CodegenError>>()?
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        for parameter in &function.parameters {
            emitter
                .locals
                .insert(parameter.name.clone(), format!("%{}", parameter.name));
        }

        let mut terminated = false;
        for instruction in &function.body.instructions {
            if terminated {
                break;
            }

            match instruction {
                Instruction::Bind { name, value, .. } => {
                    let operand = emitter.emit_value(value, &mut body)?;
                    emitter.locals.insert(name.clone(), operand);
                }
                Instruction::Evaluate { value, .. } => {
                    let _ = emitter.emit_value(value, &mut body)?;
                }
                Instruction::Return { value, .. } => {
                    if let Some(value) = value {
                        let operand = emitter.emit_value(value, &mut body)?;
                        let ty = llvm_type(function.return_type)?;
                        body.push_str(&format!("  ret {ty} {operand}\n"));
                    } else {
                        let ty = llvm_type(function.return_type)?;
                        body.push_str(&format!(
                            "  ret {ty} {}\n",
                            default_value(function.return_type)?
                        ));
                    }
                    terminated = true;
                }
            }
        }

        if !terminated
            && !body
                .lines()
                .any(|line| line.trim_start().starts_with("ret "))
        {
            let ty = llvm_type(function.return_type)?;
            body.push_str(&format!(
                "  ret {ty} {}\n",
                default_value(function.return_type)?
            ));
        }

        let return_type = if function.name == "main" {
            "i32"
        } else {
            llvm_type(function.return_type)?
        };
        let body = if function.name == "main" {
            normalize_main_returns(&body)
        } else {
            body
        };

        output.push_str(&format!(
            "define {return_type} @{}({parameters}) {{\nentry:\n{body}}}\n\n",
            function.name
        ));
    }

    Ok(output)
}

fn normalize_main_returns(body: &str) -> String {
    let mut output = String::new();
    for line in body.lines() {
        if let Some(value) = line.trim().strip_prefix("ret i64 ") {
            if let Ok(value) = value.parse::<i64>() {
                output.push_str(&format!("  ret i32 {}\n", value as i32));
            } else {
                output.push_str(&format!("  %lyra.main.exit = trunc i64 {value} to i32\n"));
                output.push_str("  ret i32 %lyra.main.exit\n");
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

fn llvm_type(ty: Type) -> Result<&'static str, CodegenError> {
    match ty {
        Type::Integer | Type::Boolean => Ok("i64"),
        Type::Float => Ok("double"),
        Type::Unit => Ok("i64"),
        Type::String => Err(CodegenError::Unsupported("string function types")),
        Type::Unknown => Err(CodegenError::Unsupported("unknown function types")),
    }
}

fn default_value(ty: Type) -> Result<&'static str, CodegenError> {
    match ty {
        Type::Integer | Type::Boolean | Type::Unit => Ok("0"),
        Type::Float => Ok("0.0"),
        Type::String => Err(CodegenError::Unsupported("string function types")),
        Type::Unknown => Err(CodegenError::Unsupported("unknown function types")),
    }
}

struct FunctionEmitter<'a> {
    next_register: usize,
    locals: HashMap<String, String>,
    signatures: &'a HashMap<String, Type>,
}

impl<'a> FunctionEmitter<'a> {
    fn new(signatures: &'a HashMap<String, Type>) -> Self {
        Self {
            next_register: 0,
            locals: HashMap::new(),
            signatures,
        }
    }

    fn register(&mut self) -> String {
        self.next_register += 1;
        format!("%{}", self.next_register)
    }

    fn emit_value(&mut self, value: &Value, body: &mut String) -> Result<String, CodegenError> {
        match value {
            Value::Integer(value, _) => Ok(value.to_string()),
            Value::Boolean(value, _) => Ok(i64::from(*value).to_string()),
            Value::Local(name, _) => self
                .locals
                .get(name)
                .cloned()
                .ok_or_else(|| CodegenError::UnknownLocal(name.clone())),
            Value::Call {
                callee, arguments, ..
            } => {
                let return_type = self
                    .signatures
                    .get(callee)
                    .copied()
                    .ok_or_else(|| CodegenError::UnknownFunction(callee.clone()))?;
                let mut operands = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    let operand = self.emit_value(argument, body)?;
                    operands.push(format!("i64 {operand}"));
                }
                let register = self.register();
                body.push_str(&format!(
                    "  {register} = call {} @{callee}({})\n",
                    llvm_type(return_type)?,
                    operands.join(", ")
                ));
                Ok(register)
            }
            Value::Unary {
                operator, operand, ..
            } => {
                let operand = self.emit_value(operand, body)?;
                let register = self.register();
                let expression = match operator {
                    UnaryOperator::Negate => format!("sub i64 0, {operand}"),
                    UnaryOperator::Not => format!("xor i64 {operand}, 1"),
                };
                body.push_str(&format!("  {register} = {expression}\n"));
                Ok(register)
            }
            Value::Binary {
                left,
                operator,
                right,
                ..
            } => {
                let left = self.emit_value(left, body)?;
                let right = self.emit_value(right, body)?;
                let arithmetic = match operator {
                    BinaryOperator::Add => Some("add"),
                    BinaryOperator::Subtract => Some("sub"),
                    BinaryOperator::Multiply => Some("mul"),
                    BinaryOperator::Divide => Some("sdiv"),
                    BinaryOperator::Remainder => Some("srem"),
                    _ => None,
                };

                if let Some(opcode) = arithmetic {
                    let register = self.register();
                    body.push_str(&format!("  {register} = {opcode} i64 {left}, {right}\n"));
                    return Ok(register);
                }

                let predicate = match operator {
                    BinaryOperator::Equal => "eq",
                    BinaryOperator::NotEqual => "ne",
                    BinaryOperator::Less => "slt",
                    BinaryOperator::LessEqual => "sle",
                    BinaryOperator::Greater => "sgt",
                    BinaryOperator::GreaterEqual => "sge",
                    BinaryOperator::And | BinaryOperator::Or => {
                        return Err(CodegenError::Unsupported("logical binary operator"));
                    }
                    _ => unreachable!("arithmetic operators returned above"),
                };

                let comparison = self.register();
                body.push_str(&format!(
                    "  {comparison} = icmp {predicate} i64 {left}, {right}\n"
                ));
                let result = self.register();
                body.push_str(&format!("  {result} = zext i1 {comparison} to i64\n"));
                Ok(result)
            }
            Value::Float(_, _) => Err(CodegenError::Unsupported("float values")),
            Value::String(_, _) => Err(CodegenError::Unsupported("string values")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lyra_ir::{Block, Function, Type};
    use lyra_span::Span;

    #[test]
    fn stops_emitting_after_return() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "main".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![
                        Instruction::Return {
                            value: Some(Value::Integer(7, span)),
                            span,
                        },
                        Instruction::Evaluate {
                            value: Value::Integer(99, span),
                            span,
                        },
                    ],
                },
                span,
            }],
        };

        let llvm = emit_llvm_ir(&module).expect("codegen should succeed");
        assert!(llvm.contains("ret i32 7"));
        assert!(!llvm.contains("99"));
    }

    #[test]
    fn emits_integer_comparison() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "main".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![Instruction::Return {
                        value: Some(Value::Binary {
                            left: Box::new(Value::Integer(42, span)),
                            operator: BinaryOperator::Greater,
                            right: Box::new(Value::Integer(7, span)),
                            span,
                        }),
                        span,
                    }],
                },
                span,
            }],
        };

        let llvm = emit_llvm_ir(&module).expect("comparison should lower");
        assert!(llvm.contains("icmp sgt i64 42, 7"));
        assert!(llvm.contains("zext i1 %1 to i64"));
        assert!(llvm.contains("trunc i64 %2 to i32"));
        assert!(llvm.contains("ret i32 %lyra.main.exit"));
    }

    #[test]
    fn emits_function_parameters_and_call() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![
                Function {
                    name: "add".into(),
                    parameters: vec![
                        lyra_ir::Parameter {
                            name: "a".into(),
                            ty: Type::Integer,
                            span,
                        },
                        lyra_ir::Parameter {
                            name: "b".into(),
                            ty: Type::Integer,
                            span,
                        },
                    ],
                    return_type: Type::Integer,
                    body: Block {
                        instructions: vec![Instruction::Return {
                            value: Some(Value::Binary {
                                left: Box::new(Value::Local("a".into(), span)),
                                operator: BinaryOperator::Add,
                                right: Box::new(Value::Local("b".into(), span)),
                                span,
                            }),
                            span,
                        }],
                    },
                    span,
                },
                Function {
                    name: "main".into(),
                    parameters: vec![],
                return_type: Type::Integer,
                    body: Block {
                        instructions: vec![Instruction::Return {
                            value: Some(Value::Call {
                                callee: "add".into(),
                                arguments: vec![Value::Integer(20, span), Value::Integer(22, span)],
                                span,
                            }),
                            span,
                        }],
                    },
                    span,
                },
            ],
        };

        let llvm = emit_llvm_ir(&module).expect("function call should lower");
        assert!(llvm.contains("define i64 @add(i64 %a, i64 %b)"));
        assert!(llvm.contains("add i64 %a, %b"));
        assert!(llvm.contains("call i64 @add(i64 20, i64 22)"));
        assert!(llvm.contains("ret i32 %lyra.main.exit"));
    }

    #[test]
    fn emits_integer_main_function() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "main".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![Instruction::Return {
                        value: Some(Value::Binary {
                            left: Box::new(Value::Integer(40, span)),
                            operator: BinaryOperator::Add,
                            right: Box::new(Value::Integer(2, span)),
                            span,
                        }),
                        span,
                    }],
                },
                span,
            }],
        };

        let llvm = emit_llvm_ir(&module).expect("LLVM IR emission should succeed");
        assert!(llvm.contains("define i32 @main()"));
        assert!(llvm.contains("add i64 40, 2"));
        assert!(llvm.contains("trunc i64 %1 to i32"));
        assert!(llvm.contains("ret i32 %lyra.main.exit"));
    }
}
