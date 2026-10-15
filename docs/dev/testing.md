# Testing and safety

Every component must have at least one automated test. At minimum, cover
configuration parsing and validation, deterministic Containerfile generation,
image hashing, mount and environment isolation, Podman command construction,
CLI parsing, proxy token authentication and API-key replacement, streaming
forwarding, lifecycle/status transitions, and debug redaction.

Prefer unit tests around pure builders and small integration tests around
process and HTTP boundaries. Tests must verify malformed configuration,
missing paths, missing API keys, invalid options, failed Podman commands, and
non-success proxy responses produce useful errors rather than panics.

Rust implementation rules:

- Never use `unwrap`, `expect`, indexing that can panic, or explicit panic
  paths in production code.
- Handle every error with a meaningful typed or displayed error.
- Validate external input before spawning processes or opening mounts.
- Never log or print the real upstream API key or proxy secret.
- Run formatting, compilation, tests, and Clippy before committing.
