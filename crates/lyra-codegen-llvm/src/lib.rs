//! LLVM text-IR backend for Lyra.
//!
//! This first backend milestone deliberately emits a small, verifiable subset
//! of LLVM IR without binding the compiler to LLVM's C API. Native object
//! emission can be layered on after the Lyra IR/backend contract stabilizes.

use std::collections::HashMap;

use lyra_ir::{build_cfg, BinaryOperator, BlockId, Instruction, Module, Terminator, Type, UnaryOperator, Value};

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
            .join(", ");
        for parameter in &function.parameters {
            emitter
                .locals
                .insert(parameter.name.clone(), format!("%{}", parameter.name));
        }

        let cfg = build_cfg(&function.body);
        for block in &cfg.blocks {
            for successor in cfg.successors(block.id) {
                if cfg.block(successor).is_none() {
                    return Err(CodegenError::Unsupported("CFG successor references missing block"));
                }
            }

            if block.id != BlockId(0) {
                body.push_str(&format!("\nbb{}:\n", block.id.0));
            }
            emitter.emit_cfg_instructions(&block.instructions, &mut body)?;
            emitter.emit_cfg_terminator(&block.terminator, function.return_type, &mut body)?;
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
    next_block: usize,
    locals: HashMap<String, String>,
    mutable_locals: HashMap<String, String>,
    signatures: &'a HashMap<String, Type>,
}

impl<'a> FunctionEmitter<'a> {
    fn new(signatures: &'a HashMap<String, Type>) -> Self {
        Self {
            next_register: 0,
            next_block: 0,
            locals: HashMap::new(),
            mutable_locals: HashMap::new(),
            signatures,
        }
    }

    fn register(&mut self) -> String {
        self.next_register += 1;
        format!("%{}", self.next_register)
    }

    fn block_label(&mut self, prefix: &str) -> String {
        self.next_block += 1;
        format!("{prefix}.{}", self.next_block)
    }

    fn emit_cfg_instructions(
        &mut self,
        instructions: &[Instruction],
        body: &mut String,
    ) -> Result<(), CodegenError> {
        for instruction in instructions {
            match instruction {
                Instruction::Bind { name, value, .. } => {
                    let operand = self.emit_value(value, body)?;
                    self.locals.insert(name.clone(), operand);
                }
                Instruction::BindMutable { name, value, .. } => {
                    let operand = self.emit_value(value, body)?;
                    let slot = format!("%{name}.addr");
                    body.push_str(&format!("  {slot} = alloca i64\n"));
                    body.push_str(&format!("  store i64 {operand}, ptr {slot}\n"));
                    self.mutable_locals.insert(name.clone(), slot);
                }
                Instruction::Assign { name, value, .. } => {
                    let operand = self.emit_value(value, body)?;
                    let slot = self
                        .mutable_locals
                        .get(name)
                        .cloned()
                        .ok_or_else(|| CodegenError::UnknownLocal(name.clone()))?;
                    body.push_str(&format!("  store i64 {operand}, ptr {slot}\n"));
                }
                Instruction::Evaluate { value, .. } => {
                    let _ = self.emit_value(value, body)?;
                }
                Instruction::Return { .. } | Instruction::If { .. } | Instruction::While { .. } => {
                    return Err(CodegenError::Unsupported(
                        "structured control flow remained after CFG lowering",
                    ));
                }
            }
        }
        Ok(())
    }

    fn emit_cfg_terminator(
        &mut self,
        terminator: &Terminator,
        return_type: Type,
        body: &mut String,
    ) -> Result<(), CodegenError> {
        match terminator {
            Terminator::Return { value, .. } => self.emit_return(value.as_ref(), return_type, body),
            Terminator::Jump { target, .. } => {
                body.push_str(&format!("  br label %bb{}\n", target.0));
                Ok(())
            }
            Terminator::Branch {
                condition,
                then_target,
                else_target,
                ..
            } => {
                let condition = self.emit_value(condition, body)?;
                let condition_i1 = self.register();
                body.push_str(&format!("  {condition_i1} = icmp ne i64 {condition}, 0\n"));
                body.push_str(&format!(
                    "  br i1 {condition_i1}, label %bb{}, label %bb{}\n",
                    then_target.0, else_target.0
                ));
                Ok(())
            }
        }
    }

    fn emit_return(
        &mut self,
        value: Option<&Value>,
        return_type: Type,
        body: &mut String,
    ) -> Result<(), CodegenError> {
        let ty = llvm_type(return_type)?;
        if let Some(value) = value {
            let operand = self.emit_value(value, body)?;
            body.push_str(&format!("  ret {ty} {operand}\n"));
        } else {
            body.push_str(&format!("  ret {ty} {}\n", default_value(return_type)?));
        }
        Ok(())
    }

    fn emit_block(
        &mut self,
        block: &lyra_ir::Block,
        return_type: Type,
        body: &mut String,
    ) -> Result<bool, CodegenError> {
        for instruction in &block.instructions {
            match instruction {
                Instruction::Bind { name, value, .. } => {
                    let operand = self.emit_value(value, body)?;
                    self.locals.insert(name.clone(), operand);
                }
                Instruction::BindMutable { name, value, .. } => {
                    let operand = self.emit_value(value, body)?;
                    let slot = format!("%{name}.addr");
                    body.push_str(&format!("  {slot} = alloca i64\n"));
                    body.push_str(&format!("  store i64 {operand}, ptr {slot}\n"));
                    self.mutable_locals.insert(name.clone(), slot);
                }
                Instruction::Assign { name, value, .. } => {
                    let operand = self.emit_value(value, body)?;
                    let slot = self
                        .mutable_locals
                        .get(name)
                        .cloned()
                        .ok_or_else(|| CodegenError::UnknownLocal(name.clone()))?;
                    body.push_str(&format!("  store i64 {operand}, ptr {slot}\n"));
                }
                Instruction::Evaluate { value, .. } => {
                    let _ = self.emit_value(value, body)?;
                }
                Instruction::Return { value, .. } => {
                    self.emit_return(value.as_ref(), return_type, body)?;
                    return Ok(true);
                }
                Instruction::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.emit_if(
                        condition,
                        then_block,
                        else_block.as_ref(),
                        return_type,
                        body,
                    )? {
                        return Ok(true);
                    }
                }
                Instruction::While {
                    condition,
                    body: loop_body,
                    ..
                } => {
                    self.emit_while(condition, loop_body, return_type, body)?;
                }
            }
        }
        Ok(false)
    }

    fn emit_if(
        &mut self,
        condition: &Value,
        then_block: &lyra_ir::Block,
        else_block: Option<&lyra_ir::Block>,
        return_type: Type,
        body: &mut String,
    ) -> Result<bool, CodegenError> {
        let condition = self.emit_value(condition, body)?;
        let condition_i1 = self.register();
        body.push_str(&format!("  {condition_i1} = icmp ne i64 {condition}, 0\n"));

        let then_label = self.block_label("if.then");
        let else_label = self.block_label("if.else");
        let merge_label = self.block_label("if.end");
        body.push_str(&format!(
            "  br i1 {condition_i1}, label %{then_label}, label %{else_label}\n\n{then_label}:\n"
        ));

        let then_terminated = self.emit_block(then_block, return_type, body)?;
        if !then_terminated {
            body.push_str(&format!("  br label %{merge_label}\n"));
        }

        body.push_str(&format!("\n{else_label}:\n"));
        let else_terminated = if let Some(else_block) = else_block {
            self.emit_block(else_block, return_type, body)?
        } else {
            false
        };
        if !else_terminated {
            body.push_str(&format!("  br label %{merge_label}\n"));
        }

        if then_terminated && else_terminated {
            Ok(true)
        } else {
            body.push_str(&format!("\n{merge_label}:\n"));
            Ok(false)
        }
    }

    fn emit_while(
        &mut self,
        condition: &Value,
        loop_body: &lyra_ir::Block,
        return_type: Type,
        body: &mut String,
    ) -> Result<(), CodegenError> {
        let condition_label = self.block_label("while.cond");
        let body_label = self.block_label("while.body");
        let exit_label = self.block_label("while.end");

        body.push_str(&format!(
            "  br label %{condition_label}\n\n{condition_label}:\n"
        ));
        let condition = self.emit_value(condition, body)?;
        let condition_i1 = self.register();
        body.push_str(&format!("  {condition_i1} = icmp ne i64 {condition}, 0\n"));
        body.push_str(&format!(
            "  br i1 {condition_i1}, label %{body_label}, label %{exit_label}\n\n{body_label}:\n"
        ));

        let body_terminated = self.emit_block(loop_body, return_type, body)?;
        if !body_terminated {
            body.push_str(&format!("  br label %{condition_label}\n"));
        }

        body.push_str(&format!("\n{exit_label}:\n"));
        Ok(())
    }

    fn emit_value(&mut self, value: &Value, body: &mut String) -> Result<String, CodegenError> {
        match value {
            Value::Integer(value, _) => Ok(value.to_string()),
            Value::Boolean(value, _) => Ok(i64::from(*value).to_string()),
            Value::Local(name, _) => {
                if let Some(slot) = self.mutable_locals.get(name).cloned() {
                    let register = self.register();
                    body.push_str(&format!("  {register} = load i64, ptr {slot}\n"));
                    Ok(register)
                } else {
                    self.locals
                        .get(name)
                        .cloned()
                        .ok_or_else(|| CodegenError::UnknownLocal(name.clone()))
                }
            }
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
    fn emits_if_else_basic_blocks() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "main".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![Instruction::If {
                        condition: Value::Boolean(true, span),
                        then_block: Block {
                            instructions: vec![Instruction::Return {
                                value: Some(Value::Integer(42, span)),
                                span,
                            }],
                        },
                        else_block: Some(Block {
                            instructions: vec![Instruction::Return {
                                value: Some(Value::Integer(0, span)),
                                span,
                            }],
                        }),
                        span,
                    }],
                },
                span,
            }],
        };

        let llvm = emit_llvm_ir(&module).expect("conditional should lower");
        assert!(llvm.contains("br i1"));
        assert!(llvm.contains("if.then."));
        assert!(llvm.contains("if.else."));
        assert!(llvm.contains("ret i32 42"));
        assert!(llvm.contains("ret i32 0"));
    }

    #[test]
    fn emits_while_basic_blocks() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "main".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![
                        Instruction::While {
                            condition: Value::Boolean(false, span),
                            body: Block {
                                instructions: vec![Instruction::Evaluate {
                                    value: Value::Integer(1, span),
                                    span,
                                }],
                            },
                            span,
                        },
                        Instruction::Return {
                            value: Some(Value::Integer(42, span)),
                            span,
                        },
                    ],
                },
                span,
            }],
        };

        let llvm = emit_llvm_ir(&module).expect("while loop should lower");
        assert!(llvm.contains("while.cond."));
        assert!(llvm.contains("while.body."));
        assert!(llvm.contains("while.end."));
        assert!(llvm.contains("br i1"));
        assert!(llvm.contains("ret i32 42"));
    }

    #[test]
    fn emits_mutable_local_storage_and_updates() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "main".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![
                        Instruction::BindMutable {
                            name: "counter".into(),
                            value: Value::Integer(0, span),
                            span,
                        },
                        Instruction::Assign {
                            name: "counter".into(),
                            value: Value::Binary {
                                left: Box::new(Value::Local("counter".into(), span)),
                                operator: BinaryOperator::Add,
                                right: Box::new(Value::Integer(1, span)),
                                span,
                            },
                            span,
                        },
                        Instruction::Return {
                            value: Some(Value::Local("counter".into(), span)),
                            span,
                        },
                    ],
                },
                span,
            }],
        };

        let llvm = emit_llvm_ir(&module).expect("mutable local should lower");
        assert!(llvm.contains("%counter.addr = alloca i64"));
        assert!(llvm.contains("store i64 0, ptr %counter.addr"));
        assert!(llvm.contains("load i64, ptr %counter.addr"));
        assert!(llvm.contains("store i64 %"));
    }

    #[test]
    fn validates_cfg_before_llvm_emission() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "main".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![
                        Instruction::If {
                            condition: Value::Boolean(true, span),
                            then_block: Block {
                                instructions: vec![Instruction::Return {
                                    value: Some(Value::Integer(42, span)),
                                    span,
                                }],
                            },
                            else_block: None,
                            span,
                        },
                        Instruction::Return {
                            value: Some(Value::Integer(0, span)),
                            span,
                        },
                    ],
                },
                span,
            }],
        };

        let cfg = build_cfg(&module.functions[0].body);
        assert!(cfg.blocks.iter().any(|block| matches!(
            block.terminator,
            Terminator::Branch {
                then_target: BlockId(_),
                else_target: BlockId(_),
                ..
            }
        )));

        let llvm = emit_llvm_ir(&module).expect("valid CFG should permit LLVM emission");
        assert!(llvm.contains("define i32 @main()"));
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
