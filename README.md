# Lyra

> A modern systems programming language focused on readability, performance, safety, and developer experience.

[![CI](https://github.com/KayvonHarris/Lyra/actions/workflows/ci.yml/badge.svg)](https://github.com/KayvonHarris/Lyra/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-early%20development-orange.svg)](#project-status)

## Overview

Lyra is an independent programming-language engineering project by **Kayvon Harris**. The project is being developed as a modern systems language with an emphasis on readable code, strong engineering foundations, performance, safety, and a productive developer experience.

The repository is currently in **early development**. The infrastructure is being established first so future compiler and language work can be developed through a consistent, tested, reviewable workflow.

## Project Status

**Early development / repository foundation**

The clean Lyra repository is intentionally starting with its engineering infrastructure before implementation is migrated or rebuilt. Features should not be considered stable or production-ready at this stage.

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

The repository will grow as implementation work is introduced. The current foundation includes:

```text
Lyra/
├── .github/
│   ├── workflows/
│   │   └── ci.yml
│   ├── dependabot.yml
│   └── pull_request_template.md
├── .gitignore
├── Cargo.toml
├── CONTRIBUTING.md
├── LICENSE
├── README.md
└── rust-toolchain.toml
```

Future source-code directories will be added deliberately as the implementation develops rather than being pre-populated with unused scaffolding.

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

Detailed language and compiler milestones will be documented separately as the implementation plan is finalized. This README intentionally avoids presenting planned capabilities as completed features.

## License

Lyra is licensed under the [Apache License 2.0](LICENSE).

## Author

**Kayvon Harris**

Software Engineering · AI/ML · Embedded Systems

[GitHub Profile](https://github.com/KayvonHarris)

---

**Note:** Lyra is an active learning and engineering project. APIs, architecture, syntax, and implementation details may change substantially during early development.
