# Compiler Pipeline

The intended compiler architecture separates Lyra semantics from backend infrastructure.

```text
Source
  -> Lexer
  -> Tokens
  -> Parser
  -> AST
  -> Semantic Analysis
  -> HIR
  -> Type Checking
  -> Lyra IR
  -> Lowering
  -> Backend
  -> Native Artifact
```

## Current v0.1 scaffold

Implemented scaffolding currently covers spans, diagnostics, lexer, AST, parser, driver, and CLI crates. The lexer and parser are intentionally minimal and do not yet implement the full example syntax.

## Future boundaries

Lyra IR is intended to be the compiler-owned semantic representation. Direct LLVM lowering should remain available for ordinary native code. MLIR may be introduced for domains where retaining higher-level structure enables useful transformations, especially tensors and heterogeneous accelerators.
