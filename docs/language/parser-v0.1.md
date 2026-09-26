# Lyra Parser v0.1 Grammar

This document records the syntax currently supported by Parser v0.1. It is a compatibility target for the parser and its tests, not a promise that later Lyra versions will retain every surface form unchanged.

## Module and items

A source file is a sequence of function declarations. Functions may declare parameters with optional type annotations and an optional return type.

```text
module       := function* EOF
function     := "fn" IDENTIFIER "(" parameters? ")" ("->" type_name)? block
parameters   := parameter ("," parameter)*
parameter    := IDENTIFIER (":" type_name)?
type_name    := IDENTIFIER
block        := "{" statement* "}"
```

Structs, enums, imports, and other item forms are outside the current parser subset.

## Statements

```text
statement   := let_statement
             | var_statement
             | assignment_statement
             | return_statement
             | if_statement
             | while_statement
             | expression_statement

let_statement        := "let" IDENTIFIER "=" expression ";"
var_statement        := "var" IDENTIFIER "=" expression ";"
assignment_statement := IDENTIFIER "=" expression ";"
return_statement     := "return" expression? ";"
if_statement         := "if" expression block ("else" block)?
while_statement      := "while" expression block
expression_statement := expression ";"
```

`let` introduces an immutable binding. `var` introduces a mutable binding; assignment syntax is parsed separately and semantic analysis determines whether the target may be assigned.

## Expressions

Parser v0.1 supports literals, identifiers, function calls, grouping, unary operators, and left-associative binary operators.

```text
expression   := logical_or
logical_or   := logical_and ("||" logical_and)*
logical_and  := equality ("&&" equality)*
equality     := comparison (("==" | "!=") comparison)*
comparison   := term (("<" | "<=" | ">" | ">=") term)*
term         := factor (("+" | "-") factor)*
factor       := unary (("*" | "/" | "%") unary)*
unary        := ("!" | "-") unary | primary
primary      := INTEGER | FLOAT | STRING | "true" | "false"
              | IDENTIFIER ("(" arguments? ")")?
              | "(" expression ")"
arguments    := expression ("," expression)*
```

From lowest to highest precedence, the binary operator groups are `||`, `&&`, equality, comparison, addition/subtraction, and multiplication/division/remainder. Unary `!` and unary `-` bind more tightly than binary operators. Parentheses override normal precedence.

## Diagnostics and recovery

Syntax errors produce parser diagnostics with source spans. Parser v0.1 performs basic synchronization at statement and item boundaries so one malformed construct does not necessarily prevent later constructs from being parsed. Recovery behavior is best-effort and will be refined as the grammar grows.

## Parser boundaries

The parser recognizes syntax only. Name resolution, type checking, mutability enforcement, control-flow validation, Lyra IR lowering, and code generation belong to later compiler stages.
