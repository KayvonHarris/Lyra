# RFC 0001: Design Principles

**Status:** Draft

## Decision

Lyra will optimize for engineering capability per unit of developer complexity: expressive source, strong static guarantees, high performance, small default footprint, and a coherent toolchain.

The core language should remain small. Specialized domains such as AI, robotics, automotive, cloud, and embedded computing should grow through well-defined libraries, compiler capabilities, and optional runtime components rather than indiscriminately expanding the core.

## Consequences

- unused capabilities should not impose unnecessary runtime/deployment cost;
- expert control must remain available beneath high-level abstractions;
- significant performance claims require reproducible evidence;
- language semantics remain independent of implementation dependencies.
