# Self-Hosting Strategy

Lyra currently uses Rust as its bootstrap implementation language.

Self-hosting is a long-term milestone, not a prerequisite for useful Lyra releases.

The intended progression is:

1. establish a correct Rust-hosted compiler and language specification;
2. stabilize Lyra semantics and Lyra IR;
3. make Lyra expressive enough to implement compiler/tooling components;
4. rewrite suitable components incrementally;
5. bootstrap and verify Lyra-built compiler artifacts;
6. preserve reproducible bootstrap paths and cross-version testing.

Self-hosting should not require abandoning LLVM, MLIR, operating-system toolchains, or interoperability infrastructure. It means Lyra becomes capable of implementing and compiling substantial portions of its own compiler/toolchain.
