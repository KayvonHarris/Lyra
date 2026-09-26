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
    pub binding: BindingId,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BindingId(pub usize);

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
    /// Builder state for a block that has not received a real terminator yet.
    Open,
    /// A semantically unreachable block.
    Unreachable,
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
    pub binding: BindingId,
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

pub type DefinitionMap = HashMap<BindingId, ValueId>;
pub type BlockDefinitionMap = HashMap<BlockId, DefinitionMap>;

#[derive(Debug, Clone, PartialEq)]
pub struct ControlFlowGraph {
    pub entry: BlockId,
    pub blocks: Vec<BasicBlock>,
    pub entry_definitions: BlockDefinitionMap,
    pub exit_definitions: BlockDefinitionMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CfgValidationError {
    MissingEntryBlock,
    ReachableOpenBlock { block: BlockId },
    MissingSuccessor { block: BlockId, successor: BlockId },
}

impl ControlFlowGraph {
    #[must_use]
    pub fn reachable_blocks(&self) -> std::collections::HashSet<BlockId> {
        let mut reachable = std::collections::HashSet::new();
        if self.block(self.entry).is_none() {
            return reachable;
        }

        let mut pending = vec![self.entry];
        while let Some(block) = pending.pop() {
            if !reachable.insert(block) {
                continue;
            }
            pending.extend(self.successors(block));
        }
        reachable
    }

    pub fn validate_reachable(&self, allow_open_exit: bool) -> Result<(), CfgValidationError> {
        if self.block(self.entry).is_none() {
            return Err(CfgValidationError::MissingEntryBlock);
        }

        let reachable = self.reachable_blocks();
        for block in &self.blocks {
            if !reachable.contains(&block.id) {
                continue;
            }

            if matches!(block.terminator, Terminator::Open) && !allow_open_exit {
                return Err(CfgValidationError::ReachableOpenBlock { block: block.id });
            }

            for successor in self.successors(block.id) {
                if self.block(successor).is_none() {
                    return Err(CfgValidationError::MissingSuccessor {
                        block: block.id,
                        successor,
                    });
                }
            }
        }
        Ok(())
    }

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
        self.entry_definitions.get(&block).and_then(|definitions| {
            definitions
                .iter()
                .filter_map(|(binding, value)| {
                    (binding_name(&self.blocks, *binding) == Some(name))
                        .then_some((*binding, *value))
                })
                .min_by_key(|(binding, _)| *binding)
                .map(|(_, value)| value)
        })
    }

    #[must_use]
    pub fn definition_at_exit(&self, block: BlockId, name: &str) -> Option<ValueId> {
        self.exit_definitions.get(&block).and_then(|definitions| {
            definitions
                .iter()
                .filter_map(|(binding, value)| {
                    (binding_name(&self.blocks, *binding) == Some(name))
                        .then_some((*binding, *value))
                })
                .min_by_key(|(binding, _)| *binding)
                .map(|(_, value)| value)
        })
    }

    #[must_use]
    pub fn successors(&self, id: BlockId) -> Vec<BlockId> {
        let Some(block) = self.block(id) else {
            return Vec::new();
        };

        match &block.terminator {
            Terminator::Open | Terminator::Unreachable | Terminator::Return { .. } => Vec::new(),
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
        binding: BindingId,
        value: Value,
        span: Span,
    },
    BindMutable {
        name: String,
        binding: BindingId,
        value: Value,
        span: Span,
    },
    Assign {
        name: String,
        binding: BindingId,
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
    Local {
        name: String,
        binding: BindingId,
        span: Span,
    },
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
            | Self::Local { span, .. }
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
    let mut lowerer = Lowerer::default();
    lowerer.push_scope();

    let parameters = function
        .parameters
        .iter()
        .map(|parameter| {
            let binding = lowerer.declare(&parameter.name);
            Parameter {
                name: parameter.name.clone(),
                binding,
                ty: parameter
                    .type_name
                    .as_ref()
                    .map_or(Type::Integer, lower_type_name),
                span: parameter.span,
            }
        })
        .collect();

    let body = lowerer.lower_block(&function.body, false);
    lowerer.pop_scope();

    Function {
        name: function.name.clone(),
        parameters,
        return_type: function
            .return_type
            .as_ref()
            .map_or(Type::Integer, lower_type_name),
        body,
        span: function.span,
    }
}

#[derive(Default)]
struct Lowerer {
    next_binding: usize,
    scopes: Vec<HashMap<String, BindingId>>,
}

impl Lowerer {
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop().expect("lowering scope must exist");
    }

    fn declare(&mut self, name: &str) -> BindingId {
        let binding = BindingId(self.next_binding);
        self.next_binding += 1;
        self.scopes
            .last_mut()
            .expect("lowering scope must exist")
            .insert(name.to_owned(), binding);
        binding
    }

    fn resolve(&self, name: &str) -> BindingId {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .expect("semantic analysis guarantees resolved local bindings")
    }

    fn lower_block(&mut self, block: &lyra_ast::Block, nested: bool) -> Block {
        if nested {
            self.push_scope();
        }
        let instructions = block
            .statements
            .iter()
            .map(|statement| self.lower_statement(statement))
            .collect();
        if nested {
            self.pop_scope();
        }
        Block { instructions }
    }

    fn lower_statement(&mut self, statement: &lyra_ast::Statement) -> Instruction {
        match statement {
            lyra_ast::Statement::Let { name, value, span } => {
                let value = self.lower_expression(value);
                let binding = self.declare(name);
                Instruction::Bind {
                    name: name.clone(),
                    binding,
                    value,
                    span: *span,
                }
            }
            lyra_ast::Statement::Var { name, value, span } => {
                let value = self.lower_expression(value);
                let binding = self.declare(name);
                Instruction::BindMutable {
                    name: name.clone(),
                    binding,
                    value,
                    span: *span,
                }
            }
            lyra_ast::Statement::Assign { name, value, span } => Instruction::Assign {
                name: name.clone(),
                binding: self.resolve(name),
                value: self.lower_expression(value),
                span: *span,
            },
            lyra_ast::Statement::Return { value, span } => Instruction::Return {
                value: value.as_ref().map(|value| self.lower_expression(value)),
                span: *span,
            },
            lyra_ast::Statement::If {
                condition,
                then_block,
                else_block,
                span,
            } => {
                let condition = self.lower_expression(condition);
                let then_block = self.lower_block(then_block, true);
                let else_block = else_block
                    .as_ref()
                    .map(|block| self.lower_block(block, true));
                Instruction::If {
                    condition,
                    then_block,
                    else_block,
                    span: *span,
                }
            }
            lyra_ast::Statement::While {
                condition,
                body,
                span,
            } => {
                let condition = self.lower_expression(condition);
                let body = self.lower_block(body, true);
                Instruction::While {
                    condition,
                    body,
                    span: *span,
                }
            }
            lyra_ast::Statement::Expression { expression, span } => Instruction::Evaluate {
                value: self.lower_expression(expression),
                span: *span,
            },
        }
    }

    fn lower_expression(&self, expression: &lyra_ast::Expression) -> Value {
        match expression {
            lyra_ast::Expression::Integer(value, span) => Value::Integer(*value, *span),
            lyra_ast::Expression::Float(value, span) => Value::Float(*value, *span),
            lyra_ast::Expression::String(value, span) => Value::String(value.clone(), *span),
            lyra_ast::Expression::Boolean(value, span) => Value::Boolean(*value, *span),
            lyra_ast::Expression::Identifier(name, span) => Value::Local {
                name: name.clone(),
                binding: self.resolve(name),
                span: *span,
            },
            lyra_ast::Expression::Call {
                callee,
                arguments,
                span,
            } => Value::Call {
                callee: callee.clone(),
                arguments: arguments
                    .iter()
                    .map(|argument| self.lower_expression(argument))
                    .collect(),
                span: *span,
            },
            lyra_ast::Expression::Unary {
                operator,
                operand,
                span,
            } => Value::Unary {
                operator: lower_unary_operator(*operator),
                operand: Box::new(self.lower_expression(operand)),
                span: *span,
            },
            lyra_ast::Expression::Binary {
                left,
                operator,
                right,
                span,
            } => Value::Binary {
                left: Box::new(self.lower_expression(left)),
                operator: lower_binary_operator(*operator),
                right: Box::new(self.lower_expression(right)),
                span: *span,
            },
        }
    }
}

#[must_use]
pub fn build_cfg(block: &Block) -> ControlFlowGraph {
    let mut builder = CfgBuilder::default();
    let entry = builder.new_block();
    builder.lower_block(block, entry);
    let (entry_definitions, exit_definitions) = builder.construct_ssa();
    ControlFlowGraph {
        entry,
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
}

impl CfgBuilder {
    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len());
        self.blocks.push(BasicBlock {
            id,
            instructions: Vec::new(),
            terminator: Terminator::Open,
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

    fn construct_ssa(&mut self) -> (BlockDefinitionMap, BlockDefinitionMap) {
        let mut entry: BlockDefinitionMap = HashMap::new();
        let mut exit: BlockDefinitionMap = HashMap::new();
        let mut phi_ids: HashMap<(BlockId, BindingId), ValueId> = HashMap::new();

        loop {
            let mut changed = false;

            for block_index in 0..self.blocks.len() {
                let block_id = BlockId(block_index);
                let predecessors = self.predecessors_of(block_id);
                let previous_entry = entry.get(&block_id).cloned().unwrap_or_default();
                let previous_exit = exit.get(&block_id).cloned().unwrap_or_default();
                let mut incoming = DefinitionMap::new();

                if predecessors.len() == 1 {
                    if let Some(definitions) = exit.get(&predecessors[0]) {
                        incoming.extend(definitions.iter().map(|(binding, id)| (*binding, *id)));
                    }
                } else if predecessors.len() >= 2 {
                    let mut names = predecessors
                        .iter()
                        .filter_map(|predecessor| exit.get(predecessor))
                        .flat_map(|definitions| definitions.keys().copied())
                        .collect::<Vec<_>>();
                    names.sort();
                    names.dedup();

                    for name in names {
                        let values = predecessors
                            .iter()
                            .map(|predecessor| {
                                exit.get(predecessor)
                                    .and_then(|definitions| definitions.get(&name))
                                    .copied()
                            })
                            .collect::<Vec<_>>();
                        let known = values.iter().flatten().copied().collect::<Vec<_>>();

                        if known.is_empty() {
                            continue;
                        }

                        let all_predecessors_known = known.len() == predecessors.len();
                        let first = known[0];
                        let all_same = known.iter().all(|id| *id == first);

                        if !all_predecessors_known {
                            if all_same {
                                incoming.insert(name, first);
                            }
                            continue;
                        }

                        let key = (block_id, name);
                        if all_same && !phi_ids.contains_key(&key) {
                            incoming.insert(name, first);
                            continue;
                        }

                        let phi_id = *phi_ids.entry(key).or_insert_with(|| {
                            let id = ValueId(self.next_value);
                            self.next_value += 1;
                            id
                        });
                        let phi_incoming = predecessors
                            .iter()
                            .zip(values)
                            .map(|(predecessor, value)| {
                                (
                                    *predecessor,
                                    value.expect("all predecessor definitions known"),
                                )
                            })
                            .collect::<Vec<_>>();

                        let phi_name = binding_name(&self.blocks, name)
                            .unwrap_or("<binding>")
                            .to_owned();
                        let block = &mut self.blocks[block_index];
                        if let Some(phi) =
                            block.phi_nodes.iter_mut().find(|phi| phi.binding == name)
                        {
                            if phi.incoming != phi_incoming {
                                phi.incoming = phi_incoming;
                                changed = true;
                            }
                        } else {
                            block.phi_nodes.push(PhiNode {
                                id: phi_id,
                                binding: name,
                                name: phi_name,
                                incoming: phi_incoming,
                            });
                            block.phi_nodes.sort_by_key(|phi| phi.binding);
                            changed = true;
                        }
                        incoming.insert(name, phi_id);
                    }
                }

                if previous_entry != incoming {
                    entry.insert(block_id, incoming.clone());
                    changed = true;
                }

                let mut outgoing = incoming;
                for definition in &self.blocks[block_index].definitions {
                    if let Some(instruction) = self.blocks[block_index]
                        .instructions
                        .get(definition.instruction_index)
                        && let Some(binding) = defined_binding(instruction)
                    {
                        outgoing.insert(binding, definition.id);
                    }
                }

                if previous_exit != outgoing {
                    exit.insert(block_id, outgoing);
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        self.resolve_definition_uses(&entry);

        (entry, exit)
    }

    fn resolve_definition_uses(&mut self, entry: &BlockDefinitionMap) {
        for block_index in 0..self.blocks.len() {
            let block_id = BlockId(block_index);
            let mut environment = entry.get(&block_id).cloned().unwrap_or_default();

            for definition_index in 0..self.blocks[block_index].definitions.len() {
                let instruction_index =
                    self.blocks[block_index].definitions[definition_index].instruction_index;
                let instruction = self.blocks[block_index].instructions[instruction_index].clone();
                let mut uses = Vec::new();
                collect_value_uses_from_environment(
                    instruction_value(&instruction),
                    &environment,
                    &mut uses,
                );
                self.blocks[block_index].definitions[definition_index].uses = uses;

                if let Some(name) = defined_binding(&instruction) {
                    let id = self.blocks[block_index].definitions[definition_index].id;
                    environment.insert(name, id);
                }
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
                    match condition {
                        Value::Boolean(true, _) => self.set_terminator(
                            condition_id,
                            Terminator::Jump {
                                target: body_id,
                                span: *span,
                            },
                        ),
                        Value::Boolean(false, _) => self.set_terminator(
                            condition_id,
                            Terminator::Jump {
                                target: exit_id,
                                span: *span,
                            },
                        ),
                        _ => self.set_terminator(
                            condition_id,
                            Terminator::Branch {
                                condition: condition.clone(),
                                then_target: body_id,
                                else_target: exit_id,
                                span: *span,
                            },
                        ),
                    }
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
                    self.blocks[current.0].instructions.push(other.clone());

                    if defined_binding(other).is_some() {
                        let id = ValueId(self.next_value);
                        self.next_value += 1;
                        self.blocks[current.0].definitions.push(ValueDefinition {
                            id,
                            instruction_index,
                            uses: Vec::new(),
                        });
                    }
                }
            }
        }
        current
    }
}

fn instruction_value(instruction: &Instruction) -> Option<&Value> {
    match instruction {
        Instruction::Bind { value, .. }
        | Instruction::BindMutable { value, .. }
        | Instruction::Assign { value, .. }
        | Instruction::Evaluate { value, .. } => Some(value),
        Instruction::Return { .. } | Instruction::If { .. } | Instruction::While { .. } => None,
    }
}

fn collect_value_uses_from_environment(
    value: Option<&Value>,
    environment: &DefinitionMap,
    uses: &mut Vec<ValueId>,
) {
    let Some(value) = value else {
        return;
    };

    match value {
        Value::Local { binding, .. } => {
            if let Some(id) = environment.get(binding) {
                uses.push(*id);
            }
        }
        Value::Call { arguments, .. } => {
            for argument in arguments {
                collect_value_uses_from_environment(Some(argument), environment, uses);
            }
        }
        Value::Unary { operand, .. } => {
            collect_value_uses_from_environment(Some(operand), environment, uses);
        }
        Value::Binary { left, right, .. } => {
            collect_value_uses_from_environment(Some(left), environment, uses);
            collect_value_uses_from_environment(Some(right), environment, uses);
        }
        Value::Integer(..) | Value::Float(..) | Value::String(..) | Value::Boolean(..) => {}
    }
}

fn terminator_targets(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Open | Terminator::Unreachable | Terminator::Return { .. } => Vec::new(),
        Terminator::Jump { target, .. } => vec![*target],
        Terminator::Branch {
            then_target,
            else_target,
            ..
        } => vec![*then_target, *else_target],
    }
}

fn defined_binding(instruction: &Instruction) -> Option<BindingId> {
    match instruction {
        Instruction::Bind { binding, .. }
        | Instruction::BindMutable { binding, .. }
        | Instruction::Assign { binding, .. } => Some(*binding),
        Instruction::Evaluate { .. }
        | Instruction::Return { .. }
        | Instruction::If { .. }
        | Instruction::While { .. } => None,
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

fn binding_name(blocks: &[BasicBlock], binding: BindingId) -> Option<&str> {
    blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| {
            (defined_binding(instruction) == Some(binding))
                .then(|| defined_name(instruction))
                .flatten()
        })
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
                value: Some(Value::Local { name, .. }),
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
        let module = lower_source("fn main() -> Int { while true { return 42; } }");
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
    fn unterminated_cfg_blocks_remain_explicitly_open() {
        let lexed = lyra_lexer::tokenize("fn main() -> Int { let value = 42; }");
        assert!(lexed.diagnostics.is_empty());
        let (ast, parser_diagnostics) = lyra_parser::parse(&lexed.tokens);
        assert!(parser_diagnostics.is_empty());
        let module = lower(&ast);
        let cfg = build_cfg(&module.functions[0].body);
        let entry = cfg.block(BlockId(0)).expect("entry block");

        assert!(matches!(entry.terminator, Terminator::Open));
        assert!(cfg.successors(entry.id).is_empty());
    }

    #[test]
    fn models_basic_block_successors() {
        let span = Span { start: 0, end: 0 };
        let cfg = ControlFlowGraph {
            entry: BlockId(0),
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
    fn inserts_phi_for_loop_carried_mutable_value() {
        let module = lower_source(
            "fn main() -> Int { var counter = 0; while counter < 2 { counter = counter + 1; } return counter; }",
        );
        let cfg = build_cfg(&module.functions[0].body);
        let loop_header = cfg
            .blocks
            .iter()
            .find(|block| {
                matches!(block.terminator, Terminator::Branch { .. })
                    && cfg.predecessors(block.id).len() == 2
            })
            .expect("loop header");
        let phi = loop_header
            .phi_nodes
            .iter()
            .find(|phi| phi.name == "counter")
            .expect("loop-carried counter phi");

        assert_eq!(phi.incoming.len(), 2);
        assert_ne!(phi.incoming[0].1, phi.incoming[1].1);
        assert_eq!(
            cfg.definition_at_entry(loop_header.id, "counter"),
            Some(phi.id)
        );
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
    fn branch_merge_inherits_missing_definitions_per_variable() {
        let module = lower_source(
            "fn main() -> Int { var counter = 0; var total = 0; while counter < 3 { if counter == 1 { total = total + 10; } else { total = total + 1; } counter = counter + 1; } return total; }",
        );
        let cfg = build_cfg(&module.functions[0].body);

        let nested_merge = cfg
            .blocks
            .iter()
            .find(|block| {
                cfg.predecessors(block.id).len() == 2
                    && block.phi_nodes.iter().any(|phi| phi.name == "total")
            })
            .expect("nested branch merge with total phi");

        assert!(
            nested_merge
                .phi_nodes
                .iter()
                .any(|phi| phi.name == "total" && phi.incoming.len() == 2)
        );
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
            binding: BindingId(0),
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
    fn inner_shadow_does_not_replace_outer_definition_after_branch() {
        let module = lower_source(
            "fn main() -> Int { let value = 1; if true { let value = 2; let inner = value; } return value; }",
        );
        let cfg = build_cfg(&module.functions[0].body);
        let merge = cfg
            .blocks
            .iter()
            .find(|block| cfg.predecessors(block.id).len() == 2)
            .expect("branch merge");

        assert_eq!(cfg.definition_at_entry(merge.id, "value"), Some(ValueId(0)));
        assert!(merge.phi_nodes.iter().all(|phi| phi.name != "value"));
    }

    #[test]
    fn shadow_initializer_reads_outer_binding() {
        let module = lower_source(
            "fn main() -> Int { let value = 1; if true { let value = value + 1; return value; } return value; }",
        );
        let outer_binding = match &module.functions[0].body.instructions[0] {
            Instruction::Bind { binding, .. } => *binding,
            _ => panic!("outer binding"),
        };
        let inner_binding = match &module.functions[0].body.instructions[1] {
            Instruction::If { then_block, .. } => match &then_block.instructions[0] {
                Instruction::Bind { binding, value, .. } => {
                    assert!(matches!(
                        value,
                        Value::Binary { left, .. }
                            if matches!(left.as_ref(), Value::Local { binding, .. } if *binding == outer_binding)
                    ));
                    *binding
                }
                _ => panic!("inner binding"),
            },
            _ => panic!("if statement"),
        };

        assert_ne!(outer_binding, inner_binding);
    }

    #[test]
    fn nested_shadow_levels_keep_distinct_binding_ids() {
        let module = lower_source(
            "fn main() -> Int { let value = 1; if true { let value = 2; if true { let value = 3; return value; } return value; } return value; }",
        );
        let outer = match &module.functions[0].body.instructions[0] {
            Instruction::Bind { binding, .. } => *binding,
            _ => panic!("outer binding"),
        };
        let (middle, inner) = match &module.functions[0].body.instructions[1] {
            Instruction::If { then_block, .. } => {
                let middle = match &then_block.instructions[0] {
                    Instruction::Bind { binding, .. } => *binding,
                    _ => panic!("middle binding"),
                };
                let inner = match &then_block.instructions[1] {
                    Instruction::If { then_block, .. } => match &then_block.instructions[0] {
                        Instruction::Bind { binding, .. } => *binding,
                        _ => panic!("inner binding"),
                    },
                    _ => panic!("nested if"),
                };
                (middle, inner)
            }
            _ => panic!("outer if"),
        };

        assert_ne!(outer, middle);
        assert_ne!(middle, inner);
        assert_ne!(outer, inner);
    }

    #[test]
    fn loop_local_shadow_does_not_replace_outer_binding() {
        let module = lower_source(
            "fn main() -> Int { let value = 1; while false { let value = 2; let inner = value; } return value; }",
        );
        let outer_binding = match &module.functions[0].body.instructions[0] {
            Instruction::Bind { binding, .. } => *binding,
            _ => panic!("outer binding"),
        };
        let return_binding = match module.functions[0].body.instructions.last() {
            Some(Instruction::Return {
                value: Some(Value::Local { binding, .. }),
                ..
            }) => *binding,
            _ => panic!("return local"),
        };

        assert_eq!(return_binding, outer_binding);
    }

    #[test]
    fn assignment_inside_shadow_scope_targets_inner_binding() {
        let module = lower_source(
            "fn main() -> Int { var value = 1; if true { var value = 2; value = value + 1; } return value; }",
        );
        let outer_binding = match &module.functions[0].body.instructions[0] {
            Instruction::BindMutable { binding, .. } => *binding,
            _ => panic!("outer binding"),
        };
        let (inner_binding, assignment_binding) = match &module.functions[0].body.instructions[1] {
            Instruction::If { then_block, .. } => {
                let inner = match &then_block.instructions[0] {
                    Instruction::BindMutable { binding, .. } => *binding,
                    _ => panic!("inner binding"),
                };
                let assignment = match &then_block.instructions[1] {
                    Instruction::Assign { binding, .. } => *binding,
                    _ => panic!("inner assignment"),
                };
                (inner, assignment)
            }
            _ => panic!("if statement"),
        };

        assert_ne!(outer_binding, inner_binding);
        assert_eq!(assignment_binding, inner_binding);
    }

    #[test]
    fn sibling_scopes_with_same_name_have_distinct_bindings() {
        let module = lower_source(
            "fn main() -> Int { if true { let value = 1; } else { let value = 2; } return 0; }",
        );
        let (then_binding, else_binding) = match &module.functions[0].body.instructions[0] {
            Instruction::If {
                then_block,
                else_block: Some(else_block),
                ..
            } => {
                let then_binding = match &then_block.instructions[0] {
                    Instruction::Bind { binding, .. } => *binding,
                    _ => panic!("then binding"),
                };
                let else_binding = match &else_block.instructions[0] {
                    Instruction::Bind { binding, .. } => *binding,
                    _ => panic!("else binding"),
                };
                (then_binding, else_binding)
            }
            _ => panic!("if statement"),
        };

        assert_ne!(then_binding, else_binding);
    }

    #[test]
    fn parameter_shadowing_uses_distinct_binding_identity() {
        let module = lower_source(
            "fn choose(value: Int) -> Int { if true { let value = 2; return value; } return value; }",
        );
        let parameter_binding = module.functions[0].parameters[0].binding;
        let inner_binding = match &module.functions[0].body.instructions[0] {
            Instruction::If { then_block, .. } => match &then_block.instructions[0] {
                Instruction::Bind { binding, .. } => *binding,
                _ => panic!("inner binding"),
            },
            _ => panic!("if statement"),
        };
        let outer_return_binding = match &module.functions[0].body.instructions[1] {
            Instruction::Return {
                value: Some(Value::Local { binding, .. }),
                ..
            } => *binding,
            _ => panic!("outer return"),
        };

        assert_ne!(parameter_binding, inner_binding);
        assert_eq!(outer_return_binding, parameter_binding);
    }

    #[test]
    fn outer_assignment_after_shadow_scope_targets_outer_binding() {
        let module = lower_source(
            "fn main() -> Int { var value = 1; if true { var value = 2; value = 3; } value = 4; return value; }",
        );
        let outer_binding = match &module.functions[0].body.instructions[0] {
            Instruction::BindMutable { binding, .. } => *binding,
            _ => panic!("outer binding"),
        };
        let post_scope_assignment = match &module.functions[0].body.instructions[2] {
            Instruction::Assign { binding, .. } => *binding,
            _ => panic!("post-scope assignment"),
        };
        let return_binding = match &module.functions[0].body.instructions[3] {
            Instruction::Return {
                value: Some(Value::Local { binding, .. }),
                ..
            } => *binding,
            _ => panic!("return local"),
        };

        assert_eq!(post_scope_assignment, outer_binding);
        assert_eq!(return_binding, outer_binding);
    }

    #[test]
    fn sibling_shadow_bindings_do_not_form_cross_scope_phi() {
        let module = lower_source(
            "fn main() -> Int { let value = 0; if true { let value = 1; let left = value; } else { let value = 2; let right = value; } return value; }",
        );
        let outer_binding = match &module.functions[0].body.instructions[0] {
            Instruction::Bind { binding, .. } => *binding,
            _ => panic!("outer binding"),
        };
        let cfg = build_cfg(&module.functions[0].body);
        let merge = cfg
            .blocks
            .iter()
            .find(|block| cfg.predecessors(block.id).len() == 2)
            .expect("branch merge");

        assert_eq!(
            cfg.entry_definitions
                .get(&merge.id)
                .and_then(|definitions| definitions.get(&outer_binding))
                .copied(),
            Some(ValueId(0))
        );
        assert!(
            merge
                .phi_nodes
                .iter()
                .all(|phi| phi.binding == outer_binding || phi.name != "value")
        );
    }

    #[test]
    fn loop_shadow_binding_does_not_create_outer_loop_carried_phi() {
        let module = lower_source(
            "fn main() -> Int { var value = 1; while false { var value = 2; value = value + 1; } return value; }",
        );
        let outer_binding = match &module.functions[0].body.instructions[0] {
            Instruction::BindMutable { binding, .. } => *binding,
            _ => panic!("outer binding"),
        };
        let cfg = build_cfg(&module.functions[0].body);

        assert!(
            cfg.blocks
                .iter()
                .flat_map(|block| &block.phi_nodes)
                .all(|phi| phi.binding != outer_binding)
        );
    }

    #[test]
    fn sibling_branch_uses_pre_branch_definition() {
        let module = lower_source(
            "fn main() -> Int { var x = 1; var y = 0; if true { x = 2; } else { y = x; } return y; }",
        );
        let cfg = build_cfg(&module.functions[0].body);
        let else_definition = cfg
            .blocks
            .iter()
            .flat_map(|block| {
                block
                    .definitions
                    .iter()
                    .map(move |definition| (block, definition))
            })
            .find(|(block, definition)| {
                matches!(
                    block.instructions.get(definition.instruction_index),
                    Some(Instruction::Assign { name, .. }) if name == "y"
                )
            })
            .map(|(_, definition)| definition)
            .expect("else branch y assignment");

        assert_eq!(else_definition.uses, vec![ValueId(0)]);
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
        assert_eq!(
            values.get(second).map(|value| value.ty),
            Some(Type::Boolean)
        );
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
