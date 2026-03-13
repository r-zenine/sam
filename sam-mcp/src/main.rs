mod loader;
mod resolver;
mod server;

use rmcp::transport::stdio;
use rmcp::ServiceExt;
use server::SamMcpServer;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let config_path = parse_config_flag();
    let ctx = match config_path {
        Some(path) => loader::load_from(path).expect("failed to load sam config"),
        None => loader::load().expect("failed to load sam config"),
    };
    SamMcpServer::new(Arc::new(ctx)).serve(stdio()).await.unwrap();
}

fn parse_config_flag() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    let pos = args.iter().position(|a| a == "--config")?;
    args.get(pos + 1).map(PathBuf::from)
}
