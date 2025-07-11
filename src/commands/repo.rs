use anyhow::anyhow;
use tracing::{debug, info, instrument};
use url::Url;
use uuid::Uuid;

use crate::{
    types::{flip::Flip, mapping::Mappings, repository::Repository},
    validators::validate_project_name,
};

#[instrument]
pub async fn add(mut flip: Flip, url: Url, name: String) -> anyhow::Result<()> {
    debug!("validating url");
    let url = gix::url::parse(url.as_str().into())?;

    debug!("Validating name");
    validate_project_name(&name)?;

    let url = url.to_string();
    let data = url.as_bytes();
    debug!("inserting repository into list, building UUID");
    let uuid = Uuid::new_v5(&Uuid::NAMESPACE_URL, data);

    debug!("Checking if repo already exists");
    if flip
        .repositories
        .iter()
        .any(|(fname, frepo)| **fname == name || frepo.uuid == uuid)
    {
        return Err(anyhow!(
            "repository: {name}/{uuid} already exists, either by UUID or name"
        ));
    }

    info!(name, "Successfully created repo {}", name);
    flip.repositories.insert(
        name,
        Repository {
            uuid,
            url,
            mappings: Mappings::default(),
        },
    );

    flip.write().await?;

    Ok(())
}

#[instrument]
pub async fn remove(mut flip: Flip, name: String) -> anyhow::Result<()> {
    match flip.repositories.remove(&name) {
        Some(_) => {
            info!("deleted repository {}", name);
        }
        None => {
            return Err(anyhow!("repository: {} not found", name));
        }
    }

    flip.write().await?;

    Ok(())
}
