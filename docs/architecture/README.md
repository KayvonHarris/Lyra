# Lyra Architecture

Lyra v0.1 begins with a deliberately small compiler frontend:

```text
Source -> Lexer -> Tokens -> Parser -> AST -> Driver
```

Planned later stages include semantic analysis, HIR, the type system, Lyra IR, LLVM code generation, the runtime, interoperability engines, and specialized MLIR lowering where it provides value.

The compiler architecture should keep Lyra language semantics independent from LLVM, MLIR, Rust, and Python.
