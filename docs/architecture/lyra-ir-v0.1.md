# Lyra IR v0.1

Lyra IR is the compiler-owned intermediate representation between the validated Lyra AST and backend-specific representations such as LLVM IR.

## Purpose

The AST represents source syntax. Lyra IR represents compiler meaning in a form that can progressively become easier to analyze, optimize, and lower. Backend choices must not define Lyra language semantics.

The v0.1 IR is intentionally small. It establishes the compiler boundary and lowering pipeline before control flow, SSA values, explicit types, and backend lowering are introduced.

## Current model

A module contains functions. A function contains an instruction block. The v0.1 instructions are:

- `Bind` for a local binding
- `Evaluate` for an expression statement
- `Return` for function return

Values currently include integer, float, string, and boolean constants; local references; unary operations; and binary operations.

Source spans are preserved so later compiler stages can continue to produce useful diagnostics.

## Lowering contract

IR lowering is only performed after lexing, parsing, and semantic analysis succeed.

```text
source
  -> lexer
  -> parser
  -> AST
  -> semantic analysis
  -> Lyra IR
  -> future optimization/lowering
  -> LLVM IR
  -> machine code
```

The driver does not produce IR for an invalid program. This keeps backend stages from having to interpret malformed or semantically invalid source.

## Evolution toward SSA

The v0.1 IR is not SSA and is not intended to be the final low-level representation. It is the first compiler-owned lowering layer.

Expected later evolution includes:

- stable value identifiers rather than source variable names
- explicit IR types
- basic blocks
- terminators
- branches and conditional branches
- function parameters and call instructions
- typed function signatures
- phi nodes or block arguments where appropriate
- validation of IR invariants
- canonicalization and optimization passes
- a dedicated LLVM lowering layer

These changes should happen as the source language gains the corresponding semantics. We should not prematurely encode control-flow machinery that the current grammar cannot express.

## Backend independence

Lyra IR must not expose LLVM-specific concepts merely because LLVM is the first native backend. LLVM IR is an implementation target, not the definition of Lyra.

Likewise, future MLIR support for tensors, accelerators, and parallel execution can lower from suitable Lyra-owned representations without forcing ordinary Lyra programs through MLIR.

## v0.1 non-goals

This milestone does not implement SSA, optimization, LLVM code generation, MLIR, function calls, branches, loops, memory layout, ownership, borrowing, or machine-specific types.
