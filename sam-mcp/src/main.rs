mod loader;
mod resolver;
mod server;

use rmcp::service::QuitReason;
use rmcp::transport::stdio;
use rmcp::ServiceExt;
use server::SamMcpServer;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    init_logging();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "sam-mcp started");

    let config_path = parse_config_flag();
    let ctx = match config_path {
        Some(path) => loader::load_from(path),
        None => loader::load(),
    }
    .unwrap_or_else(|e| {
        tracing::error!(error = %e, "failed to load sam config");
        std::process::exit(1);
    });

    let service = SamMcpServer::new(Arc::new(ctx))
        .serve(stdio())
        .await
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "failed to initialize MCP service");
            std::process::exit(1);
        });

    match service.waiting().await {
        Ok(QuitReason::Closed | QuitReason::Cancelled) => {
            tracing::info!("sam-mcp stopped");
        }
        Ok(QuitReason::JoinError(e)) => {
            tracing::error!(error = %e, "service task panicked");
            std::process::exit(1);
        }
        Ok(reason) => {
            tracing::warn!(?reason, "sam-mcp stopped with unexpected reason");
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to join service task");
            std::process::exit(1);
        }
    }
}

fn init_logging() {
    let log_path = std::env::var("SAM_MCP_LOG_FILE").unwrap_or_else(|_| {
        let base = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{base}/.local/share/sam-mcp/sam-mcp.log")
    });

    let log_file_path = std::path::Path::new(&log_path);
    if let Some(parent) = log_file_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("sam-mcp: could not open log file {log_path}: {e}");
            return;
        }
    };

    let level = std::env::var("SAM_MCP_LOG").unwrap_or_else(|_| "info".to_string());
    let filter = tracing_subscriber::EnvFilter::new(level);

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::sync::Mutex::new(file))
        .with_ansi(false)
        .init();
}

fn parse_config_flag() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    let pos = args.iter().position(|a| a == "--config")?;
    args.get(pos + 1).map(PathBuf::from)
}
