use axum::{Router, routing::any};
use clap::{Parser, Subcommand};
use kuina::{
    Config, KuinaError, ProxyState, containerfile, environment, image_name, podman_run_args,
    proxy_handler, random_token, session_dir,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;

#[derive(Debug, Parser)]
#[command(name = "kuina")]
struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    Doctor,
    Version,
    Proxy {
        #[command(subcommand)]
        command: ProxyCommand,
    },
    Debug {
        #[command(subcommand)]
        command: DebugCommand,
    },
    Run {
        #[arg(long)]
        rebuild: bool,
    },
    Sh {
        #[arg(long)]
        rebuild: bool,
    },
}
#[derive(Debug, Subcommand)]
enum ProxyCommand {
    Start,
    Status,
    Stop,
    Restart,
}
#[derive(Debug, Subcommand)]
enum DebugCommand {
    Containerfile,
    PodmanCommand,
}

#[tokio::main]
async fn main() {
    if let Err(error) = execute(Cli::parse()).await {
        eprintln!("kuina: {error}");
        std::process::exit(1);
    }
}

async fn execute(cli: Cli) -> Result<(), KuinaError> {
    match cli.command {
        CommandKind::Version => {
            println!("kuina {}", env!("CARGO_PKG_VERSION"));
        }
        CommandKind::Doctor => doctor().await?,
        CommandKind::Debug {
            command: DebugCommand::Containerfile,
        } => println!("{}", build_containerfile()?),
        CommandKind::Debug {
            command: DebugCommand::PodmanCommand,
        } => println!("{}", build_command(false)?),
        CommandKind::Run { rebuild } => run("pi", rebuild).await?,
        CommandKind::Sh { rebuild } => run("bash", rebuild).await?,
        CommandKind::Proxy { command } => proxy_command(command).await?,
    }
    Ok(())
}

fn config() -> Result<Config, KuinaError> {
    Config::load(&PathBuf::from("config.toml"))
}
fn build_containerfile() -> Result<String, KuinaError> {
    let c = config()?;
    Ok(containerfile(
        "node:22-bookworm-slim",
        &c.packages,
        &c.pi_extensions,
    ))
}
fn build_command(rebuild: bool) -> Result<String, KuinaError> {
    let c = config()?;
    let file = containerfile("node:22-bookworm-slim", &c.packages, &c.pi_extensions);
    let image = image_name(&file);
    let workspace = std::env::current_dir()?;
    let session = session_dir(&workspace);
    let envs = environment(
        &c,
        "<redacted>",
        "127.0.0.1:0"
            .parse::<std::net::SocketAddr>()
            .map_err(|e| KuinaError::Config(e.to_string()))?,
    );
    let args = podman_run_args(&c, &image, &workspace, &session, &envs, "pi");
    let mut command = format!(
        "podman build -f Containerfile -t {image} . && podman {}",
        args.join(" ")
    );
    if rebuild {
        command = format!(
            "podman build --no-cache -f Containerfile -t {image} . && podman {}",
            args.join(" ")
        );
    } else {
        command.push_str(&args.join(" "));
    }
    Ok(command)
}

async fn run(entrypoint: &str, _rebuild: bool) -> Result<(), KuinaError> {
    let mut c = config()?;
    c.canonicalize_mounts()?;
    let file = containerfile("node:22-bookworm-slim", &c.packages, &c.pi_extensions);
    let image = image_name(&file);
    let workspace = std::env::current_dir()?;
    let session = session_dir(&workspace);
    tokio::fs::create_dir_all(&session).await?;
    let token = random_token();
    let key = std::env::var(&c.proxy.api_key_env).map_err(|_| {
        KuinaError::Config(format!(
            "missing API key environment variable {}",
            c.proxy.api_key_env
        ))
    })?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let state = Arc::new(ProxyState {
        client: reqwest::Client::new(),
        base_url: c.proxy.base_url.clone(),
        api_key: key,
        token: token.clone(),
    });
    let server = tokio::spawn(async move {
        let router = Router::new().fallback(any(proxy_handler)).with_state(state);
        axum::serve(listener, router).await
    });
    let envs = environment(&c, &token, address);
    let args = podman_run_args(&c, &image, &workspace, &session, &envs, entrypoint);
    let status = Command::new("podman")
        .args(&args)
        .status()
        .await
        .map_err(|e| KuinaError::PodmanStart(e.to_string()))?;
    server.abort();
    status.code().map_or(Err(KuinaError::Podman(1)), |code| {
        if code == 0 {
            Ok(())
        } else {
            Err(KuinaError::Podman(code))
        }
    })
}

async fn doctor() -> Result<(), KuinaError> {
    let output = Command::new("podman").arg("--version").output().await?;
    println!("OS: {}", std::env::consts::OS);
    println!("Podman: {}", String::from_utf8_lossy(&output.stdout).trim());
    Ok(())
}
async fn proxy_command(command: ProxyCommand) -> Result<(), KuinaError> {
    match command {
        ProxyCommand::Start | ProxyCommand::Restart => {
            println!("proxy lifecycle is coupled to kuina run")
        }
        ProxyCommand::Status => println!("stopped"),
        ProxyCommand::Stop => println!("stopped"),
    };
    Ok(())
}
