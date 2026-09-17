# Lyra Semantic Rules v0.1

This document defines the semantic rules implemented by the Lyra v0.1 semantic analyzer. It describes the current compiler behavior, not the final Lyra type system.

## Pipeline

The v0.1 front end processes source code as:

```text
source -> lexer -> parser -> AST -> semantic analysis -> diagnostics
```

Semantic analysis runs after parsing and before future lowering into Lyra IR.

## Primitive types

The semantic analyzer currently models:

- `Integer`
- `Float`
- `String`
- `Boolean`
- `Unit`
- `Unknown`

`Unknown` is an internal recovery type. It allows analysis to continue after an earlier semantic error without generating unnecessary follow-on diagnostics.

## Variable bindings

A `let` statement introduces a variable into the current function scope. Its type is inferred from the initializer expression.

```lyra
let speed = 65;      // Integer
let ratio = 1.5;     // Float
let active = true;   // Boolean
```

A variable cannot be declared twice in the same scope. References to names that are not defined in an active scope produce an `unknown identifier` diagnostic.

Each function receives an independent scope. Variables declared in one function are not visible in another function.

## Numeric operations

`+`, `-`, `*`, `/`, and `%` currently require numeric operands.

Integer operations produce `Integer`. If either operand is a `Float`, the result is `Float`.

This numeric promotion rule is an initial v0.1 rule and may become more explicit as Lyra gains concrete integer widths, conversions, and overflow semantics.

## Unary operations

Unary `-` requires an `Integer` or `Float` operand and preserves its numeric type.

Unary `!` requires a `Boolean` operand and produces `Boolean`.

## Comparisons

`<`, `<=`, `>`, and `>=` require numeric operands and produce `Boolean`.

## Equality

`==` and `!=` accept operands of the same primitive type. Integer and float operands are also considered compatible in v0.1. Equality produces `Boolean`.

## Logical operations

`&&` and `||` require boolean operands and produce `Boolean`.

## Return statements

The analyzer validates expressions inside `return` statements. Function return-type declarations and consistency checking are not yet part of the v0.1 grammar, so return types are not currently enforced.

## Error recovery

Semantic errors are reported through `lyra-diagnostics`. The analyzer attempts to continue after an error. Expressions depending on an unresolved or invalid expression use the internal `Unknown` type to reduce cascading diagnostics.

## Not yet implemented

The following are intentionally outside this semantic milestone:

- explicit type annotations
- function parameters
- function calls
- declared function return types
- nested block scopes in the grammar
- assignment and mutability rules
- structs, enums, traits, or generics
- arrays, tuples, references, or pointers
- ownership and borrowing
- integer width and signedness rules
- compile-time constants
- Lyra IR lowering

These features should be introduced through later grammar, semantic, and IR milestones rather than being assumed by v0.1.
