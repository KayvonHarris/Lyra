# RFC 0002: Compiler Architecture

**Status:** Draft

## Decision

Lyra uses a staged compiler architecture with explicit semantic boundaries.

```text
Source -> Lexer -> Parser -> AST -> Semantics/HIR -> Types -> Lyra IR -> Lowering -> Backend
```

The compiler should produce diagnostics with precise source spans throughout the pipeline.

## Rationale

Separating parsing, semantic analysis, compiler-owned IR, and backend lowering prevents backend technologies from becoming the language specification and makes testing individual stages practical.
