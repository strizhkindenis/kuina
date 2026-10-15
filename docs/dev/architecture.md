# Architecture

Kuina is a host-side Rust binary. It orchestrates a short-lived, rootless
Podman container running an unmodified `pi` agent for the current project.

The lifecycle is:

1. Read and validate the project configuration.
2. Generate a deterministic Containerfile from the base image, Debian
   packages, and `pi` extensions.
3. Hash that file and reuse `pi-sandbox:<hash>` unless a rebuild is requested.
4. Start a host-only HTTP proxy on an ephemeral loopback port.
5. Give the container only a per-run proxy token, never the real upstream key.
6. Run `pi` with `--userns=keep-id`, explicit mounts, dropped capabilities,
   and an interactive TTY.
7. Stop the proxy when the container exits and return its exit status.

The proxy authenticates the sandbox token, replaces it with the host-held API
key, forwards safe headers and streaming bodies, and supports SSE without full
response buffering. Workspace mounts are read-write; global Pi configuration
is read-only and sessions use a project-specific writable directory.

The older shell implementation is [pi_run.zsh](../knowledge/pi_run.zsh), and
the detailed requirements are in [rough_spec.md](../knowledge/rough_spec.md).
