//! davd daemon entry point
//!
//! This is the main entry point for the davd daemon. It can be run in two modes:
//! - CLI mode: for manual sync triggers and queries
//! - Daemon mode: background service that runs the D-Bus interface

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "davd=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Sync { account } => {
            tracing::info!("Triggering sync for account: {:?}", account);
            // TODO: Implement sync command
            todo!("Sync command not yet implemented");
        }
        Commands::ListEvents { start, end } => {
            tracing::info!("Listing events from {} to {}", start, end);
            // TODO: Implement list-events command
            todo!("List events command not yet implemented");
        }
        Commands::Daemon => {
            tracing::info!("Starting davd daemon");
            // TODO: Start D-Bus service and sync loop
            todo!("Daemon mode not yet implemented");
        }
    }
}
