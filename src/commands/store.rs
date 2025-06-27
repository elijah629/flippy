use anyhow::{Result, bail};
use cliclack::confirm;
use gix::open;
use tokio::fs;

use crate::{git, progress::progress, types::flip::Flip};

pub async fn fetch(flip: Flip) -> Result<()> {
    let path = flip.source_path;

    let (progress, handle) = progress();

    for (i, (name, repo)) in flip.repositories.iter().enumerate() {
        let url = gix::url::parse(repo.url.as_str().into())?;
        let path = path.join("store").join(repo.uuid.to_string());
        let exists = fs::try_exists(&path).await?;

        let sub_progress_str = format!(
            "{}: {url} -> store/{}",
            if exists { "fetching" } else { "cloning" },
            repo.uuid
        );

        let mut sub_progress = progress.add_child_with_id(name, (i as u32).to_le_bytes());

        sub_progress.info(sub_progress_str);

        if exists {
            let repo = open(path)?;

            git::fetch::fetch(repo, None, &mut sub_progress)?;
        } else {
            git::clone::clone(url.to_string(), Some(path), &mut sub_progress)?;
        }
    }

    handle.shutdown_and_wait();
    Ok(())
}

pub async fn clean(flip: Flip) -> Result<()> {
    let path = flip.source_path;

    if !confirm("Delete all store items? This includes repos, firmware, caches, ...etc.")
        .interact()?
    {
        bail!("Aborted");
    }

    tokio::fs::remove_dir_all(path.join("store")).await?;
    tokio::fs::create_dir(path.join("store")).await?;

    Ok(())
}
