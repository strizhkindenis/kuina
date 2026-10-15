# CLI and debug commands

The command-line interface is organized around diagnostics, proxy lifecycle,
and container entrypoints.

## Diagnostics

- `kuina doctor` reports the current operating system, whether it is
  supported, Podman availability and version, and other required runtime
  prerequisites. Linux and macOS/Darwin are supported; Windows is detected but
  not supported.
- `kuina version` prints the program version and the Git commit hash used to
  build the binary.

## Proxy lifecycle

- `kuina proxy start` starts the host-side OpenAI proxy.
- `kuina proxy status` reports whether the proxy is stopped, starting, or
  running without exposing credentials.
- `kuina proxy stop` stops the managed proxy.
- `kuina proxy restart` stops and starts the managed proxy.

## Container entrypoints

- `kuina run` reads `config.toml`, builds or reuses the image, starts the proxy
  as needed, and runs `pi` in the configured container.
- `kuina sh` starts an interactive Bash shell in the same configured container
  environment.

Both container commands accept a rebuild option and additional validated Podman
options. They use rootless Podman, `--rm`, `-it`, `--userns=keep-id`, an
explicit worker user, `--cap-drop=ALL`, `--security-opt=no-new-privileges`,
`--pull=never`, the workspace mount, and configured mounts. The real upstream
API key remains on the host.

Debug commands must not start a container or reveal secrets:

- `kuina debug podman-command` prints the fully assembled, copyable Podman
  build/run commands with secrets redacted.
- `kuina debug containerfile` prints the generated Containerfile.

Debug output should be deterministic where possible so it can be copied and
compared with direct Podman runs.
