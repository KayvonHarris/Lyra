//! Compiler-owned intermediate representation for Lyra.
//!
//! Lyra IR sits between the validated AST and backend-specific IRs such as
//! LLVM IR. It preserves Lyra semantics without coupling the language to a
//! particular code-generation framework.

use std::collections::HashMap;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub struct SsaValue {
    pub id: ValueId,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ValueTable {
    values: Vec<SsaValue>,
}

impl ValueTable {
    #[must_use]
    pub fn allocate(&mut self, ty: Type, span: Span) -> ValueId {
        let id = ValueId(self.values.len());
        self.values.push(SsaValue { id, ty, span });
        id
    }

    #[must_use]
    pub fn get(&self, id: ValueId) -> Option<&SsaValue> {
        self.values.get(id.0)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    Return {
        value: Option<Value>,
        span: Span,
    },
    Jump {
        target: BlockId,
        span: Span,
    },
    Branch {
        condition: Value,
        then_target: BlockId,
        else_target: BlockId,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueDefinition {
    pub id: ValueId,
    pub instruction_index: usize,
    pub uses: Vec<ValueId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhiNode {
    pub id: ValueId,
    pub name: String,
    pub incoming: Vec<(BlockId, ValueId)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
    pub definitions: Vec<ValueDefinition>,
    pub phi_nodes: Vec<PhiNode>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ControlFlowGraph {
    pub blocks: Vec<BasicBlock>,
    pub entry_definitions: HashMap<BlockId, HashMap<String, ValueId>>,
    pub exit_definitions: HashMap<BlockId, HashMap<String, ValueId>>,
}

impl ControlFlowGraph {
    #[must_use]
    pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.iter().find(|block| block.id == id)
    }

    #[must_use]
    pub fn predecessors(&self, id: BlockId) -> Vec<BlockId> {
        self.blocks
            .iter()
            .filter(|block| self.successors(block.id).contains(&id))
            .map(|block| block.id)
            .collect()
    }

    #[must_use]
    pub fn definition_at_entry(&self, block: BlockId, name: &str) -> Option<ValueId> {
        self.entry_definitions
            .get(&block)
            .and_then(|definitions| definitions.get(name))
            .copied()
    }

    #[must_use]
    pub fn definition_at_exit(&self, block: BlockId, name: &str) -> Option<ValueId> {
        self.exit_definitions
            .get(&block)
            .and_then(|definitions| definitions.get(name))
            .copied()
    }

    #[must_use]
    pub fn successors(&self, id: BlockId) -> Vec<BlockId> {
        let Some(block) = self.block(id) else {
            return Vec::new();
        };

        match &block.terminator {
            Terminator::Return { .. } => Vec::new(),
            Terminator::Jump { target, .. } => vec![*target],
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => vec![*then_target, *else_target],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Bind {
        name: String,
        value: Value,
        span: Span,
    },
    BindMutable {
        name: String,
        value: Value,
        span: Span,
    },
    Assign {
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
    While {
        condition: Value,
        body: Block,
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
        lyra_ast::Statement::Var { name, value, span } => Instruction::BindMutable {
            name: name.clone(),
            value: lower_expression(value),
            span: *span,
        },
        lyra_ast::Statement::Assign { name, value, span } => Instruction::Assign {
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
        lyra_ast::Statement::While {
            condition,
            body,
            span,
        } => Instruction::While {
            condition: lower_expression(condition),
            body: lower_block(body),
            span: *span,
        },
        lyra_ast::Statement::Expression { expression, span } => Instruction::Evaluate {
            value: lower_expression(expression),
            span: *span,
        },
    }
}

#[must_use]
pub fn build_cfg(block: &Block) -> ControlFlowGraph {
    let mut builder = CfgBuilder::default();
    let entry = builder.new_block();
    builder.lower_block(block, entry);
    builder.insert_phi_nodes();
    let (entry_definitions, exit_definitions) = builder.compute_reaching_definitions();
    ControlFlowGraph {
        blocks: builder.blocks,
        entry_definitions,
        exit_definitions,
    }
}

#[derive(Default)]
struct CfgBuilder {
    blocks: Vec<BasicBlock>,
    terminated: Vec<bool>,
    next_value: usize,
    locals: HashMap<String, ValueId>,
}

impl CfgBuilder {
    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len());
        self.blocks.push(BasicBlock {
            id,
            instructions: Vec::new(),
            terminator: Terminator::Return {
                value: None,
                span: Span::default(),
            },
            definitions: Vec::new(),
            phi_nodes: Vec::new(),
        });
        self.terminated.push(false);
        id
    }

    fn set_terminator(&mut self, id: BlockId, terminator: Terminator) {
        self.blocks[id.0].terminator = terminator;
        self.terminated[id.0] = true;
    }

    fn is_terminated(&self, id: BlockId) -> bool {
        self.terminated[id.0]
    }

    fn compute_reaching_definitions(
        &self,
    ) -> (
        HashMap<BlockId, HashMap<String, ValueId>>,
        HashMap<BlockId, HashMap<String, ValueId>>,
    ) {
        let mut entry: HashMap<BlockId, HashMap<String, ValueId>> = HashMap::new();
        let mut exit: HashMap<BlockId, HashMap<String, ValueId>> = HashMap::new();
        let max_iterations = self.blocks.len().saturating_mul(4).max(1);

        for _ in 0..max_iterations {
            let mut changed = false;
            for block in &self.blocks {
                let mut incoming = HashMap::new();
                let predecessors = self.predecessors_of(block.id);
                for predecessor in predecessors {
                    if let Some(definitions) = exit.get(&predecessor) {
                        for (name, id) in definitions {
                            incoming.entry(name.clone()).or_insert(*id);
                        }
                    }
                }
                for phi in &block.phi_nodes {
                    incoming.insert(phi.name.clone(), phi.id);
                }

                if entry.get(&block.id) != Some(&incoming) {
                    entry.insert(block.id, incoming.clone());
                    changed = true;
                }

                let mut outgoing = incoming;
                for definition in &block.definitions {
                    if let Some(instruction) = block.instructions.get(definition.instruction_index) {
                        if let Some(name) = defined_name(instruction) {
                            outgoing.insert(name.to_owned(), definition.id);
                        }
                    }
                }
                if exit.get(&block.id) != Some(&outgoing) {
                    exit.insert(block.id, outgoing);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        (entry, exit)
    }

    fn insert_phi_nodes(&mut self) {
        let block_ids: Vec<BlockId> = self.blocks.iter().map(|block| block.id).collect();

        for block_id in block_ids {
            let predecessors = self.predecessors_of(block_id);
            if predecessors.len() < 2 {
                continue;
            }

            let mut incoming_by_name: HashMap<String, Vec<(BlockId, ValueId)>> = HashMap::new();
            for predecessor in predecessors {
                for (name, id) in self.definitions_reaching_end(predecessor) {
                    incoming_by_name
                        .entry(name)
                        .or_default()
                        .push((predecessor, id));
                }
            }

            let mut names: Vec<String> = incoming_by_name.keys().cloned().collect();
            names.sort();
            for name in names {
                let incoming = incoming_by_name.remove(&name).unwrap_or_default();
                if incoming.len() < 2 {
                    continue;
                }
                let first = incoming[0].1;
                if incoming.iter().all(|(_, id)| *id == first) {
                    continue;
                }

                let id = ValueId(self.next_value);
                self.next_value += 1;
                self.blocks[block_id.0].phi_nodes.push(PhiNode { id, name, incoming });
            }
        }
    }

    fn predecessors_of(&self, id: BlockId) -> Vec<BlockId> {
        self.blocks
            .iter()
            .filter(|block| terminator_targets(&block.terminator).contains(&id))
            .map(|block| block.id)
            .collect()
    }

    fn definitions_reaching_end(&self, id: BlockId) -> HashMap<String, ValueId> {
        let mut definitions = HashMap::new();
        for definition in &self.blocks[id.0].definitions {
            if let Some(instruction) = self.blocks[id.0]
                .instructions
                .get(definition.instruction_index)
            {
                if let Some(name) = defined_name(instruction) {
                    definitions.insert(name.to_owned(), definition.id);
                }
            }
        }
        definitions
    }

    fn instruction_uses(&self, instruction: &Instruction) -> Vec<ValueId> {
        let value = match instruction {
            Instruction::Bind { value, .. }
            | Instruction::BindMutable { value, .. }
            | Instruction::Assign { value, .. }
            | Instruction::Evaluate { value, .. } => value,
            Instruction::Return { .. } | Instruction::If { .. } | Instruction::While { .. } => {
                return Vec::new();
            }
        };

        let mut uses = Vec::new();
        self.collect_value_uses(value, &mut uses);
        uses
    }

    fn collect_value_uses(&self, value: &Value, uses: &mut Vec<ValueId>) {
        match value {
            Value::Local(name, _) => {
                if let Some(id) = self.locals.get(name) {
                    uses.push(*id);
                }
            }
            Value::Call { arguments, .. } => {
                for argument in arguments {
                    self.collect_value_uses(argument, uses);
                }
            }
            Value::Unary { operand, .. } => self.collect_value_uses(operand, uses),
            Value::Binary { left, right, .. } => {
                self.collect_value_uses(left, uses);
                self.collect_value_uses(right, uses);
            }
            Value::Integer(..)
            | Value::Float(..)
            | Value::String(..)
            | Value::Boolean(..) => {}
        }
    }

    fn lower_block(&mut self, block: &Block, mut current: BlockId) -> BlockId {
        for instruction in &block.instructions {
            match instruction {
                Instruction::If {
                    condition,
                    then_block,
                    else_block,
                    span,
                } => {
                    let then_id = self.new_block();
                    let else_id = self.new_block();
                    let merge_id = self.new_block();
                    self.set_terminator(
                        current,
                        Terminator::Branch {
                            condition: condition.clone(),
                            then_target: then_id,
                            else_target: else_id,
                            span: *span,
                        },
                    );
                    let then_end = self.lower_block(then_block, then_id);
                    if !self.is_terminated(then_end) {
                        self.set_terminator(
                            then_end,
                            Terminator::Jump {
                                target: merge_id,
                                span: *span,
                            },
                        );
                    }
                    if let Some(else_block) = else_block {
                        let else_end = self.lower_block(else_block, else_id);
                        if !self.is_terminated(else_end) {
                            self.set_terminator(
                                else_end,
                                Terminator::Jump {
                                    target: merge_id,
                                    span: *span,
                                },
                            );
                        }
                    } else {
                        self.set_terminator(
                            else_id,
                            Terminator::Jump {
                                target: merge_id,
                                span: *span,
                            },
                        );
                    }
                    current = merge_id;
                }
                Instruction::While {
                    condition,
                    body,
                    span,
                } => {
                    let condition_id = self.new_block();
                    let body_id = self.new_block();
                    let exit_id = self.new_block();
                    self.set_terminator(
                        current,
                        Terminator::Jump {
                            target: condition_id,
                            span: *span,
                        },
                    );
                    self.set_terminator(
                        condition_id,
                        Terminator::Branch {
                            condition: condition.clone(),
                            then_target: body_id,
                            else_target: exit_id,
                            span: *span,
                        },
                    );
                    let body_end = self.lower_block(body, body_id);
                    if !self.is_terminated(body_end) {
                        self.set_terminator(
                            body_end,
                            Terminator::Jump {
                                target: condition_id,
                                span: *span,
                            },
                        );
                    }
                    current = exit_id;
                }
                Instruction::Return { value, span } => {
                    self.set_terminator(
                        current,
                        Terminator::Return {
                            value: value.clone(),
                            span: *span,
                        },
                    );
                    return current;
                }
                other => {
                    let instruction_index = self.blocks[current.0].instructions.len();
                    let uses = self.instruction_uses(other);
                    self.blocks[current.0].instructions.push(other.clone());

                    if let Some(name) = defined_name(other) {
                        let id = ValueId(self.next_value);
                        self.next_value += 1;
                        self.locals.insert(name.to_owned(), id);
                        self.blocks[current.0].definitions.push(ValueDefinition {
                            id,
                            instruction_index,
                            uses,
                        });
                    }
                }
            }
        }
        current
    }
}

fn terminator_targets(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Return { .. } => Vec::new(),
        Terminator::Jump { target, .. } => vec![*target],
        Terminator::Branch {
            then_target,
            else_target,
            ..
        } => vec![*then_target, *else_target],
    }
}

fn defined_name(instruction: &Instruction) -> Option<&str> {
    match instruction {
        Instruction::Bind { name, .. }
        | Instruction::BindMutable { name, .. }
        | Instruction::Assign { name, .. } => Some(name),
        Instruction::Evaluate { .. }
        | Instruction::Return { .. }
        | Instruction::If { .. }
        | Instruction::While { .. } => None,
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
    fn lowers_mutable_bindings_and_assignment() {
        let module = lower_source(
            "fn main() -> Int { var counter = 0; counter = counter + 1; return counter; }",
        );
        assert!(matches!(
            &module.functions[0].body.instructions[0],
            Instruction::BindMutable { name, .. } if name == "counter"
        ));
        assert!(matches!(
            &module.functions[0].body.instructions[1],
            Instruction::Assign { name, .. } if name == "counter"
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
    fn lowers_while_control_flow() {
        let module = lower_source("fn main() -> Int { while true { return 42; } return 0; }");
        assert!(matches!(
            &module.functions[0].body.instructions[0],
            Instruction::While {
                condition: Value::Boolean(true, _),
                body,
                ..
            } if matches!(body.instructions[0], Instruction::Return { .. })
        ));
    }

    #[test]
    fn builds_cfg_for_if_and_while() {
        let module = lower_source(
            "fn main() -> Int { var counter = 0; while counter < 2 { if counter == 1 { return 42; } counter = counter + 1; } return 0; }",
        );
        let cfg = build_cfg(&module.functions[0].body);

        assert!(cfg.blocks.len() >= 7);
        assert!(
            cfg.blocks
                .iter()
                .any(|block| matches!(block.terminator, Terminator::Branch { .. }))
        );
        assert!(
            cfg.blocks
                .iter()
                .any(|block| matches!(block.terminator, Terminator::Jump { .. }))
        );
        assert!(cfg.blocks.iter().any(|block| matches!(
            block.terminator,
            Terminator::Return {
                value: Some(Value::Integer(42, _)),
                ..
            }
        )));
    }

    #[test]
    fn models_basic_block_successors() {
        let span = Span { start: 0, end: 0 };
        let cfg = ControlFlowGraph {
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    instructions: vec![],
                    terminator: Terminator::Branch {
                        condition: Value::Boolean(true, span),
                        then_target: BlockId(1),
                        else_target: BlockId(2),
                        span,
                    },
                    definitions: vec![],
                    phi_nodes: vec![],
                },
                BasicBlock {
                    id: BlockId(1),
                    instructions: vec![],
                    terminator: Terminator::Jump {
                        target: BlockId(2),
                        span,
                    },
                    definitions: vec![],
                    phi_nodes: vec![],
                },
                BasicBlock {
                    id: BlockId(2),
                    instructions: vec![],
                    terminator: Terminator::Return {
                        value: Some(Value::Integer(42, span)),
                        span,
                    },
                    definitions: vec![],
                    phi_nodes: vec![],
                },
            ],
            entry_definitions: HashMap::new(),
            exit_definitions: HashMap::new(),
        };

        assert_eq!(cfg.successors(BlockId(0)), vec![BlockId(1), BlockId(2)]);
        assert_eq!(cfg.successors(BlockId(1)), vec![BlockId(2)]);
        assert!(cfg.successors(BlockId(2)).is_empty());
        assert!(cfg.predecessors(BlockId(0)).is_empty());
        assert_eq!(cfg.predecessors(BlockId(1)), vec![BlockId(0)]);
        assert_eq!(cfg.predecessors(BlockId(2)), vec![BlockId(0), BlockId(1)]);
        assert!(cfg.block(BlockId(99)).is_none());
    }

    #[test]
    fn propagates_reaching_definitions_through_intermediate_blocks() {
        let module = lower_source(
            "fn main() -> Int { var value = 1; if true { let other = value + 1; } return value; }",
        );
        let cfg = build_cfg(&module.functions[0].body);
        let merge = cfg
            .blocks
            .iter()
            .find(|block| cfg.predecessors(block.id).len() == 2)
            .expect("merge block");

        assert_eq!(cfg.definition_at_entry(merge.id, "value"), Some(ValueId(0)));
        assert_eq!(cfg.definition_at_exit(merge.id, "value"), Some(ValueId(0)));
    }

    #[test]
    fn inserts_phi_for_distinct_branch_definitions() {
        let module = lower_source(
            "fn main() -> Int { var value = 0; if true { value = 20; } else { value = 22; } return value; }",
        );
        let cfg = build_cfg(&module.functions[0].body);
        let merge = cfg
            .blocks
            .iter()
            .find(|block| block.phi_nodes.iter().any(|phi| phi.name == "value"))
            .expect("merge block with value phi");
        let phi = merge
            .phi_nodes
            .iter()
            .find(|phi| phi.name == "value")
            .expect("value phi");

        assert_eq!(phi.incoming.len(), 2);
        assert_ne!(phi.incoming[0].1, phi.incoming[1].1);
    }

    #[test]
    fn basic_blocks_can_record_phi_nodes() {
        let phi = PhiNode {
            id: ValueId(4),
            name: "counter".to_owned(),
            incoming: vec![(BlockId(1), ValueId(2)), (BlockId(2), ValueId(3))],
        };

        assert_eq!(phi.id, ValueId(4));
        assert_eq!(phi.name, "counter");
        assert_eq!(phi.incoming.len(), 2);
        assert_eq!(phi.incoming[0], (BlockId(1), ValueId(2)));
        assert_eq!(phi.incoming[1], (BlockId(2), ValueId(3)));
    }

    #[test]
    fn basic_blocks_can_record_ssa_definitions() {
        let definition = ValueDefinition {
            id: ValueId(3),
            instruction_index: 1,
            uses: vec![ValueId(1), ValueId(2)],
        };

        assert_eq!(definition.id, ValueId(3));
        assert_eq!(definition.instruction_index, 1);
        assert_eq!(definition.uses, vec![ValueId(1), ValueId(2)]);
    }

    #[test]
    fn cfg_assigns_value_ids_and_tracks_uses() {
        let module = lower_source(
            "fn main() -> Int { let first = 20; let second = first + 22; return second; }",
        );
        let cfg = build_cfg(&module.functions[0].body);
        let entry = cfg.block(BlockId(0)).expect("entry block");

        assert_eq!(entry.definitions.len(), 2);
        assert_eq!(entry.definitions[0].id, ValueId(0));
        assert!(entry.definitions[0].uses.is_empty());
        assert_eq!(entry.definitions[1].id, ValueId(1));
        assert_eq!(entry.definitions[1].uses, vec![ValueId(0)]);
    }

    #[test]
    fn cfg_assignment_creates_a_new_value_version() {
        let module = lower_source(
            "fn main() -> Int { var counter = 0; counter = counter + 1; return counter; }",
        );
        let cfg = build_cfg(&module.functions[0].body);
        let entry = cfg.block(BlockId(0)).expect("entry block");

        assert_eq!(entry.definitions.len(), 2);
        assert_eq!(entry.definitions[0].id, ValueId(0));
        assert_eq!(entry.definitions[1].id, ValueId(1));
        assert_eq!(entry.definitions[1].uses, vec![ValueId(0)]);
    }

    #[test]
    fn allocates_stable_ssa_value_ids() {
        let span = Span { start: 4, end: 9 };
        let mut values = ValueTable::default();

        let first = values.allocate(Type::Integer, span);
        let second = values.allocate(Type::Boolean, span);

        assert_eq!(first, ValueId(0));
        assert_eq!(second, ValueId(1));
        assert_eq!(values.len(), 2);
        assert_eq!(values.get(first).map(|value| value.ty), Some(Type::Integer));
        assert_eq!(values.get(second).map(|value| value.ty), Some(Type::Boolean));
        assert!(values.get(ValueId(99)).is_none());
    }

    #[test]
    fn models_explicit_control_flow_terminators() {
        let span = Span { start: 0, end: 0 };
        let branch = Terminator::Branch {
            condition: Value::Boolean(true, span),
            then_target: BlockId(1),
            else_target: BlockId(2),
            span,
        };
        let jump = Terminator::Jump {
            target: BlockId(3),
            span,
        };
        let ret = Terminator::Return {
            value: Some(Value::Integer(42, span)),
            span,
        };

        assert!(matches!(
            branch,
            Terminator::Branch {
                then_target: BlockId(1),
                else_target: BlockId(2),
                ..
            }
        ));
        assert!(matches!(
            jump,
            Terminator::Jump {
                target: BlockId(3),
                ..
            }
        ));
        assert!(matches!(
            ret,
            Terminator::Return {
                value: Some(Value::Integer(42, _)),
                ..
            }
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
