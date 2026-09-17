# Lyra Design Philosophy

Lyra's long-term goal is to multiply engineering capability: concise source code, strong safety, high performance, hardware awareness, and a coherent path from software intent to production execution.

## Design principles

1. **Less code, more engineering capability.** Concision must come from expressive, verifiable abstractions rather than hidden behavior.
2. **Small core, specialized loadouts.** Ordinary programs should not pay for AI, Python, GPU, networking, or other capabilities they do not use.
3. **Performance by default, control when required.** Lyra should choose strong defaults while preserving explicit control for expert users.
4. **One coherent toolchain.** Build, run, test, format, lint, benchmark, profile, audit, document, and publish should converge on the `lyra` tool.
5. **Language independence.** Lyra semantics belong to Lyra. LLVM, MLIR, Rust, Python, and other technologies are implementation infrastructure or interoperability targets, not the definition of the language.
6. **Evidence over claims.** Performance, footprint, safety, and productivity claims must be supported by reproducible benchmarks and reference projects.

These are architectural goals, not claims about capabilities already implemented.
