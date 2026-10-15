---
name: rust-testing-boundaries
description: >-
  Build fast, deterministic, maintainable tests for Rust code using pure
  functions, focused traits, fakes, and boundary integration tests. Use when
  adding behavior, refactoring code, or preparing a Rust change for review.
---

# Rust testing boundaries

Design the seam before writing the implementation. Make transformations and
validation pure; isolate each external effect behind a small trait or a
concrete boundary function.

- Unit-test each public behavior and important invariant, not private lines.
- Use an in-memory fake or recording fake for ports; avoid real network,
  process, clock, filesystem, and sleeps in unit tests.
- Keep fakes minimal and behavior-focused. Do not duplicate the production
  implementation in a fake.
- Test success, malformed input, missing data, boundary values, and every
  expected external failure. Assert useful error variants or messages without
  asserting incidental formatting.
- Prefer deterministic test data and explicit dependency injection through
  function parameters or constructors.
- Put cross-module wiring and real adapters in integration tests. Keep those
  tests few and focused.
- Use `#[cfg(test)]` helpers only for test support; do not weaken production
  invariants to make tests convenient.
- When a bug is found, first add a regression test at the lowest boundary that
  reproduces it.

Before finishing, run `cargo test`, then the relevant integration tests and
`cargo clippy --all-targets --all-features -- -D warnings`. Check that tests
run without network access, ambient environment variables, or order
dependencies.

Sources: The Rust Book, Chapter 11; Rust API Guidelines.
