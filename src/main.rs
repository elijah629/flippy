use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand, arg, command};
use std::path::PathBuf;
use tokio::fs;
use tracing::Level;
use types::flip::Flip;
use url::Url;

mod commands;
mod git;
mod progress;
mod types;
mod validators;
mod walking_diff;

/// CLI entrypoint
#[derive(Parser, Debug)]
#[command(
    about = "Flippy is a CLI tool to manager flipper zero repositories, databases, and firmware. It will copy and install firmware upgrades and pull remote databases automatically.",
    subcommand_required = true
)]
struct Cli {
    /// Sets the logging level
    #[arg(long, default_value = "info")]
    log_level: Level,

    #[command(subcommand)]
    command: Commands,

    /// Path to flip
    #[arg(value_parser, default_value = ".")]
    path: PathBuf,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Interactive setup for a new flip
    New,

    Upload,

    /// Manages mappings in flip.toml files
    Map {
        #[arg(value_parser = [ "subghz", "rfid","nfc", "ir", "ibutton", "badusb" ])]
        db_type: String,

        /// Name of repository
        repo: String,

        /// Path
        path: PathBuf,
    },

    /// Add or removes repositories
    Repo {
        #[command(subcommand)]
        command: Repo,
    },

    /// Manages firmware settings
    Firmware {
        #[command(subcommand)]
        command: Firmware,
    },

    /// Manages store files and updates repositories
    Store {
        #[command(subcommand)]
        command: Store,
    },
}

#[derive(Subcommand, Debug)]
enum Repo {
    /// Adds a repository to fetch files/folders from
    Add {
        /// URL to repository
        #[arg(value_parser)]
        url: Url,

        /// Name to use for repository identification
        name: String,
    },

    /// Removes a repository
    Remove {
        /// Repository name
        name: String,
    },
}

#[derive(Subcommand, Debug)]
enum Firmware {
    /// Sets the firmware used during updates
    Set {
        /// URL or identifier of firmware
        firmware: String,
    },

    /// Pulls the current firmware into the store
    Pull,
}

#[derive(Subcommand, Debug)]
enum Store {
    /// Fetches all repositories and firmware files
    Fetch, //{
           // /// Uses experimental 'git sparse-checkout'
           // TODO: Wait for gix to implement sparse functionality.
           // #[arg(short = 'o', long = "optimize")]
           // optimize: bool,
           // },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut sub = tracing_subscriber::fmt()
        .compact()
        .with_max_level(cli.log_level)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_level(true)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time()
        .with_target(true);

    if cli.log_level >= Level::TRACE {
        sub = sub.with_file(true).with_line_number(true);
    }

    sub.init();

    if let Err(e) = run(cli).await {
        tracing::error!("{e}");

        std::process::exit(1);
    }

    Ok(())
}

async fn run(cli: Cli) -> Result<()> {
    if matches!(cli.command, Commands::New) {
        commands::new::run(cli.path).await?;
        return Ok(());
    }

    let path = fs::canonicalize(&cli.path).await?;

    if !Flip::exists(&path).await? {
        return Err(anyhow!("Project not found in {}", path.display()));
    }

    let flip = Flip::from_path(&path).await?;

    match cli.command {
        Commands::Upload => commands::upload::run(flip).await?,

        Commands::Map {
            db_type,
            repo,
            path,
        } => commands::map::run(flip, db_type, repo, path).await?,

        Commands::Repo { command } => match command {
            Repo::Add { url, name } => {
                commands::repo::add(flip, url, name).await?;
            }

            Repo::Remove { name } => {
                commands::repo::remove(flip, name).await?;
            }
        },

        Commands::Firmware { command } => match command {
            Firmware::Set { firmware } => {
                commands::firmware::set(flip, firmware).await?;
            }
            Firmware::Pull => commands::firmware::pull(flip).await?,
        },

        Commands::Store { command } => match command {
            Store::Fetch => {
                commands::store::fetch(flip).await?;
            }
        },

        _ => unreachable!("Impossible for new to reach after new was checked"),
    }

    Ok(())
}
