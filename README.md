# Lyra

> A modern systems programming language focused on readability, performance, safety, and developer experience.

[![CI](https://github.com/KayvonHarris/Lyra/actions/workflows/ci.yml/badge.svg)](https://github.com/KayvonHarris/Lyra/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-early%20development-orange.svg)](#project-status)

## Overview

Lyra is an independent programming-language engineering project by **Kayvon Harris**. The project is being developed as a modern systems language with an emphasis on readable code, strong engineering foundations, performance, safety, and a productive developer experience.

The repository is currently in **early development**. The initial Rust compiler workspace and frontend scaffolding are now being established through a consistent, tested, reviewable workflow.

## Project Status

**Early development / repository foundation**

The clean Lyra repository began with engineering infrastructure and now contains the first v0.1 compiler/frontend scaffold. The lexer and parser remain intentionally minimal; planned capabilities should not be considered implemented, stable, or production-ready.

The previous experimental implementation is preserved separately in the [Lyra-Legacy](https://github.com/KayvonHarris/Lyra-Legacy) repository for historical reference.

## Engineering Workflow

Changes to Lyra are developed on focused branches and submitted to `main` through pull requests.

Every pull request is validated by GitHub Actions. As Rust packages are added, CI is configured to run:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --release
```

The `main` branch is protected so development follows a reviewable workflow:

```text
feature branch
      ↓
pull request
      ↓
automated CI
      ↓
validation
      ↓
squash merge
      ↓
main
```

## Repository Structure

The current foundation includes the repository engineering layer plus the first deliberately small compiler workspace:

```text
Lyra/
├── .github/
├── crates/
│   ├── lyra-span/
│   ├── lyra-diagnostics/
│   ├── lyra-lexer/
│   ├── lyra-ast/
│   ├── lyra-parser/
│   └── lyra-driver/
├── tools/
│   └── lyra/
├── docs/
│   ├── architecture/
│   ├── rfcs/
│   └── vision/
├── examples/
├── tests/
├── benchmarks/
├── Cargo.toml
└── README.md
```

Scaffolding is added only when it establishes a real architectural boundary or near-term implementation target.

## Development Environment

### Prerequisites

- Git
- Rust via `rustup`

The repository includes `rust-toolchain.toml`, which keeps contributors on the expected Rust toolchain and installs the required `rustfmt` and `clippy` components.

### Clone

```bash
git clone https://github.com/KayvonHarris/Lyra.git
cd Lyra
```

### Validate

Once Rust workspace packages are present, the standard local validation sequence is:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --release
```

## Repository Standards

Lyra uses:

- protected `main` branch
- pull-request-based development
- automated GitHub Actions validation
- Rust formatting and Clippy enforcement
- automated testing as implementation is added
- Dependabot for Cargo and GitHub Actions dependencies
- focused Conventional Commit-style messages
- squash merging to maintain a readable `main` history

See [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow.

## Roadmap

The evolving roadmap is documented in `docs/vision/roadmap.md`. Architecture notes and draft RFCs preserve design direction for Lyra IR, LLVM/MLIR, execution engines, self-hosting, and the package/build system.

These documents deliberately distinguish implemented scaffolding from planned or research-stage capabilities.

## License

Lyra is licensed under the [Apache License 2.0](LICENSE).

## Author

**Kayvon Harris**

Software Engineering · AI/ML · Embedded Systems

[GitHub Profile](https://github.com/KayvonHarris)

---

**Note:** Lyra is an active learning and engineering project. APIs, architecture, syntax, and implementation details may change substantially during early development.
