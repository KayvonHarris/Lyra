# RFC 0003: Lyra IR

**Status:** Draft / research direction

## Decision

Lyra will develop a compiler-owned intermediate representation between typed language semantics and backend-specific lowering.

Lyra IR should eventually retain information needed for Lyra-specific optimization and verification, potentially including ownership/effects, types, tensor shapes, concurrency intent, and target constraints.

## Non-decision

The exact IR structure, SSA strategy, dialect model, serialization format, and optimization pipeline are not yet specified. MLIR may be used beneath or alongside Lyra IR where appropriate, but does not replace ownership of Lyra semantics.
