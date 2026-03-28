use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{
    cli::{Cli, CliCommand, FirmwareCommand, MapCommand, RepoCommand, StoreCommand},
    config::{CONFIG_FILE_NAME, FlipConfig, RepositoryConfig},
    sources::{FirmwareSource, RepositoryRemote, RepositorySource},
};

pub fn run(cli: Cli, cwd: &Path, stdout: &mut impl Write) -> Result<()> {
    match cli.command {
        CliCommand::New(args) => create_project(&args.path, stdout),
        CliCommand::Map(args) => match args.command {
            MapCommand::Repo(repo_args) => match repo_args.command {
                RepoCommand::Add(add_args) => {
                    add_repository(cwd, add_args.source, add_args.name, stdout)
                }
            },
            MapCommand::Firmware(firmware_args) => match firmware_args.command {
                FirmwareCommand::Set(set_args) => set_firmware(cwd, set_args.source, stdout),
            },
        },
        CliCommand::Store(args) => match args.command {
            StoreCommand::Create => create_store(cwd, stdout),
            StoreCommand::Fetch(fetch_args) => fetch_store(cwd, fetch_args.optimize, stdout),
        },
    }
}

fn create_project(path: &Path, stdout: &mut impl Write) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create project directory at '{}'", path.display()))?;

    let config_path = path.join(CONFIG_FILE_NAME);
    if config_path.exists() {
        bail!("project already contains {}", config_path.display());
    }

    let name = path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("flippy");

    let config = FlipConfig::new(name.to_owned());
    config.save_to_dir(path)?;
    ensure_store_dir(path)?;

    writeln!(stdout, "created {}", path.display())?;
    writeln!(stdout, "wrote {}", config_path.display())?;
    Ok(())
}

fn add_repository(
    cwd: &Path,
    source: RepositorySource,
    requested_name: Option<String>,
    stdout: &mut impl Write,
) -> Result<()> {
    let mut config = FlipConfig::load_from_dir(cwd)?;
    let name = requested_name
        .map(|name| validate_repository_name(name.trim()))
        .transpose()?
        .unwrap_or_else(|| config.next_repository_name(suggest_repository_name(&source)));

    if config.repositories.contains_key(&name) {
        bail!(
            "repository '{name}' already exists in {}",
            cwd.join(CONFIG_FILE_NAME).display()
        );
    }

    config.repositories.insert(
        name.clone(),
        RepositoryConfig {
            source: source.clone(),
        },
    );
    config.save_to_dir(cwd)?;

    writeln!(stdout, "added repository '{name}' ({source})")?;
    Ok(())
}

fn set_firmware(cwd: &Path, source: FirmwareSource, stdout: &mut impl Write) -> Result<()> {
    let mut config = FlipConfig::load_from_dir(cwd)?;
    config.firmware = source.clone();
    config.save_to_dir(cwd)?;

    writeln!(stdout, "set firmware to {source}")?;
    Ok(())
}

fn create_store(cwd: &Path, stdout: &mut impl Write) -> Result<()> {
    let store_dir = ensure_store_dir(cwd)?;
    writeln!(stdout, "ensured {}", store_dir.display())?;
    Ok(())
}

fn fetch_store(cwd: &Path, optimize: bool, stdout: &mut impl Write) -> Result<()> {
    let config = FlipConfig::load_from_dir(cwd)?;
    let store_dir = ensure_store_dir(cwd)?;
    validate_sources(cwd, &config)?;

    let plan = render_fetch_plan(&config, optimize);
    let plan_path = store_dir.join("fetch-plan.txt");
    fs::write(&plan_path, plan)
        .with_context(|| format!("failed to write fetch plan to '{}'", plan_path.display()))?;

    writeln!(stdout, "validated {}", cwd.join(CONFIG_FILE_NAME).display())?;
    writeln!(stdout, "wrote {}", plan_path.display())?;
    Ok(())
}

fn ensure_store_dir(cwd: &Path) -> Result<PathBuf> {
    let store_dir = cwd.join("store");
    fs::create_dir_all(&store_dir)
        .with_context(|| format!("failed to create store directory '{}'", store_dir.display()))?;
    Ok(store_dir)
}

fn validate_sources(cwd: &Path, config: &FlipConfig) -> Result<()> {
    if let FirmwareSource::Local(path) = &config.firmware {
        validate_local_path(cwd, path, "firmware source")?;
    }

    for (name, repository) in &config.repositories {
        if let RepositorySource::Local(path) = &repository.source {
            validate_local_path(cwd, path, &format!("repository '{name}'"))?;
        }
    }

    Ok(())
}

fn validate_local_path(cwd: &Path, path: &Path, kind: &str) -> Result<()> {
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };

    if candidate.exists() {
        Ok(())
    } else {
        bail!("{kind} '{}' does not exist", candidate.display())
    }
}

fn render_fetch_plan(config: &FlipConfig, optimize: bool) -> String {
    let mut plan = String::new();
    plan.push_str("flippy fetch plan\n");
    plan.push_str("=================\n");
    plan.push_str(&format!("name: {}\n", config.name));
    plan.push_str(&format!("optimize: {optimize}\n"));
    plan.push_str(&format!("firmware: {}\n", config.firmware));
    plan.push_str("repositories:\n");

    if config.repositories.is_empty() {
        plan.push_str("- none\n");
    } else {
        for (name, repository) in &config.repositories {
            plan.push_str(&format!("- {name}: {}\n", repository.source));
        }
    }

    plan
}

fn validate_repository_name(name: &str) -> Result<String> {
    if name.is_empty() {
        bail!("repository name cannot be empty");
    }

    Ok(name.to_owned())
}

fn suggest_repository_name(source: &RepositorySource) -> String {
    match source {
        RepositorySource::Local(path) => path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .map(normalize_repository_name)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "repository".to_owned()),
        RepositorySource::Remote(RepositoryRemote::Url(url)) => url
            .path_segments()
            .and_then(Iterator::last)
            .map(normalize_repository_name)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "repository".to_owned()),
        RepositorySource::Remote(RepositoryRemote::ScpLike(remote)) => remote
            .rsplit(['/', ':'])
            .next()
            .map(normalize_repository_name)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "repository".to_owned()),
    }
}

fn normalize_repository_name(value: &str) -> String {
    let trimmed = value.trim_end_matches(".git");
    let mut normalized = String::with_capacity(trimmed.len());
    let mut last_was_separator = false;

    for character in trimmed.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            normalized.push('-');
            last_was_separator = true;
        }
    }

    normalized.trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use clap::Parser;
    use tempfile::tempdir;

    use super::run;
    use crate::{
        cli::Cli,
        config::FlipConfig,
        sources::{FirmwarePreset, FirmwareSource, RepositoryRemote, RepositorySource},
    };

    #[test]
    fn new_creates_project_files() {
        let temp_dir = tempdir().unwrap();
        let project_dir = temp_dir.path().join("demo");
        let cli = Cli::try_parse_from(["flippy", "new", project_dir.to_str().unwrap()]).unwrap();
        let mut output = Vec::new();

        run(cli, temp_dir.path(), &mut output).unwrap();

        assert!(project_dir.join("flip.toml").exists());
        assert!(project_dir.join("store").exists());
    }

    #[test]
    fn map_commands_update_flip_toml() {
        let temp_dir = tempdir().unwrap();
        let mut output = Vec::new();

        run(
            Cli::try_parse_from(["flippy", "new", temp_dir.path().to_str().unwrap()]).unwrap(),
            temp_dir.path(),
            &mut output,
        )
        .unwrap();

        run(
            Cli::try_parse_from(["flippy", "map", "firmware", "set", "rogue-free"]).unwrap(),
            temp_dir.path(),
            &mut output,
        )
        .unwrap();
        run(
            Cli::try_parse_from([
                "flippy",
                "map",
                "repo",
                "add",
                "git@github.com:flipperdevices/flipperzero-firmware.git",
            ])
            .unwrap(),
            temp_dir.path(),
            &mut output,
        )
        .unwrap();

        let config = FlipConfig::load_from_dir(temp_dir.path()).unwrap();
        assert_eq!(
            config.firmware,
            FirmwareSource::Preset(FirmwarePreset::RogueFree)
        );
        assert_eq!(config.repositories.len(), 1);
        assert_eq!(
            config.repositories["flipperzero-firmware"].source,
            RepositorySource::Remote(RepositoryRemote::ScpLike(
                "git@github.com:flipperdevices/flipperzero-firmware.git".to_owned(),
            ))
        );
    }

    #[test]
    fn fetch_validates_local_sources_and_writes_plan() {
        let temp_dir = tempdir().unwrap();
        let firmware_path = temp_dir.path().join("fw.tgz");
        fs::write(&firmware_path, b"firmware").unwrap();
        let repository_dir = temp_dir.path().join("Flipper-IRDB");
        fs::create_dir_all(&repository_dir).unwrap();

        let mut output = Vec::new();
        run(
            Cli::try_parse_from(["flippy", "new", temp_dir.path().to_str().unwrap()]).unwrap(),
            temp_dir.path(),
            &mut output,
        )
        .unwrap();
        run(
            Cli::try_parse_from(["flippy", "map", "firmware", "set", "path://fw.tgz"]).unwrap(),
            temp_dir.path(),
            &mut output,
        )
        .unwrap();
        run(
            Cli::try_parse_from([
                "flippy",
                "map",
                "repo",
                "add",
                "path://Flipper-IRDB",
                "irdb",
            ])
            .unwrap(),
            temp_dir.path(),
            &mut output,
        )
        .unwrap();

        run(
            Cli::try_parse_from(["flippy", "store", "fetch", "--optimize"]).unwrap(),
            temp_dir.path(),
            &mut output,
        )
        .unwrap();

        let plan =
            fs::read_to_string(temp_dir.path().join("store").join("fetch-plan.txt")).unwrap();
        assert!(plan.contains("optimize: true"));
        assert!(plan.contains("firmware: path://fw.tgz"));
        assert!(plan.contains("- irdb: path://Flipper-IRDB"));
    }

    #[test]
    fn fetch_fails_for_missing_local_sources() {
        let temp_dir = tempdir().unwrap();
        let mut output = Vec::new();

        run(
            Cli::try_parse_from(["flippy", "new", temp_dir.path().to_str().unwrap()]).unwrap(),
            temp_dir.path(),
            &mut output,
        )
        .unwrap();
        run(
            Cli::try_parse_from(["flippy", "map", "firmware", "set", "path://missing.tgz"])
                .unwrap(),
            temp_dir.path(),
            &mut output,
        )
        .unwrap();

        let error = run(
            Cli::try_parse_from(["flippy", "store", "fetch"]).unwrap(),
            temp_dir.path(),
            &mut output,
        )
        .unwrap_err();

        assert!(error.to_string().contains("firmware source"));
    }
}
