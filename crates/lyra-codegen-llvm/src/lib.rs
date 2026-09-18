//! LLVM text-IR backend for Lyra.
//!
//! This first backend milestone deliberately emits a small, verifiable subset
//! of LLVM IR without binding the compiler to LLVM's C API. Native object
//! emission can be layered on after the Lyra IR/backend contract stabilizes.

use std::collections::HashMap;

use lyra_ir::{BinaryOperator, Instruction, Module, UnaryOperator, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    Unsupported(&'static str),
    UnknownLocal(String),
}

pub fn emit_llvm_ir(module: &Module) -> Result<String, CodegenError> {
    let mut output = String::from("; ModuleID = 'lyra'\nsource_filename = \"lyra\"\n\n");

    for function in &module.functions {
        let mut emitter = FunctionEmitter::default();
        let mut body = String::new();

        for instruction in &function.body.instructions {
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
                        body.push_str(&format!("  ret i64 {operand}\n"));
                    } else {
                        body.push_str("  ret i64 0\n");
                    }
                }
            }
        }

        if !body.lines().any(|line| line.trim_start().starts_with("ret ")) {
            body.push_str("  ret i64 0\n");
        }

        output.push_str(&format!("define i64 @{}() {{\nentry:\n{body}}}\n\n", function.name));
    }

    Ok(output)
}

#[derive(Default)]
struct FunctionEmitter {
    next_register: usize,
    locals: HashMap<String, String>,
}

impl FunctionEmitter {
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
            Value::Unary { operator, operand, .. } => {
                let operand = self.emit_value(operand, body)?;
                let register = self.register();
                let expression = match operator {
                    UnaryOperator::Negate => format!("sub i64 0, {operand}"),
                    UnaryOperator::Not => format!("xor i64 {operand}, 1"),
                };
                body.push_str(&format!("  {register} = {expression}\n"));
                Ok(register)
            }
            Value::Binary { left, operator, right, .. } => {
                let left = self.emit_value(left, body)?;
                let right = self.emit_value(right, body)?;
                let register = self.register();
                let expression = match operator {
                    BinaryOperator::Add => format!("add i64 {left}, {right}"),
                    BinaryOperator::Subtract => format!("sub i64 {left}, {right}"),
                    BinaryOperator::Multiply => format!("mul i64 {left}, {right}"),
                    BinaryOperator::Divide => format!("sdiv i64 {left}, {right}"),
                    BinaryOperator::Remainder => format!("srem i64 {left}, {right}"),
                    BinaryOperator::Equal => format!("icmp eq i64 {left}, {right}"),
                    BinaryOperator::NotEqual => format!("icmp ne i64 {left}, {right}"),
                    BinaryOperator::Less => format!("icmp slt i64 {left}, {right}"),
                    BinaryOperator::LessEqual => format!("icmp sle i64 {left}, {right}"),
                    BinaryOperator::Greater => format!("icmp sgt i64 {left}, {right}"),
                    BinaryOperator::GreaterEqual => format!("icmp sge i64 {left}, {right}"),
                    BinaryOperator::And | BinaryOperator::Or => {
                        return Err(CodegenError::Unsupported("logical binary operator"));
                    }
                };
                body.push_str(&format!("  {register} = {expression}\n"));
                Ok(register)
            }
            Value::Float(_, _) => Err(CodegenError::Unsupported("float values")),
            Value::String(_, _) => Err(CodegenError::Unsupported("string values")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lyra_ir::{Block, Function};
    use lyra_span::Span;

    #[test]
    fn emits_integer_comparison() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "main".into(),
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
    }

    #[test]
    fn emits_integer_main_function() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "main".into(),
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
        assert!(llvm.contains("define i64 @main()"));
        assert!(llvm.contains("add i64 40, 2"));
        assert!(llvm.contains("ret i64 %1"));
    }
}
