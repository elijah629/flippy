use anyhow::{Result, anyhow};
use clap::{ArgAction, Parser, Subcommand};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{Level, error};
use types::flip::Flip;

use crate::art::{FLIPPY, get_art};

mod art;
mod commands;
mod flipper;
mod git;
mod progress;
mod types;
mod validators;
mod walking_diff;

/// Flippy: Automates upgrades and pulls remote databases, files, and firmware for the Flipper Zero.
#[derive(Parser, Debug)]
#[command(
    name = "flippy",
    version,
    about = format!("{FLIPPY}Automates upgrades and pulls remote databases, files, and firmware for the Flipper Zero :kekw:"),
    propagate_version = true,
    subcommand_required = true
)]
struct Cli {
    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Interactive setup for a new flip
    New {
        /// Path to initialize
        #[arg(value_parser, default_value = ".")]
        path: PathBuf,
    },

    /// Upload local changes to remote storage
    Upload {
        /// Force walking full directory tree
        #[arg(short, long)]
        force_walkdir: bool,

        /// Path of project
        #[arg(value_parser, default_value = ".")]
        path: PathBuf,
    },

    /// Manages mappings in flip.toml files
    Map {
        /// Database type
        #[arg(value_parser = ["subghz","rfid","nfc","ir","ibutton","badusb"])]
        db_type: String,

        /// Repository name
        repo: String,

        /// Pathspec to include or exclude
        pathspec: PathBuf,

        /// Exclude instead of include
        #[arg(long)]
        excludes: bool,

        /// Path of project
        #[arg(value_parser, default_value = ".")]
        path: PathBuf,
    },

    /// Add or remove repositories
    Repo {
        #[command(subcommand)]
        command: RepoCommand,
    },

    /// Manages firmware settings
    Firmware {
        #[command(subcommand)]
        command: FirmwareCommand,
    },

    /// Manages store files and updates repositories
    Store {
        #[command(subcommand)]
        command: StoreCommand,
    },
}

#[derive(Subcommand, Debug)]
enum RepoCommand {
    /// Add a repository to fetch files/folders from
    Add {
        /// URL to repository
        #[arg(value_parser)]
        url: url::Url,

        /// Name for identification
        name: String,

        /// Path of project
        #[arg(value_parser, default_value = ".")]
        path: PathBuf,
    },

    /// Remove a repository
    Remove {
        /// Repository name
        name: String,

        /// Path of project
        #[arg(value_parser, default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum FirmwareCommand {
    /// Set the firmware used during updates
    Set {
        /// URL or identifier of firmware
        firmware: String,

        /// Path of project
        #[arg(value_parser, default_value = ".")]
        path: PathBuf,
    },

    /// Pulls the current firmware into the store, then puts it onto the flipper and updates it.
    Update {
        /// Path of project
        #[arg(value_parser, default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum StoreCommand {
    /// Fetch all repositories and firmware files
    Fetch {
        /// Path of project
        #[arg(value_parser, default_value = ".")]
        path: PathBuf,
    },

    /// Deletes the store and everything inside of it
    Clean {
        /// Path of project
        #[arg(value_parser, default_value = ".")]
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    init_logging(cli.verbose);

    if let Err(err) = run(cli).await {
        error!("{}", get_art());
        error!("{}", err);

        std::process::exit(1);
    }
    Ok(())
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::New { path } => {
            commands::new::run(path).await?;
        }
        Commands::Upload {
            force_walkdir,
            path,
        } => {
            let flip = try_flip_from_path(&path).await?;
            commands::upload::run(flip, force_walkdir).await?;
        }
        Commands::Map {
            db_type,
            repo,
            pathspec,
            excludes,
            path,
        } => {
            let flip = try_flip_from_path(&path).await?;
            commands::map::run(flip, db_type, repo, pathspec, excludes).await?;
        }
        Commands::Repo { command } => match command {
            RepoCommand::Add { url, name, path } => {
                let flip = try_flip_from_path(&path).await?;
                commands::repo::add(flip, url, name).await?;
            }

            RepoCommand::Remove { name, path } => {
                let flip = try_flip_from_path(&path).await?;
                commands::repo::remove(flip, name).await?;
            }
        },

        Commands::Firmware { command } => match command {
            FirmwareCommand::Set { firmware, path } => {
                let flip = try_flip_from_path(&path).await?;

                commands::firmware::set(flip, firmware).await?;
            }
            FirmwareCommand::Update { path } => {
                let flip = try_flip_from_path(&path).await?;

                commands::firmware::update(flip).await?;
            }
        },

        Commands::Store { command } => match command {
            StoreCommand::Fetch { path } => {
                let flip = try_flip_from_path(&path).await?;

                commands::store::fetch(flip).await?;
            }
            StoreCommand::Clean { path } => {
                let flip = try_flip_from_path(&path).await?;

                commands::store::clean(flip).await?;
            }
        },
    }
    Ok(())
}

async fn try_flip_from_path(p: impl AsRef<Path>) -> Result<Flip> {
    let project = fs::canonicalize(p).await?;
    if !Flip::exists(&project).await? {
        return Err(anyhow!("No flippy project found at {}", project.display()));
    }
    let flip = Flip::from_path(&project).await?;

    Ok(flip)
}

/// Initialize logging based on verbosity count
fn init_logging(level: u8) {
    let level = match level {
        0 => Level::INFO,
        1 => Level::DEBUG,
        2 => Level::TRACE,
        _ => Level::TRACE,
    };

    let mut sub = tracing_subscriber::fmt()
        .compact()
        .with_max_level(level)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_level(true)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time()
        .with_target(true);

    if level == Level::TRACE {
        sub = sub.with_file(true).with_line_number(true);
    }

    sub.init();
}
