use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::sources::{FirmwareSource, RepositorySource};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Manage Flipper Zero project-side configuration"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {
    /// Create a new project directory with `flip.toml` and `store/`.
    New(NewArgs),
    /// Update `flip.toml`.
    Map(MapArgs),
    /// Manage the local `store/` directory and fetch plan.
    Store(StoreArgs),
}

#[derive(Debug, Args)]
pub struct NewArgs {
    /// Path used to store project files.
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct MapArgs {
    #[command(subcommand)]
    pub command: MapCommand,
}

#[derive(Debug, Subcommand)]
pub enum MapCommand {
    /// Edit repository definitions in `flip.toml`.
    Repo(RepoArgs),
    /// Edit the firmware source in `flip.toml`.
    Firmware(FirmwareArgs),
}

#[derive(Debug, Args)]
pub struct RepoArgs {
    #[command(subcommand)]
    pub command: RepoCommand,
}

#[derive(Debug, Subcommand)]
pub enum RepoCommand {
    /// Add a repository source to `flip.toml`.
    Add(RepoAddArgs),
}

#[derive(Debug, Args)]
pub struct RepoAddArgs {
    /// Repository URL, SCP-style Git remote, or local path.
    pub source: RepositorySource,
    /// Optional local identifier for the repository.
    pub name: Option<String>,
}

#[derive(Debug, Args)]
pub struct FirmwareArgs {
    #[command(subcommand)]
    pub command: FirmwareCommand,
}

#[derive(Debug, Subcommand)]
pub enum FirmwareCommand {
    /// Set the firmware source used in `flip.toml`.
    Set(FirmwareSetArgs),
}

#[derive(Debug, Args)]
pub struct FirmwareSetArgs {
    /// Firmware preset (`ofw`, `unleashed`, `momentum`, `rogue-free`) or a local/remote tarball path.
    pub source: FirmwareSource,
}

#[derive(Debug, Args)]
pub struct StoreArgs {
    #[command(subcommand)]
    pub command: StoreCommand,
}

#[derive(Debug, Subcommand)]
pub enum StoreCommand {
    /// Create the local `store/` directory if it does not exist.
    Create,
    /// Validate sources and write `store/fetch-plan.txt`.
    Fetch(StoreFetchArgs),
}

#[derive(Debug, Args)]
pub struct StoreFetchArgs {
    /// Record whether sparse or otherwise optimized fetching is intended.
    #[arg(short, long)]
    pub optimize: bool,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::{Cli, CliCommand, FirmwareCommand, MapCommand, RepoCommand};
    use crate::sources::{FirmwarePreset, FirmwareSource, RepositoryRemote, RepositorySource};

    #[test]
    fn firmware_set_accepts_relative_local_paths() {
        let cli = Cli::try_parse_from(["flippy", "map", "firmware", "set", "fw.tgz"]).unwrap();

        let source = match cli.command {
            CliCommand::Map(map) => match map.command {
                MapCommand::Firmware(firmware) => match firmware.command {
                    FirmwareCommand::Set(set) => set.source,
                },
                _ => panic!("expected firmware subcommand"),
            },
            _ => panic!("expected map command"),
        };

        assert_eq!(source, FirmwareSource::Local(PathBuf::from("fw.tgz")));
    }

    #[test]
    fn firmware_set_accepts_presets() {
        let cli = Cli::try_parse_from(["flippy", "map", "firmware", "set", "momentum"]).unwrap();

        let source = match cli.command {
            CliCommand::Map(map) => match map.command {
                MapCommand::Firmware(firmware) => match firmware.command {
                    FirmwareCommand::Set(set) => set.source,
                },
                _ => panic!("expected firmware subcommand"),
            },
            _ => panic!("expected map command"),
        };

        assert_eq!(source, FirmwareSource::Preset(FirmwarePreset::Momentum));
    }

    #[test]
    fn repo_add_accepts_local_paths() {
        let cli = Cli::try_parse_from(["flippy", "map", "repo", "add", "../Flipper-IRDB", "irdb"])
            .unwrap();

        let source = match cli.command {
            CliCommand::Map(map) => match map.command {
                MapCommand::Repo(repo) => match repo.command {
                    RepoCommand::Add(add) => add.source,
                },
                _ => panic!("expected repo subcommand"),
            },
            _ => panic!("expected map command"),
        };

        assert_eq!(
            source,
            RepositorySource::Local(PathBuf::from("../Flipper-IRDB"))
        );
    }

    #[test]
    fn repo_add_accepts_scp_like_remotes() {
        let cli = Cli::try_parse_from([
            "flippy",
            "map",
            "repo",
            "add",
            "git@github.com:flipperdevices/flipperzero-firmware.git",
        ])
        .unwrap();

        let source = match cli.command {
            CliCommand::Map(map) => match map.command {
                MapCommand::Repo(repo) => match repo.command {
                    RepoCommand::Add(add) => add.source,
                },
                _ => panic!("expected repo subcommand"),
            },
            _ => panic!("expected map command"),
        };

        assert_eq!(
            source,
            RepositorySource::Remote(RepositoryRemote::ScpLike(
                "git@github.com:flipperdevices/flipperzero-firmware.git".to_owned(),
            ))
        );
    }
}
