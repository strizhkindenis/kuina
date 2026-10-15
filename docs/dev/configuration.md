# Configuration

The command reads a project-level `config.toml`. Configuration is validated
before any Podman process starts.

Supported sections:

- `packages`: Debian packages installed into the generated image.
- `pi_extensions`: third-party Pi extensions installed into the image.
- `mounts`: host source and container target paths with explicit `ro` or `rw`
  mode. Sources are canonicalized and must exist.
- `env`: static key/value entries and named host variables to pass through.
- `proxy`: upstream LLM base URL and the host environment variable containing
  its real API key.

Only explicit environment values are passed to the container, alongside the
minimal runtime environment and terminal variables. The upstream secret is
kept on the host. Package and extension lists must be sorted before generating
the Containerfile so image hashes are reproducible.

CLI-provided extra Podman options are appended only after validation and must
not silently override security, mounts, image, entrypoint, or proxy wiring.
