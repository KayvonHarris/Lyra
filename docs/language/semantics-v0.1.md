# Lyra Semantic Rules v0.1

This document defines the semantic rules currently implemented by the Lyra v0.1 semantic analyzer. It describes current compiler behavior, not the final Lyra type system.

## Pipeline

```text
source -> lexer -> parser -> AST -> semantic analysis -> Lyra IR
```

Semantic analysis must succeed before IR lowering proceeds.

## Primitive types

The analyzer models `Integer` (source `Int`), `Float`, `String`, `Boolean` (source `Bool`), `Unit`, and internal recovery type `Unknown`. Unknown type annotations produce diagnostics.

## Functions and signatures

Function declarations are collected before bodies are analyzed, allowing calls regardless of source order. Parameters and declared return types participate in checking. Untyped parameters and functions currently default to `Integer` as a v0.1 compatibility rule.

Duplicate function names and duplicate parameter names are rejected. Parameters cannot have type `Unit`. `main` cannot currently declare parameters and must return `Int`; calls to `main` are rejected.

Function calls validate the callee, argument count, and argument types against the collected signature.

## Bindings, mutability, and scopes

`let` introduces an immutable binding and `var` a mutable binding. Their types are inferred from initializers; `Unit` cannot be bound as a value.

Function bodies have independent scopes. `if` branches and `while` bodies introduce nested scopes. Duplicate declarations in one scope are rejected while inner scopes may shadow outer bindings.

Assignment requires an existing mutable binding and a value compatible with its inferred type. Assignment to an immutable binding is rejected.

## Expressions

Arithmetic operators require numeric operands. Integer-only arithmetic produces `Integer`; mixed integer/float arithmetic produces `Float`. Unary `-` requires a numeric operand and unary `!` requires `Boolean`.

Ordering comparisons require numeric operands and produce `Boolean`. Equality accepts matching primitive types and the v0.1 integer/float numeric pairing. `&&` and `||` require boolean operands.

## Return and control flow

Returns are checked against the function's declared or default return type. Returning no value produces `Unit`. Non-`Unit` functions are diagnosed when control may reach the end without returning.

An `if` is non-fallthrough when both branches are non-fallthrough; literal `while true` is also treated as non-fallthrough. Statements after proven non-fallthrough control flow are diagnosed as unreachable. `if` and `while` conditions must be `Bool`.

## Error recovery

Semantic errors are reported through `lyra-diagnostics`. Analysis attempts to continue after an error, using `Unknown` to reduce cascading diagnostics.

## Remaining v0.1 boundaries

The current semantic model does not define structs, enums, traits, generics, arrays, tuples, references, pointers, ownership/borrowing, integer width and signedness rules, compile-time constants, or user-defined aggregate types.
