# RFC 0005: Package and Build System

**Status:** Draft / research direction

## Goal

The `lyra` tool should converge package management and build orchestration into a fast, secure, coherent workflow.

Potential capabilities include:

- parallel dependency resolution and builds
- incremental compilation
- content-addressed local caching
- optional remote caching
- deterministic lockfiles
- signed package metadata/artifacts
- private registries and mirrors
- dependency auditing
- native handling of Lyra packages plus defined Rust/Python interoperability metadata

The design should learn from fast modern package/build tools without copying their architecture blindly. Exact manifest, lockfile, registry, and resolver specifications remain future RFC work.
