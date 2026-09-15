# ADR 0007: Rust for the portable Windows companion

## Status

Accepted.

## Context

Level 2 needs a Windows companion that can be copied to and run from removable storage without an installer or administrator privileges.

The companion must eventually manage encrypted snapshot files, expose a local-only browser bridge and provide a native UI. It should not require Node.js, a system service or a machine-wide runtime installation.

## Decision

Implement the portable companion as a Rust application under `apps/windows-companion`.

M16 intentionally uses only the Rust standard library. The executable derives its default storage root from `std::env::current_exe()` and creates `snapshots/` beside itself only when initialization is requested.

The Windows CI job compiles and tests the application on `windows-latest` and uploads the release-mode `.exe` as a build artifact.

## Consequences

- the core companion can be distributed as a native Windows executable
- no administrator privilege is required by the M16 architecture
- running the executable from a USB directory naturally keeps its default snapshot library on that device
- GUI and localhost-protocol dependencies can be evaluated separately in later milestones instead of being baked into the foundation
- platform-specific Windows behavior remains isolated from the browser extension and snapshot format packages
