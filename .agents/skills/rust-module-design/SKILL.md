---
name: rust-module-design
description: >-
  Design or refactor Rust modules, crates, structs, and public APIs for clear
  ownership, low coupling, reuse, and easy testing. Use when implementing a
  Rust feature or deciding where types and behavior belong.
---

# Rust module design

Before editing, identify the feature's input, output, side effects, and
invariants. Keep pure domain logic separate from filesystem, process, network,
and time-dependent code.

- Give each module one cohesive responsibility. Keep orchestration thin.
- Keep fields private when invariants matter; expose constructors that return
  `Result` and small query methods instead of mutable public state.
- Define domain types for validated values rather than passing unchecked
  strings and integers through the system.
- Depend on the narrowest stable interface. Put traits at the boundary where
  the consumer needs behavior, not beside every concrete implementation.
- Prefer generics and static dispatch by default. Use trait objects only when
  runtime substitution is required.
- Keep public APIs small. Re-export only the types users must name.
- Pass borrowed data when ownership is not required; return owned data when it
  makes lifetimes and ownership clearer.
- Make invalid states unrepresentable where practical, and document `Errors`
  and `Panics` sections for public APIs that need them.

A useful structure is `domain` for types and pure rules, `ports` for small
traits, `adapters` for I/O implementations, and an application layer that
coordinates them. Do not create layers without a responsibility or an
independent test seam.

After changes, test the pure module directly and one integration path through
the public API. Run `cargo fmt`, `cargo test`, and `cargo clippy --all-targets
--all-features -- -D warnings` when the project supports them.

Sources: Rust Book Chapters 7, 9, 10, and 12; Rust API Guidelines.
