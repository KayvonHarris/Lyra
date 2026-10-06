# Semantic type ownership (v0.1)

## Contract

Semantic analysis is the sole authority for source-level expression types, function
signatures, lexical binding types, and numeric promotion. Lyra IR must consume
validated semantic results; it must not independently apply language typing
rules to an AST. LLVM consumes explicit types and conversions in Lyra IR and
must not infer source-language types.

## Current gap

The current `Analysis` exposes diagnostics only. IR lowering separately
reconstructs function return types, local expression types, and mixed numeric
promotion from the AST. LLVM also reconstructs some operand types from IR
shapes. These duplicated decisions can diverge as the language grows.

## Target representation

Semantic analysis should produce a typed semantic module in addition to
diagnostics. It must retain the validated lexical binding identity, declared
function signatures, and the type of every expression, including nested
expressions and call arguments. It must encode implicit numeric conversions
explicitly, preserving their source spans.

Prefer typed expression nodes with owned children over a map keyed solely by
source span: synthetic/test ASTs can reuse spans, and spans describe locations,
not stable expression identities. The typed representation should be created
during the same traversal that validates expressions, not by a second type
inference pass.

## Migration sequence

1. Introduce typed semantic expression/statement/function structures and
   populate them during validation, preserving existing diagnostics and
   lexical scope behavior.
2. Change driver orchestration to pass the validated typed module to IR
   lowering. Eliminate IR's AST-based `expression_type` and duplicate
   numeric-promotion rules. Keep stable `BindingId` identity across CFG/SSA.
3. Make LLVM consume explicit IR operand types rather than reconstructing
   source typing rules. Preserve explicit Int-to-Float conversions.
4. Add regression coverage for nested expressions, shadowed bindings, mixed
   numeric operations, calls, and mutable values across phi nodes.

## Acceptance criteria

The existing native integer and Float programs remain green; invalid programs
never reach IR lowering; one authoritative semantic decision determines each
expression's type; and no backend silently guesses an implicit conversion.
