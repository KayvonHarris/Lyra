//! LLVM text-IR backend for Lyra.
//!
//! This first backend milestone deliberately emits a small, verifiable subset
//! of LLVM IR without binding the compiler to LLVM's C API. Native object
//! emission can be layered on after the Lyra IR/backend contract stabilizes.

use std::collections::HashMap;

use lyra_ir::{
    BasicBlock, BinaryOperator, BlockId, Instruction, Module, Terminator, Type, UnaryOperator,
    Value, ValueId, build_cfg,
};

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
                    return Err(CodegenError::Unsupported(
                        "CFG successor references missing block",
                    ));
                }
            }

            if block.id != BlockId(0) {
                body.push_str(&format!("\nbb{}:\n", block.id.0));
            }
            emitter.enter_cfg_block(&cfg, block);
            emitter.emit_phi_nodes(block, &mut body)?;
            emitter.emit_cfg_instructions(block, &mut body)?;
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
    locals: HashMap<String, String>,
    mutable_locals: HashMap<String, String>,
    signatures: &'a HashMap<String, Type>,
    ssa_locals: HashMap<String, String>,
}

impl<'a> FunctionEmitter<'a> {
    fn new(signatures: &'a HashMap<String, Type>) -> Self {
        Self {
            next_register: 0,
            locals: HashMap::new(),
            mutable_locals: HashMap::new(),
            signatures,
            ssa_locals: HashMap::new(),
        }
    }

    fn register(&mut self) -> String {
        self.next_register += 1;
        format!("%{}", self.next_register)
    }

    fn enter_cfg_block(&mut self, cfg: &lyra_ir::ControlFlowGraph, block: &BasicBlock) {
        self.ssa_locals.clear();
        if let Some(definitions) = cfg.entry_definitions.get(&block.id) {
            for (name, id) in definitions {
                self.ssa_locals
                    .insert(name.clone(), Self::ssa_register(*id));
            }
        }
    }

    fn emit_phi_nodes(
        &mut self,
        block: &BasicBlock,
        body: &mut String,
    ) -> Result<(), CodegenError> {
        for phi in &block.phi_nodes {
            let incoming = phi
                .incoming
                .iter()
                .map(|(predecessor, value)| {
                    format!(
                        "[ {}, %{} ]",
                        Self::ssa_register(*value),
                        Self::block_label(*predecessor)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            body.push_str(&format!(
                "  {} = phi i64 {incoming}\n",
                Self::ssa_register(phi.id)
            ));
        }
        Ok(())
    }

    fn ssa_register(id: ValueId) -> String {
        format!("%ssa{}", id.0)
    }

    fn block_label(id: BlockId) -> String {
        if id == BlockId(0) {
            "entry".to_owned()
        } else {
            format!("bb{}", id.0)
        }
    }

    fn emit_cfg_instructions(
        &mut self,
        block: &BasicBlock,
        body: &mut String,
    ) -> Result<(), CodegenError> {
        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            let definition = block
                .definitions
                .iter()
                .find(|definition| definition.instruction_index == instruction_index);
            match instruction {
                Instruction::Bind { name, value, .. } => {
                    let operand = self.emit_value(value, body)?;
                    let operand = self.materialize_ssa_definition(definition, &operand, body);
                    self.ssa_locals.insert(name.clone(), operand.clone());
                    self.locals.insert(name.clone(), operand);
                }
                Instruction::BindMutable { name, value, .. } => {
                    let operand = self.emit_value(value, body)?;
                    let operand = self.materialize_ssa_definition(definition, &operand, body);
                    let slot = format!("%{name}.addr");
                    body.push_str(&format!("  {slot} = alloca i64\n"));
                    body.push_str(&format!("  store i64 {operand}, ptr {slot}\n"));
                    self.ssa_locals.insert(name.clone(), operand);
                    self.mutable_locals.insert(name.clone(), slot);
                }
                Instruction::Assign { name, value, .. } => {
                    let operand = self.emit_value(value, body)?;
                    let operand = self.materialize_ssa_definition(definition, &operand, body);
                    let slot = self
                        .mutable_locals
                        .get(name)
                        .cloned()
                        .ok_or_else(|| CodegenError::UnknownLocal(name.clone()))?;
                    body.push_str(&format!("  store i64 {operand}, ptr {slot}\n"));
                    self.ssa_locals.insert(name.clone(), operand);
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

    fn materialize_ssa_definition(
        &self,
        definition: Option<&lyra_ir::ValueDefinition>,
        operand: &str,
        body: &mut String,
    ) -> String {
        let Some(definition) = definition else {
            return operand.to_owned();
        };
        let register = Self::ssa_register(definition.id);
        body.push_str(&format!("  {register} = add i64 {operand}, 0\n"));
        register
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

    fn emit_value(&mut self, value: &Value, body: &mut String) -> Result<String, CodegenError> {
        match value {
            Value::Integer(value, _) => Ok(value.to_string()),
            Value::Boolean(value, _) => Ok(i64::from(*value).to_string()),
            Value::Local(name, _) => {
                if let Some(value) = self.ssa_locals.get(name).cloned() {
                    Ok(value)
                } else if let Some(slot) = self.mutable_locals.get(name).cloned() {
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
        assert!(llvm.contains("bb1:"));
        assert!(llvm.contains("bb2:"));
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
        assert!(llvm.contains("bb1:"));
        assert!(llvm.contains("bb2:"));
        assert!(llvm.contains("bb3:"));
        assert!(llvm.contains("br i1"));
        assert!(llvm.contains("ret i32 42"));
    }

    #[test]
    fn loop_condition_reads_header_phi_value() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "count".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![
                        Instruction::BindMutable {
                            name: "counter".into(),
                            value: Value::Integer(0, span),
                            span,
                        },
                        Instruction::While {
                            condition: Value::Binary {
                                left: Box::new(Value::Local("counter".into(), span)),
                                operator: BinaryOperator::Less,
                                right: Box::new(Value::Integer(3, span)),
                                span,
                            },
                            body: Block {
                                instructions: vec![Instruction::Assign {
                                    name: "counter".into(),
                                    value: Value::Binary {
                                        left: Box::new(Value::Local("counter".into(), span)),
                                        operator: BinaryOperator::Add,
                                        right: Box::new(Value::Integer(1, span)),
                                        span,
                                    },
                                    span,
                                }],
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

        let llvm = emit_llvm_ir(&module).expect("loop-carried SSA should lower");
        let phi_line = llvm
            .lines()
            .find(|line| line.contains(" = phi i64 "))
            .expect("loop header phi");
        let phi_register = phi_line.trim().split(" =").next().expect("phi register");
        assert!(phi_line.contains("[ %ssa0, %entry ]"));
        assert!(phi_line.contains("[ %ssa"));
        assert!(llvm.contains(&format!("icmp slt i64 {phi_register}, 3")));
        assert!(llvm.contains(&format!("add i64 {phi_register}, 1")));
    }

    #[test]
    fn multiple_loop_carried_locals_use_distinct_header_phis() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "accumulate".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![
                        Instruction::BindMutable {
                            name: "counter".into(),
                            value: Value::Integer(0, span),
                            span,
                        },
                        Instruction::BindMutable {
                            name: "total".into(),
                            value: Value::Integer(10, span),
                            span,
                        },
                        Instruction::While {
                            condition: Value::Binary {
                                left: Box::new(Value::Local("counter".into(), span)),
                                operator: BinaryOperator::Less,
                                right: Box::new(Value::Integer(3, span)),
                                span,
                            },
                            body: Block {
                                instructions: vec![
                                    Instruction::Assign {
                                        name: "total".into(),
                                        value: Value::Binary {
                                            left: Box::new(Value::Local("total".into(), span)),
                                            operator: BinaryOperator::Add,
                                            right: Box::new(Value::Local("counter".into(), span)),
                                            span,
                                        },
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
                                ],
                            },
                            span,
                        },
                        Instruction::Return {
                            value: Some(Value::Local("total".into(), span)),
                            span,
                        },
                    ],
                },
                span,
            }],
        };

        let llvm = emit_llvm_ir(&module).expect("multiple loop-carried values should lower");
        let phi_lines = llvm
            .lines()
            .filter(|line| line.contains(" = phi i64 "))
            .collect::<Vec<_>>();
        assert_eq!(
            phi_lines.len(),
            2,
            "expected one loop-header phi per carried local"
        );
        assert!(phi_lines.iter().all(|line| line.contains("%entry")));
        assert!(
            phi_lines
                .iter()
                .all(|line| line.matches("[ %ssa").count() == 2)
        );

        let phi_registers = phi_lines
            .iter()
            .map(|line| line.trim().split(" =").next().expect("phi register"))
            .collect::<Vec<_>>();
        assert_ne!(phi_registers[0], phi_registers[1]);
        assert!(
            llvm.contains(&format!("icmp slt i64 {}", phi_registers[0]))
                || llvm.contains(&format!("icmp slt i64 {}", phi_registers[1]))
        );
        assert!(
            llvm.contains(&format!("ret i64 {}", phi_registers[0]))
                || llvm.contains(&format!("ret i64 {}", phi_registers[1]))
        );
    }

    #[test]
    fn loop_with_nested_branch_preserves_carried_ssa_value() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "branching_loop".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![
                        Instruction::BindMutable {
                            name: "counter".into(),
                            value: Value::Integer(0, span),
                            span,
                        },
                        Instruction::BindMutable {
                            name: "total".into(),
                            value: Value::Integer(0, span),
                            span,
                        },
                        Instruction::While {
                            condition: Value::Binary {
                                left: Box::new(Value::Local("counter".into(), span)),
                                operator: BinaryOperator::Less,
                                right: Box::new(Value::Integer(3, span)),
                                span,
                            },
                            body: Block {
                                instructions: vec![
                                    Instruction::If {
                                        condition: Value::Binary {
                                            left: Box::new(Value::Local("counter".into(), span)),
                                            operator: BinaryOperator::Equal,
                                            right: Box::new(Value::Integer(1, span)),
                                            span,
                                        },
                                        then_block: Block {
                                            instructions: vec![Instruction::Assign {
                                                name: "total".into(),
                                                value: Value::Binary {
                                                    left: Box::new(Value::Local(
                                                        "total".into(),
                                                        span,
                                                    )),
                                                    operator: BinaryOperator::Add,
                                                    right: Box::new(Value::Integer(10, span)),
                                                    span,
                                                },
                                                span,
                                            }],
                                        },
                                        else_block: Some(Block {
                                            instructions: vec![Instruction::Assign {
                                                name: "total".into(),
                                                value: Value::Binary {
                                                    left: Box::new(Value::Local(
                                                        "total".into(),
                                                        span,
                                                    )),
                                                    operator: BinaryOperator::Add,
                                                    right: Box::new(Value::Integer(1, span)),
                                                    span,
                                                },
                                                span,
                                            }],
                                        }),
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
                                ],
                            },
                            span,
                        },
                        Instruction::Return {
                            value: Some(Value::Local("total".into(), span)),
                            span,
                        },
                    ],
                },
                span,
            }],
        };

        let llvm = emit_llvm_ir(&module).expect("nested branch loop should lower");
        let phi_lines = llvm
            .lines()
            .filter(|line| line.contains(" = phi i64 "))
            .collect::<Vec<_>>();
        assert!(
            phi_lines.len() >= 3,
            "expected loop-carried phis plus the nested branch merge"
        );
        assert!(llvm.contains("icmp eq i64"));
        assert!(llvm.contains("add i64"));
        assert!(
            llvm.lines()
                .any(|line| line.trim_start().starts_with("ret i64 %ssa"))
        );
    }

    #[test]
    fn emits_cfg_phi_nodes() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "main".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![
                        Instruction::BindMutable {
                            name: "value".into(),
                            value: Value::Integer(0, span),
                            span,
                        },
                        Instruction::If {
                            condition: Value::Boolean(true, span),
                            then_block: Block {
                                instructions: vec![Instruction::Assign {
                                    name: "value".into(),
                                    value: Value::Integer(20, span),
                                    span,
                                }],
                            },
                            else_block: Some(Block {
                                instructions: vec![Instruction::Assign {
                                    name: "value".into(),
                                    value: Value::Integer(22, span),
                                    span,
                                }],
                            }),
                            span,
                        },
                        Instruction::Return {
                            value: Some(Value::Local("value".into(), span)),
                            span,
                        },
                    ],
                },
                span,
            }],
        };

        let llvm = emit_llvm_ir(&module).expect("phi nodes should lower");
        assert!(llvm.contains("%ssa0 = add i64 0, 0"));
        assert!(llvm.contains("%ssa1 = add i64 20, 0"));
        assert!(llvm.contains("%ssa2 = add i64 22, 0"));
        assert!(llvm.contains(" = phi i64 "));
        assert!(llvm.contains("[ %ssa1"));
        assert!(llvm.contains("[ %ssa2"));
    }

    #[test]
    fn merged_local_reads_use_phi_value() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "choose".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![
                        Instruction::BindMutable {
                            name: "value".into(),
                            value: Value::Integer(0, span),
                            span,
                        },
                        Instruction::If {
                            condition: Value::Boolean(true, span),
                            then_block: Block {
                                instructions: vec![Instruction::Assign {
                                    name: "value".into(),
                                    value: Value::Integer(20, span),
                                    span,
                                }],
                            },
                            else_block: Some(Block {
                                instructions: vec![Instruction::Assign {
                                    name: "value".into(),
                                    value: Value::Integer(22, span),
                                    span,
                                }],
                            }),
                            span,
                        },
                        Instruction::Return {
                            value: Some(Value::Local("value".into(), span)),
                            span,
                        },
                    ],
                },
                span,
            }],
        };

        let llvm = emit_llvm_ir(&module).expect("merged local should use phi value");
        let phi_line = llvm
            .lines()
            .find(|line| line.contains(" = phi i64 "))
            .expect("phi");
        let phi_register = phi_line.trim().split(" =").next().expect("phi register");
        assert!(llvm.contains(&format!("ret i64 {phi_register}")));
    }

    #[test]
    fn loop_condition_reads_loop_carried_phi_value() {
        let span = Span { start: 0, end: 0 };
        let module = Module {
            functions: vec![Function {
                name: "count".into(),
                parameters: vec![],
                return_type: Type::Integer,
                body: Block {
                    instructions: vec![
                        Instruction::BindMutable {
                            name: "counter".into(),
                            value: Value::Integer(0, span),
                            span,
                        },
                        Instruction::While {
                            condition: Value::Binary {
                                left: Box::new(Value::Local("counter".into(), span)),
                                operator: BinaryOperator::Less,
                                right: Box::new(Value::Integer(3, span)),
                                span,
                            },
                            body: Block {
                                instructions: vec![Instruction::Assign {
                                    name: "counter".into(),
                                    value: Value::Binary {
                                        left: Box::new(Value::Local("counter".into(), span)),
                                        operator: BinaryOperator::Add,
                                        right: Box::new(Value::Integer(1, span)),
                                        span,
                                    },
                                    span,
                                }],
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

        let llvm = emit_llvm_ir(&module).expect("loop-carried SSA should lower");
        let phi_line = llvm
            .lines()
            .find(|line| line.contains(" = phi i64 "))
            .expect("loop header phi");
        let phi_register = phi_line.trim().split(" =").next().expect("phi register");
        assert!(
            llvm.lines()
                .any(|line| line.contains("icmp slt i64") && line.contains(phi_register)),
            "loop condition should read the loop-carried phi value: {llvm}"
        );
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
        assert!(llvm.contains("%ssa0 = add i64 0, 0"));
        assert!(llvm.contains("store i64 %ssa0, ptr %counter.addr"));
        assert!(llvm.contains("%ssa1 = add i64 %"));
        assert!(llvm.contains("store i64 %ssa1, ptr %counter.addr"));
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
