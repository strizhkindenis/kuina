use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KuinaError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("podman exited with status {0}")]
    Podman(i32),
    #[error("podman failed to start: {0}")]
    PodmanStart(String),
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub pi_extensions: Vec<String>,
    #[serde(default)]
    pub mounts: Vec<MountConfig>,
    #[serde(default)]
    pub env: EnvConfig,
    pub proxy: ProxyConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MountConfig {
    pub source: PathBuf,
    pub target: PathBuf,
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "ro".to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnvConfig {
    #[serde(default)]
    pub static_values: BTreeMap<String, String>,
    #[serde(default)]
    pub passthrough: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProxyConfig {
    pub base_url: String,
    pub api_key_env: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, KuinaError> {
        let text = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&text).map_err(|e| KuinaError::Config(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), KuinaError> {
        if self.proxy.base_url.is_empty() || self.proxy.api_key_env.is_empty() {
            return Err(KuinaError::Config(
                "proxy.base_url and proxy.api_key_env are required".into(),
            ));
        }
        for package in &self.packages {
            if package.trim().is_empty() || package.chars().any(char::is_whitespace) {
                return Err(KuinaError::Config(format!("invalid package: {package}")));
            }
        }
        for extension in &self.pi_extensions {
            if extension.trim().is_empty() || extension.chars().any(char::is_whitespace) {
                return Err(KuinaError::Config(format!(
                    "invalid extension: {extension}"
                )));
            }
        }
        for mount in &self.mounts {
            if mount.mode != "ro" && mount.mode != "rw" {
                return Err(KuinaError::Config(format!(
                    "invalid mount mode: {}",
                    mount.mode
                )));
            }
            if !mount.source.exists() {
                return Err(KuinaError::Config(format!(
                    "mount source does not exist: {}",
                    mount.source.display()
                )));
            }
            if !mount.target.is_absolute() {
                return Err(KuinaError::Config(format!(
                    "mount target is not absolute: {}",
                    mount.target.display()
                )));
            }
        }
        for key in self
            .env
            .static_values
            .keys()
            .chain(self.env.passthrough.iter())
        {
            if key.is_empty() || key.contains('=') {
                return Err(KuinaError::Config(format!(
                    "invalid environment name: {key}"
                )));
            }
        }
        Ok(())
    }

    pub fn canonicalize_mounts(&mut self) -> Result<(), KuinaError> {
        for mount in &mut self.mounts {
            mount.source = std::fs::canonicalize(&mount.source)?;
        }
        Ok(())
    }
}

pub fn containerfile(base_image: &str, packages: &[String], extensions: &[String]) -> String {
    let mut packages = packages.to_vec();
    let mut extensions = extensions.to_vec();
    packages.sort();
    extensions.sort();
    let mut out = format!("FROM {base_image}\n\n");
    if !packages.is_empty() {
        out.push_str("RUN apt-get update && apt-get install -y --no-install-recommends \\\n");
        for package in &packages {
            let _ = writeln!(out, "  {package} \\");
        }
        out.push_str("  && apt-get clean && rm -rf /var/lib/apt/lists/*\n\n");
    }
    out.push_str("RUN npm install -g --ignore-scripts @earendil-works/pi-coding-agent");
    for extension in &extensions {
        let _ = write!(out, " && npm install -g --ignore-scripts {extension}");
    }
    out.push_str("\n\nRUN useradd --create-home --shell /bin/bash worker\nUSER worker\nENV HOME=/home/worker USER=worker\n");
    out
}

pub fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex_encode(&hasher.finalize())
}

pub fn image_name(containerfile: &str) -> String {
    format!("pi-sandbox:{}", content_hash(containerfile))
}

pub fn environment(config: &Config, token: &str, proxy_addr: SocketAddr) -> Vec<(String, String)> {
    let mut values = vec![
        ("HOME".into(), "/home/worker".into()),
        ("USER".into(), "worker".into()),
        (
            "PATH".into(),
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        ),
        ("TMPDIR".into(), "/tmp".into()),
        (
            "OPENAI_BASE_URL".into(),
            format!("http://host.containers.internal:{}/v1", proxy_addr.port()),
        ),
        ("OPENAI_API_KEY".into(), token.into()),
    ];
    for (key, value) in &config.env.static_values {
        values.push((key.clone(), value.clone()));
    }
    for key in &config.env.passthrough {
        if let Ok(value) = env::var(key) {
            values.push((key.clone(), value));
        }
    }
    for key in ["TERM", "COLORTERM", "LANG", "LC_ALL"] {
        if let Ok(value) = env::var(key) {
            values.push((key.into(), value));
        }
    }
    values
}

pub fn mount_args(config: &Config, workspace: &Path, session_dir: &Path) -> Vec<String> {
    let mut args = vec![
        "-v".into(),
        format!("{}:{}:rw", workspace.display(), workspace.display()),
        "-v".into(),
        format!("{}:/home/worker/.pi-sessions:rw", session_dir.display()),
    ];
    for mount in &config.mounts {
        args.push("-v".into());
        args.push(format!(
            "{}:{}:{}",
            mount.source.display(),
            mount.target.display(),
            mount.mode
        ));
    }
    args
}

pub fn podman_run_args(
    config: &Config,
    image: &str,
    workspace: &Path,
    session_dir: &Path,
    envs: &[(String, String)],
    entrypoint: &str,
) -> Vec<String> {
    let mut args = vec![
        "run".into(),
        "--rm".into(),
        "-it".into(),
        "--userns=keep-id".into(),
        "--cap-drop=ALL".into(),
        "--security-opt=no-new-privileges".into(),
        "--pull=never".into(),
        "--add-host=host.containers.internal:host-gateway".into(),
        "--user=worker".into(),
        "--workdir".into(),
        workspace.display().to_string(),
    ];
    for (key, value) in envs {
        args.extend(["--env".into(), format!("{key}={value}")]);
    }
    args.extend(mount_args(config, workspace, session_dir));
    args.extend([image.into(), entrypoint.into()]);
    args
}

pub async fn proxy_handler(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<ProxyState>>,
    req: axum::http::Request<axum::body::Body>,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    use axum::body::Body;
    use axum::http::header::{AUTHORIZATION, HOST};
    let auth = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    if auth != Some(&format!("Bearer {}", state.token)) {
        return Err(axum::http::StatusCode::UNAUTHORIZED);
    }
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or("/");
    let url = format!("{}{}", state.base_url.trim_end_matches('/'), path_query);
    let method = req.method().clone();
    let headers = req.headers().clone();
    let body = req.into_body().into_data_stream();
    let mut builder = state
        .client
        .request(method, url)
        .body(reqwest::Body::wrap_stream(body));
    for (name, value) in &headers {
        if name != AUTHORIZATION && name != HOST {
            builder = builder.header(name, value);
        }
    }
    builder = builder.header(AUTHORIZATION, format!("Bearer {}", state.api_key));
    let response = builder
        .send()
        .await
        .map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
    let status = response.status();
    let response_headers = response.headers().clone();
    let stream = response.bytes_stream();
    let mut output = axum::http::Response::new(Body::from_stream(stream));
    *output.status_mut() = status;
    for (name, value) in &response_headers {
        output.headers_mut().append(name, value.clone());
    }
    Ok(output)
}

pub struct ProxyState {
    pub client: reqwest::Client,
    pub base_url: String,
    pub api_key: String,
    pub token: String,
}

pub fn random_token() -> String {
    use rand::RngExt;
    let mut bytes = [0_u8; 32];
    rand::rng().fill(&mut bytes);
    format!("sk-sandbox-{}", hex_encode(&bytes))
}
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn session_dir(workspace: &Path) -> PathBuf {
    let hash = content_hash(&workspace.to_string_lossy());
    PathBuf::from(env::var_os("HOME").unwrap_or_else(|| ".".into()))
        .join(".local/share/pi-sandbox")
        .join(hash)
        .join("sessions")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn containerfile_is_deterministic() {
        let a = containerfile("node:slim", &["git".into(), "bash".into()], &["x".into()]);
        let b = containerfile("node:slim", &["bash".into(), "git".into()], &["x".into()]);
        assert_eq!(a, b);
    }
    #[test]
    fn token_has_expected_shape() {
        assert!(random_token().starts_with("sk-sandbox-"));
    }
    #[test]
    fn hash_is_stable() {
        assert_eq!(content_hash("x"), content_hash("x"));
    }
}
