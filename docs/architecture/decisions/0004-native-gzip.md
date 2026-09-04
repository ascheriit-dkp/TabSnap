# ADR 0004: Native gzip compression

Status: accepted

## Decision

Use gzip through `CompressionStream` and `DecompressionStream`.

## Why

Chrome, modern Firefox and Node support the API. It avoids shipping a compression library in the extension.

## Safety

Decompression is streamed and capped at 64 MiB before JSON parsing.

## Consequences

The portable app must implement compatible gzip handling. That is not a problem.
