# LLVM and MLIR in Lyra

LLVM and MLIR serve different potential roles.

## LLVM

LLVM is the planned low-level native-code backend for early Lyra. A simple path should remain possible:

```text
Lyra -> Lyra IR -> LLVM IR -> machine code
```

LLVM provides mature low-level optimization and code generation for targets such as x86-64, ARM, and RISC-V.

## MLIR

MLIR is a future multi-level lowering and optimization layer, especially useful when Lyra must preserve domain information such as tensor shapes, parallel operations, or accelerator intent.

A specialized path may become:

```text
Lyra -> Lyra IR -> MLIR -> lower-level representation -> backend
```

Not every program should be forced through MLIR.

## Architectural rule

Neither LLVM IR nor MLIR defines Lyra semantics. Lyra IR remains owned by the Lyra compiler so backend infrastructure can evolve without redefining the language.
