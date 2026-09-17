# RFC 0004: Execution Engine Model

**Status:** Draft / research direction

## Decision

Native Lyra is the primary long-term execution model. Rust and Python are optional implementation/interoperability engines rather than mandatory runtime layers.

A future planner may choose among supported execution strategies and hardware targets when doing so provides measurable benefit.

## Requirements

Planner decisions must be deterministic/reproducible when required, inspectable, benchmarkable, and explicitly overridable. Crossing an FFI/runtime boundary must be treated as a cost rather than assumed to be beneficial.
