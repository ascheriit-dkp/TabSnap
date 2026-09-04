# ADR 0003: Strict versioned snapshot schema

Status: accepted

## Decision

The plaintext snapshot model is strict, browser-agnostic and versioned.

Unknown fields are rejected during import.

Browser-specific adapters convert to and from this model.

## Consequences

- imports fail early on malformed data
- format changes require a version bump or migration
- Chrome support does not leak into the core schema
- other tools can implement the format without TabSnap internals
