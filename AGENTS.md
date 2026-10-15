# Kuina agent guide

Kuina is a Rust host binary for running the `pi` coding agent in a rootless
Podman container while keeping the real OpenAI credential on the host.

Read these short project documents before changing behavior:

- [Architecture and lifecycle](docs/dev/architecture.md)
- [Configuration](docs/dev/configuration.md)
- [CLI and debug commands](docs/dev/cli.md)
- [Testing and safety rules](docs/dev/testing.md)
- [Source knowledge and prior script](docs/knowledge/rough_spec.md)

The detailed requirements, command behavior, and safety rules are in the
linked documents above; the knowledge directory contains the prior design and
shell implementation.
