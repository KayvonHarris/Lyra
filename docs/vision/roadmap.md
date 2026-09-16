# Lyra Roadmap

This roadmap records direction rather than a release guarantee.

## v0.1 — Core language
Frontend, diagnostics, semantic analysis, types, Lyra IR, LLVM code generation, and native execution for a deliberately small language subset.

## v0.2 — Systems foundation
Memory-safety model, generics, traits/interfaces, pattern matching, concurrency, FFI/ABI, and `no_std` foundations.

## v0.3 — Unified toolchain
Build/run/test/fmt/lint/bench/profile/audit/doc commands, package manifests and lockfiles, incremental builds, parallel execution, and content-addressed caching.

## v0.4 — Interoperability engines
Defined Rust and Python boundaries, cost-aware crossings, shared-buffer strategies where safe, and explicit engine controls.

## v0.5 — AI foundation
Tensor and shape-aware semantics, model execution, accelerator abstractions, and MLIR integration where multi-level lowering provides measurable value.

## v0.6 — Embedded and edge
ARM/RISC-V targets, constrained runtimes, hardware abstractions, sensors, realtime primitives, and edge deployment.

## v0.7 — Execution planning
Reproducible, inspectable workload planning across native Lyra, interoperability engines, and available hardware.

## v0.8 — Robotics and automotive
Vision, robotics, automotive/sensor-fusion libraries, simulation, and substantial edge reference workloads.

## v0.9 — Self-hosting
Progressively rewrite suitable compiler/toolchain components in Lyra until Lyra can compile increasingly large portions of itself.

## v1.0 — Production baseline
Stable documented language subset, compatibility policy, mature diagnostics/tooling, reproducible builds, security processes, benchmarks, and production-grade reference applications.
