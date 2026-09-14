# Contributing to Lyra

Lyra uses a pull-request-based workflow to keep `main` clean and reviewable.

## Local checks

Before opening a pull request, run the checks that apply to the current workspace:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Branches

Create focused branches from `main`. Suggested prefixes:

- `feat/` for new functionality
- `fix/` for bug fixes
- `docs/` for documentation
- `refactor/` for internal restructuring
- `test/` for test changes
- `chore/` for repository maintenance

## Commit messages

Use concise Conventional Commit-style messages where practical, for example:

- `feat: add lexer token definitions`
- `fix: handle unterminated string literals`
- `docs: document local development workflow`
- `ci: tighten clippy validation`

Keep commits focused so the history remains easy to review.
