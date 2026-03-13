mod loader;
mod resolver;
mod server;

use rmcp::transport::stdio;
use rmcp::ServiceExt;
use server::SamMcpServer;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let ctx = Arc::new(loader::load().expect("failed to load sam config"));
    SamMcpServer::new(ctx).serve(stdio()).await.unwrap();
}
