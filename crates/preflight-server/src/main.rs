use std::sync::Arc;

use clap::Parser;
use preflight_core::json_store::JsonFileStore;
use preflight_mcp::client::PreflightClient;
use preflight_mcp::server::PreflightMcp;
use rmcp::{ServiceExt, transport::stdio};
use tokio::net::TcpListener;

#[derive(Parser)]
#[command(
    name = "preflight",
    about = "Local code review tool for AI-generated changes"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Start the web server
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "3000", env = "PREFLIGHT_PORT")]
        port: u16,
    },
    /// Start the MCP stdio server
    Mcp {
        /// Port of the running preflight web server to connect to
        #[arg(long, default_value = "3000", env = "PREFLIGHT_PORT")]
        port: u16,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Command::Serve { port: 3000 }) {
        Command::Serve { port } => run_serve(port).await,
        Command::Mcp { port } => run_mcp(port).await,
    }
}

async fn run_serve(port: u16) {
    let store = JsonFileStore::new("preflight-state.json").await.unwrap();
    let app = preflight_server::app(Arc::new(store));
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn run_mcp(port: u16) {
    let client = PreflightClient::new(port);
    let ws_tx = client.connect_ws().await;
    let server = PreflightMcp::new(client, ws_tx);
    let service = server.serve(stdio()).await.unwrap();
    service.waiting().await.unwrap();
}
