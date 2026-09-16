# Execution Engine Model

Lyra's long-term execution model may combine native Lyra execution with optional interoperability engines.

```text
                    Lyra Source
                         |
                     Lyra IR
                         |
                 Execution Planning
                   /      |      \
                  /       |       \
             Native     Rust     Python
               Lyra     interop   interop
```

## Native Lyra

Native Lyra is the intended primary execution path. Ordinary Lyra programs must not require Python or a Python interpreter.

## Rust

Rust is the bootstrap implementation language and may remain useful for selected runtime, tooling, interoperability, and performance components. Rust must not define Lyra language semantics.

## Python

Python interoperability is intended to provide access to mature AI/data/scientific ecosystems where useful. Python is optional and should not become a mandatory production dependency for unrelated Lyra applications.

## Planning

A future execution planner may select execution strategies based on workload, target hardware, crossing cost, reproducibility, and explicit developer constraints. Planner decisions must be inspectable and overridable. This is research direction, not current functionality.
