# Lyra Parser v0.1 Grammar

This document records the syntax intentionally supported by Parser v0.1. It is a compatibility target for the parser and its tests, not a promise that later Lyra versions will retain every surface form unchanged.

## Module and items

A source file is a sequence of items. Parser v0.1 supports zero-parameter function declarations:

```text
module      := function* EOF
function    := "fn" IDENTIFIER "(" ")" block
block       := "{" statement* "}"
```

Function parameters, return types, structs, enums, imports, and other item forms are reserved for later parser milestones.

## Statements

```text
statement   := let_statement
             | return_statement
             | expression_statement

let_statement        := "let" IDENTIFIER "=" expression ";"
return_statement     := "return" expression? ";"
expression_statement := expression ";"
```

`var` is tokenized by the lexer but is not part of Parser v0.1 syntax yet.

## Expressions

Parser v0.1 supports literals, identifiers, grouping, unary operators, and left-associative binary operators.

```text
expression   := logical_or
logical_or   := logical_and ("||" logical_and)*
logical_and  := equality ("&&" equality)*
equality     := comparison (("==" | "!=") comparison)*
comparison   := term (("<" | "<=" | ">" | ">=") term)*
term         := factor (("+" | "-") factor)*
factor       := unary (("*" | "/" | "%") unary)*
unary        := ("!" | "-") unary | primary
primary      := INTEGER | FLOAT | STRING | "true" | "false" | IDENTIFIER
              | "(" expression ")"
```

From lowest to highest precedence, the binary operator groups are `||`, `&&`, equality, comparison, addition/subtraction, and multiplication/division/remainder. Unary `!` and unary `-` bind more tightly than binary operators. Parentheses override normal precedence.

## Diagnostics and recovery

Syntax errors produce parser diagnostics with source spans. Parser v0.1 performs basic synchronization at statement and item boundaries so one malformed construct does not necessarily prevent later constructs from being parsed. Recovery behavior is best-effort and will be refined as the grammar grows.

## Non-goals for v0.1

Parser v0.1 does not perform name resolution, type checking, constant evaluation, ownership analysis, lowering to Lyra IR, or code generation. Those belong to later compiler stages.
