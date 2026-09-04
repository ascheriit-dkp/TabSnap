# ADR 0002: Browser adapters

Status: accepted

## Decision

Browser-specific capture and restore logic stays behind adapters.

The snapshot schema, validation, compression and crypto stay browser-agnostic.

## Consequences

Chrome ships first without making the format Chrome-specific.
