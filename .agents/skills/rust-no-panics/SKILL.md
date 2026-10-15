---
name: rust-no-panics
description: >-
  Prevent accidental panics and improve recoverable error handling in Rust
  production code. Use when implementing, reviewing, or hardening Rust code
  that handles external input, I/O, concurrency, or process boundaries.
---

# Rust no-panics policy

Treat external input and operational failure as normal control flow. Return a
meaningful typed error and let the caller decide whether to retry, report, or
exit.

- Do not use `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, or
  indexing in production paths.
- Prefer `?`, `ok_or_else`, `map_err`, `if let`, and exhaustive `match`.
- Replace indexing with checked access such as `get` and handle `Option`.
- Validate configuration, paths, protocol data, and numeric ranges before
  starting side effects.
- Use a dedicated error enum for domain failures; preserve source errors with
  `#[source]` or `thiserror` when available. Add context at boundaries.
- Treat mutex poisoning, channel closure, task cancellation, parse failures,
  missing environment variables, and non-success process or HTTP responses as
  explicit errors.
- Do not hide errors with discarded `Result`s. If an error is intentionally
  ignored, explain the decision in the surrounding code and use an explicit
  operation such as `is_ok` when appropriate.
- Keep `main` and CLI entry points responsible for displaying errors and exit
  codes. Library code should not terminate the process.
- `unsafe` is exceptional: minimize it, isolate it, and document its safety
  invariants. Never use it to bypass ordinary error handling.

Search changed production code for forbidden calls and indexing. Then run
`cargo fmt`, `cargo check`, `cargo test`, and
`cargo clippy --all-targets --all-features -- -D warnings`.

A panic may be acceptable in a test, example, or a proven internal invariant,
but document public panic behavior. Returning `Result` is the default for
recoverable failure.

Sources: The Rust Book, Chapters 9 and 12; Rust API Guidelines.
