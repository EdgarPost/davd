//! CLI command definitions

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "davd")]
#[command(about = "CalDAV/CardDAV sync daemon for Linux", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Trigger a manual sync for an account
    Sync {
        /// Account name to sync (optional, syncs all if not specified)
        account: Option<String>,
    },

    /// List events within a time range
    ListEvents {
        /// Start time (ISO 8601 format)
        #[arg(short, long)]
        start: String,

        /// End time (ISO 8601 format)
        #[arg(short, long)]
        end: String,
    },

    /// Run as daemon (D-Bus service)
    Daemon,
}
