use crate::git;
use gix::{
    prelude::ObjectIdExt,
    refspec::match_group::validate::Fix,
    remote::fetch::{Status, refs::update::TypeChange},
};

pub fn fetch<P>(
    repo: gix::Repository,
    remote: Option<String>,
    mut progress: P,
) -> anyhow::Result<()>
where
    P: gix::NestedProgress,
    P::SubProgress: 'static,
{
    let remote = git::remote::by_name_or_url(&repo, remote.as_deref())?;
    let res: gix::remote::fetch::Outcome = remote
        .connect(gix::remote::Direction::Fetch)?
        .prepare_fetch(&mut progress, Default::default())?
        //.with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(1.try_into()?)) Cannot shallow
        //clone due to deep remote diffing on the upload command. This also does not save
        //much data on UberGuidoZ/Flipper
        .receive(&mut progress, &gix::interrupt::IS_INTERRUPTED)?;

    let ref_specs = remote.refspecs(gix::remote::Direction::Fetch);
    match res.status {
        Status::NoPackReceived {
            update_refs,
            negotiate,
            dry_run: _,
        } => {
            print_updates(
                &repo,
                &negotiate.unwrap_or_default(),
                update_refs,
                ref_specs,
                res.ref_map,
                &mut progress,
            )?;
            Ok::<_, anyhow::Error>(())
        }
        Status::Change {
            update_refs,
            write_pack_bundle,
            negotiate,
        } => {
            print_updates(
                &repo,
                &negotiate,
                update_refs,
                ref_specs,
                res.ref_map,
                &mut progress,
            )?;
            if let Some(data_path) = write_pack_bundle.data_path {
                progress.info(format!("pack  file: \"{}\"", data_path.display()));
            }
            if let Some(index_path) = write_pack_bundle.index_path {
                progress.info(format!("index file: \"{}\"", index_path.display()));
            }
            Ok(())
        }
    }?;
    Ok(())
}

pub fn print_updates<P>(
    repo: &gix::Repository,
    negotiate: &gix::remote::fetch::outcome::Negotiate,
    update_refs: gix::remote::fetch::refs::update::Outcome,
    refspecs: &[gix::refspec::RefSpec],
    mut map: gix::remote::fetch::RefMap,
    mut progress: P,
) -> anyhow::Result<()>
where
    P: gix::NestedProgress,
    P::SubProgress: 'static,
{
    let mut last_spec_index = gix::remote::fetch::refmap::SpecIndex::ExplicitInRemote(usize::MAX);
    let mut updates = update_refs
        .iter_mapping_updates(&map.mappings, refspecs, &map.extra_refspecs)
        .filter_map(|(update, mapping, spec, edit)| spec.map(|spec| (update, mapping, spec, edit)))
        .collect::<Vec<_>>();
    updates.sort_by_key(|t| t.2);
    let mut skipped_due_to_implicit_tag = None;
    fn consume_skipped_tags<P>(skipped: &mut Option<usize>, progress: P) -> std::io::Result<()>
    where
        P: gix::NestedProgress,
        P::SubProgress: 'static,
    {
        if let Some(skipped) = skipped.take() {
            if skipped != 0 {
                progress.info(format!(
                    "\tskipped {skipped} tags known to the remote without bearing on this commit-graph."
                ));
            }
        }
        Ok(())
    }
    for (update, mapping, spec, edit) in updates {
        if mapping.spec_index != last_spec_index {
            last_spec_index = mapping.spec_index;
            consume_skipped_tags(&mut skipped_due_to_implicit_tag, &mut progress)?;

            progress.info(format!(
                "{}{}",
                spec.to_ref().to_bstring(),
                match mapping.spec_index.implicit_index() {
                    Some(_) => {
                        if spec.to_ref()
                            == gix::remote::fetch::Tags::Included
                                .to_refspec()
                                .expect("always yields refspec")
                        {
                            skipped_due_to_implicit_tag = Some(0);
                            " (implicit, due to auto-tag)"
                        } else {
                            " (implicit)"
                        }
                    }
                    None => "",
                }
            ));
        }

        if let Some(num_skipped) = skipped_due_to_implicit_tag.as_mut() {
            if let gix::remote::fetch::refs::update::Mode::NoChangeNeeded = update.mode {
                *num_skipped += 1;
                continue;
            }
        }

        let mode_and_type = update.type_change.map_or_else(
            || format!("{}", update.mode),
            |type_change| {
                format!(
                    "{} ({})",
                    update.mode,
                    match type_change {
                        TypeChange::DirectToSymbolic => {
                            "direct ref overwrites symbolic"
                        }
                        TypeChange::SymbolicToDirect => {
                            "symbolic ref overwrites direct"
                        }
                    }
                )
            },
        );
        progress.info(format!(
            "\t{}{}",
            match &mapping.remote {
                gix::remote::fetch::refmap::Source::ObjectId(id) => {
                    id.attach(repo).shorten_or_id().to_string()
                }
                gix::remote::fetch::refmap::Source::Ref(r) => git::remote::ref_to_string(r),
            },
            match edit {
                Some(edit) => format!(" -> {} [{mode_and_type}]", edit.name),
                None => format!(" [{mode_and_type}]"),
            }
        ));
    }

    consume_skipped_tags(&mut skipped_due_to_implicit_tag, &mut progress)?;

    if !map.fixes.is_empty() {
        progress.info(
            "The following destination refs were removed as they didn't start with 'ref/'"
                .to_string(),
        );

        map.fixes.sort_by(|l, r| match (l, r) {
            (
                Fix::MappingWithPartialDestinationRemoved { spec: l, .. },
                Fix::MappingWithPartialDestinationRemoved { spec: r, .. },
            ) => l.cmp(r),
        });
        let mut prev_spec = None;
        for fix in &map.fixes {
            match fix {
                Fix::MappingWithPartialDestinationRemoved { name, spec } => {
                    if prev_spec.is_some_and(|prev_spec| prev_spec != spec) {
                        prev_spec = spec.into();
                        progress.info(spec.to_ref().to_bstring().to_string());
                    }

                    progress.info(format!("\t{name}"));
                }
            }
        }
    }
    if map.remote_refs.len() - map.mappings.len() != 0 {
        progress.info(format!(
            "server sent {} tips, {} were filtered due to {} refspec(s).",
            map.remote_refs.len(),
            map.remote_refs.len() - map.mappings.len(),
            refspecs.len()
        ));
    }

    let rounds = negotiate.rounds.len();

    if rounds == 1 {
        return Ok(());
    }

    progress.done(match negotiate.rounds.len() {
        0 => "no negotiation was necessary".to_string(),
        rounds => format!("needed {rounds} rounds of pack-negotiation"),
    });

    Ok(())
}
