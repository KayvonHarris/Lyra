# Lyra IR v0.1

Lyra IR is the compiler-owned intermediate representation between the validated Lyra AST and backend-specific representations such as LLVM IR.

## Purpose

The AST represents source syntax. Lyra IR represents compiler meaning in a form that is easier to validate, transform, and lower. Backend choices must not define Lyra language semantics.

The current v0.1 IR has grown beyond its original scaffold and carries the control flow and binding identity required by the supported native subset.

## Current model

A module contains typed functions with parameters and return types. Function bodies use control-flow graphs with explicit basic blocks and terminators. Bindings carry stable identities so shadowed source names remain distinct through lowering.

The IR supports local bindings, mutable assignment, expression evaluation, returns, branches, conditional branches, function parameters and calls, typed signatures, loops, and SSA-style values including phi nodes for merged or loop-carried state.

Values include primitive constants, local references, calls, unary operations, and binary operations. Source spans remain available for compiler diagnostics.

## Lowering contract

IR lowering occurs only after lexing, parsing, and semantic analysis succeed.

```text
source
  -> lexer
  -> parser
  -> AST
  -> semantic analysis
  -> Lyra IR / CFG / SSA
  -> LLVM lowering
  -> LLVM IR
  -> clang
  -> native executable
```

The driver does not produce backend IR for a program with error-severity diagnostics.

## IR invariants

Lyra IR owns control-flow validation rather than delegating Lyra semantics to LLVM. Entry blocks are explicit, blocks distinguish open from terminated control flow, and CFG validation checks structural invariants expected by later lowering. Binding identity is preserved across CFG and SSA transformations.

## Backend independence

LLVM is a backend target, not the definition of Lyra. LLVM lowering translates validated Lyra-owned representations into textual LLVM IR, which the CLI can pass to Clang for native execution.

Future MLIR support can lower from suitable Lyra-owned representations without forcing ordinary Lyra programs through MLIR.

## Remaining v0.1 boundaries

The current IR does not attempt production optimization, memory layout, ownership/borrowing, machine-specific source types, aggregates, generics, or MLIR lowering.
