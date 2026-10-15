# Project Specification: Rootless Podman Orchestrator & API Proxy for `pi`

## 1. Objective
Build a host-side Rust binary that manages ephemeral, rootless Podman development containers for the `pi` coding agent per project, paired with an in-process host-side HTTP reverse proxy that isolates real LLM API keys from the container environment.

`pi` runs completely unmodified inside the container without custom wrapper extensions. All isolation, secret management, and filesystem boundaries are enforced at the host-container boundary.

---

## 2. Project Configuration (`config.toml`)
The binary reads a project-level configuration file (`config.toml`) that defines:
1. **`packages`**: List of Debian (`apt`) packages required in the container image.
2. **`pi_extensions`**: List of third-party `pi` extensions to pre-install inside the container image.
3. **`mounts`**: List of additional host directories or files to bind-mount into the container, each with an explicit mode (`ro` or `rw`), source path, and target container path.
4. **`env`**:
   - Static `KEY = "VALUE"` environment variables to set inside the container.
   - Passthrough environment variable names to copy from the host environment into the container.
5. **`proxy`**:
   - Upstream LLM API base URL (e.g., `https://api.openai.com`).
   - Name of the host environment variable holding the real upstream API key (e.g., `OPENAI_API_KEY`).

---

## 3. Core Business Logic & Architectural Bounds

### 3.1 Deterministic Image Caching & Build Lifecycle
* **Pitfall**: Installing `apt` packages or `pi` extensions at container startup (`podman run`) causes severe startup latency, wastes bandwidth, and breaks offline usage.
* **Logic**:
  1. Sort and serialize the base image definition, `packages` list, and `pi_extensions` list into a deterministic `Containerfile`.
  2. The generated `Containerfile` must create an unprivileged user named `worker` with home directory `/home/worker`, install the Debian packages cleanly (`--no-install-recommends`), and install `pi` plus any configured `pi_extensions`.
  3. Compute a cryptographic hash (SHA-256 or Blake3) of the generated `Containerfile` content.
  4. Tag target image as `pi-sandbox:<hash>`.
  5. Check image existence via `podman image exists pi-sandbox:<hash>`.
  6. Execute `podman build` **only** on a cache miss (or when an explicit `--rebuild` CLI flag is passed).

### 3.2 Single-User Namespace Mapping (`--userns=keep-id`)
* **Pitfall**: `pi` executes file operations (`read`, `write`, `edit`, `grep`, `find`, `ls`) in-process using the UID of the Node/Bun runtime. Running `pi` under a separate container UID from the workspace owner breaks `--userns=keep-id`, creating files owned by unprivileged host subuids (e.g., `101001`) that corrupt the host Git working tree.
* **Logic**:
  1. Run the container with a **single user** (`worker`) mapped 1:1 to the invoking host UID/GID via `--userns=keep-id:uid=<container_worker_uid>,gid=<container_worker_gid>` (or matching default UID `1000`).
  2. Run the `pi` process directly as `worker`.
  3. Drop all Linux capabilities and prevent privilege escalation:
     - `--cap-drop=ALL`
     - `--security-opt=no-new-privileges`
  4. Do not install or rely on `sudo`, `doas`, or `setuid` binaries inside the container.
  5. Launch the container with `--rm` and inherited interactive TTY (`-it`) so `pi`'s TUI renders natively.

### 3.3 Filesystem Mounts & `pi` State Separation
* **Pitfall 1**: Relying on container user permissions to protect `pi` configurations fails if `pi` runs as `worker`. Protection must be enforced by the host kernel via read-only bind mounts (`:ro`).
* **Pitfall 2**: `pi` reads configuration from `~/.pi/agent/` (`settings.json`, `models.json`, `AGENTS.md`, `skills/`, `extensions/`), **but also writes runtime state** into `~/.pi/agent/sessions/` and `~/.pi/agent/pi-debug.log`. Mounting `~/.pi/agent` as `:ro` without redirecting sessions breaks session persistence (`--continue` and `/resume`).
* **Logic**:
  1. **Workspace Mount**: Bind-mount the host current working directory (`PWD`) as `:rw` into the container working directory (`--workdir`).
  2. **Global Config Mount (Read-Only)**: Bind-mount the host `~/.pi/agent` directory to `/home/worker/.pi/agent:ro`.
  3. **Isolated Session Mount (Read-Write)**:
     - Create a project-specific session directory on the host (e.g., `~/.local/share/pi-sandbox/<project-path-hash>/sessions`).
     - Bind-mount it to `/home/worker/.pi-sessions:rw`.
     - Set `PI_CODING_AGENT_SESSION_DIR=/home/worker/.pi-sessions` inside the container environment.
  4. **Custom Mounts**: Validate and canonicalize all `mounts` from `config.toml` and attach them with explicit `:ro` or `:rw` flags. Fail fast if a required source path does not exist on the host.

### 3.4 Environment Variable Isolation
* **Pitfall**: Passing `--env-host` leaks host secrets into the container, while starting with a completely empty environment (`env -i` equivalent) breaks compilers, linkers, and TUI rendering.
* **Logic**:
  1. Never pass the full host environment to `podman run`.
  2. Preserve the container image's baseline runtime environment (`PATH`, `HOME=/home/worker`, `USER=worker`, `LANG`, `LC_ALL`, `TMPDIR`) and forward the host `TERM` and `COLORTERM` variables for TUI compatibility.
  3. Inject only the explicit `env` keys configured in `config.toml`.
  4. **Never** pass the real upstream LLM API key into the container environment.

### 3.5 Host-Side LLM API Reverse Proxy
* **Pitfall 1**: Exposing an unauthenticated localhost proxy on the host allows any other local unprivileged process or container to hijack the user's LLM API quota while `pi` is running.
* **Pitfall 2**: Response buffering or Nagle's algorithm on the proxy socket stalls Server-Sent Events (`text/event-stream`), causing `pi`'s TUI output to freeze or stutter.
* **Logic**:
  1. **Ephemeral Listener**: Before spawning `podman run`, bind an async HTTP server on host `127.0.0.1:0` (OS-assigned ephemeral port).
  2. **Ephemeral Sandbox Token**: Generate a cryptographically random per-session token (`sk-sandbox-<32-byte-hex>`).
  3. **Container Network Wiring**:
     - Pass `--add-host=host.containers.internal:host-gateway` to `podman run`.
     - Pass `OPENAI_BASE_URL=http://host.containers.internal:<ephemeral_port>/v1` (or the configured provider path prefix) into the container environment.
     - Pass `OPENAI_API_KEY=sk-sandbox-<32-byte-hex>` into the container environment.
  4. **Authentication & Key Swap**:
     - For every incoming HTTP request to the proxy, validate that the `Authorization` header matches `Bearer sk-sandbox-<32-byte-hex>`. Return `401 Unauthorized` immediately on mismatch.
     - Replace the `Authorization` header with `Bearer <real_host_api_key>` read from the host's memory.
     - Forward method, path, query, safe headers, and body to the configured upstream base URL.
  5. **Low-Latency Streaming**:
     - Enable `TCP_NODELAY` on the local listener and upstream client sockets.
     - Stream request and response bodies chunk-by-chunk without full-body buffering so `text/event-stream` SSE chunks flush immediately to the container.
  6. **Lifecycle Coupling**:
     - Run the proxy task concurrently with the `podman run` child process.
     - When `podman run` exits, shut down the proxy listener and exit the Rust binary with the container's exit code.