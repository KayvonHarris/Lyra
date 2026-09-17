# Lyra Integration Tests

As the compiler becomes executable, integration coverage will be organized into:

- `compile-pass/` for valid Lyra programs
- `compile-fail/` for diagnostics and invalid programs
- `runtime/` for executable behavior

Unit tests remain next to the Rust crates that implement each compiler stage.
