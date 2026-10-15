# CLI and debug commands

The command-line interface should make normal lifecycle actions explicit:

- `kuina start`: read `config.toml`, build or reuse the image, start the proxy,
  and run the Pi container.
- `kuina restart`: stop the managed run, then perform the complete start flow.
- `kuina status`: report whether the managed proxy is stopped, starting, or
  running, without exposing the upstream key.

Start accepts a rebuild option and additional validated Podman options. The
normal command uses rootless Podman, `--rm`, `-it`, `--userns=keep-id`, an
explicit worker user, `--cap-drop=ALL`, `--security-opt=no-new-privileges`,
`--pull=never`, the workspace mount, and the configured mounts.

Debug commands must not start a container or reveal secrets:

- `kuina debug podman-command` prints the fully assembled, copyable Podman
  build/run commands with secrets redacted.
- `kuina debug containerfile` prints the generated Containerfile.

Debug output should be deterministic where possible so it can be copied and
compared with direct Podman runs.
