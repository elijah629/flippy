use crate::types::directory::Id;
use crate::types::firmware::Firmware;
use crate::types::flip::Flip;
use crate::validators::{ProjectNameValidator, URLValidator};
use anyhow::anyhow;
use cliclack::{confirm, input, intro, outro, select};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};

pub async fn run<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    intro("flippy new")?;

    debug!("Extracting default name from provided path");
    let default_name_opt: Option<&str> = path.as_ref().file_name().and_then(|v| v.to_str());

    debug!("Starting new flip wizard");

    let config = new_wizard(default_name_opt.map(str::to_string))?;

    debug!("Flip config: {:?}", config);

    debug!("Prompting user to confirm flip creation");
    let confirm = confirm("Create a new flip with these settings?").interact()?;

    if !confirm {
        info!("Cancelled flip creation.");
        return Ok(());
    }

    let final_path = {
        if default_name_opt == Some(&config.0) {
            path.as_ref().to_path_buf()
        } else {
            path.as_ref().join(&config.0)
        }
    };

    if final_path.exists() {
        return Err(anyhow!("directory {} already exists", final_path.display()));
    }

    let config = Flip {
        name: config.0,
        firmware: config.1,
        source_path: final_path,
        ..Flip::default()
    };

    debug!(
        "Creating project directory: {}",
        config.source_path.display()
    );

    // makes both project dir and store
    fs::create_dir_all(config.source_path.join("store")).await?;

    debug!("Creating flip.toml");
    config.write().await?;

    outro("flip created successfuly")?;

    Ok(())
}

fn new_wizard(default_name: Option<String>) -> anyhow::Result<(String, Firmware)> {
    debug!("Prompting for project name");
    let mut name = input("Name").validate_interactively(ProjectNameValidator);

    if let Some(default_name) = default_name {
        name = name.default_input(&default_name);
    }

    let name = name.interact()?;

    debug!("Prompting for firmware source");
    let firmware = {
        let firmware_type = select("Firmware source")
            .item("official", "Official", "maintained by flipper staff")
            .item("unleashed", "Unleashed", "")
            .item("momentum", "Momentum", "")
            .item(
                "custom",
                "Custom (.tgz)",
                "supply a custom firmware through a direct link to a .tgz file",
            )
            .interact()?;

        debug!(firmware_type, "User selected firmware:");

        if firmware_type == "custom" {
            debug!("Prompting for custom firmware URL");
            Firmware::Custom(
                input("URL to custom firmware")
                    .validate(URLValidator)
                    .interact()?,
            )
        } else {
            let channel = select("Channel")
                .initial_value(Id::Release)
                .item(Id::Release, "Release", "Stable releases")
                .item(
                    Id::ReleaseCanidate,
                    "Release candidate",
                    "This is going to be released soon",
                )
                .item(
                    Id::Development,
                    "Development",
                    "Latest builds, not tested, and may be unstable, be careful",
                )
                .interact()?;

            match firmware_type {
                "official" => Firmware::Official(channel),
                "unleashed" => Firmware::Unleashed(channel),
                "momentum" => Firmware::Momentum(channel),

                _ => unreachable!("todo: add more fw matches"),
            }
        }
    };

    Ok((name, firmware))
}
