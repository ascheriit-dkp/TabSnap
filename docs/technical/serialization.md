# Serialization and compression

The v1 pipeline starts like this:

```text
validated snapshot
      |
      v
UTF-8 JSON
      |
      v
gzip
```

Encryption comes after gzip.

## Serialization

The snapshot is validated against the v1 schema before serialization and again after deserialization.

The encoded JSON is UTF-8.

Invalid UTF-8 fails.

## Compression

Compression uses gzip through the platform `CompressionStream` / `DecompressionStream` APIs.

No remote library is needed at runtime.

Decoded payloads are capped at 64 MiB. Decompression stops once that limit is crossed.

The compressed byte stream is an internal stage. The final encrypted envelope will record the compression algorithm explicitly.
