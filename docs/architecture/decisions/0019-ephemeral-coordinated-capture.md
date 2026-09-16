# ADR 0019 — Ephemeral coordinated capture

## Status

Accepted for Level 4 M30.

## Context

M29 gives the Windows companion an authenticated, ephemeral registry of explicitly connected Chrome, Edge and Firefox extension instances. M30 needs the companion to ask those live instances for a workspace capture and collect the results without turning the companion into a trusted plaintext processor.

The existing TabSnap encryption boundary remains mandatory: snapshot passwords and decrypted browser workspaces stay inside extensions. Persistence of a multi-browser machine snapshot is deferred to M31.

## Decision

The companion owns **ephemeral in-memory capture jobs**. Browser extensions cannot create jobs over loopback; capture creation is available only to companion-side code.

A job snapshots the currently active M29 registry entries that advertise the `capture` capability. Each target is addressed by its random browser instance identifier and remains bound to the extension origin that registered it.

Connected extensions poll an authenticated capture endpoint while the TabSnap page stays explicitly connected. A claimed target has a 15-second claim lease so an interrupted attempt can be offered again. Unfinished targets fail after 60 seconds. Completed jobs remain in memory for at most five minutes. The store is capped at eight jobs, 32 targets per job, 65 MiB per encrypted browser result and 256 MiB of encrypted result data per job.

When an extension receives a capture assignment, it:

1. reads the password already present in that local TabSnap page;
2. captures its own browser workspace through the existing browser adapter;
3. encrypts the snapshot locally with the existing TabSnap envelope;
4. submits only the opaque encrypted bytes to the companion.

The password is never included in the capture protocol and is never persisted by M30. If the page has no valid local password, or capture/encryption fails, the extension submits only a bounded failure code. Raw exception text, URLs, tab titles and plaintext browser state are not sent as failure details.

Submission failures keep at most one bounded encrypted result or failure outcome in the extension page's memory for retry. Disconnecting companion mode clears that pending outcome.

The companion exposes per-target pending, claimed, complete or failed state so partial success is explicit. Encrypted results are retained only in the in-memory M30 job store. M31 will define the versioned machine snapshot container and atomic persistence of those opaque results.

A minimal `tabsnap-companion capture` CLI command starts the protocol, lets the user explicitly connect the browser pages to include, creates the job after confirmation and reports per-browser progress. This is diagnostic control for M30; M33 owns the final whole-machine GUI and retry UX.

## Consequences

- The companion coordinates multiple browsers without receiving a password, encryption key or decrypted workspace.
- Only explicitly connected, currently live and capture-capable browser instances can participate.
- Browser-origin binding from M29 also protects capture polling and result submission.
- A browser that disappears or stalls becomes a bounded partial failure instead of blocking a job indefinitely.
- M30 results are intentionally non-durable; closing the companion before M31 persistence loses the in-memory job data.
- Final one-action password/progress UX is not defined here and remains M33 work.