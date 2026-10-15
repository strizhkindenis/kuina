---
name: rust-review-loop
description: >-
  Implement Rust changes with a short verify-and-review loop for correctness,
  safety, tests, and maintainability. Use before declaring a Rust task done.
---

# Rust implementation loop

Work in small vertical slices: define the behavior, add or update a focused
test, implement the smallest design, and verify before moving on.

1. Inspect neighboring modules, public APIs, tests, and project instructions.
2. State the invariant and failure cases before choosing types.
3. Implement pure logic first; add the boundary adapter afterward.
4. Run focused tests after each slice.
5. Review the diff for unnecessary coupling, duplicated validation, leaked
   secrets, blocking work in async code, accidental public API, and panic
   paths.
6. Run the complete project checks before reporting completion.

Required final checks when supported:

```text
cargo fmt -- --check
cargo check --all-targets --all-features
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

If a command cannot run, report the exact command and reason. Never claim a
check passed without running it. Keep the final report concise: changed
behavior, tests run, and remaining risks.
