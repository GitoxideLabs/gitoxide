use std::{
    collections::{BTreeSet, VecDeque},
    io::{self, Seek},
    sync::atomic::{AtomicBool, Ordering},
};

use gix_object::{Exists, FindExt, Kind};

use crate::{
    ObjectId, Remote, Repository, Result,
    error::{ResultExt, bail, message},
    remote::{Direction, fetch, local},
};

impl Remote<'_> {
    /// Fetch configured refs, reading local repositories directly without launching Git.
    ///
    /// Network URLs use [`Self::connect()`] and [`fetch::Prepare::receive()`]. Local fetches
    /// transfer verified raw objects in a self-contained pack, obey tag and ref visibility
    /// policies, and protect reference updates with transactions. Replacement refs do not
    /// affect transferred objects. Neither path writes `FETCH_HEAD` nor runs maintenance.
    ///
    /// Native local fetch currently supports preserving the shallow boundary and setting an
    /// absolute depth. Shallow source boundaries are preserved, as with `git fetch --update-shallow`;
    /// `clone.rejectShallow` can prohibit such sources. Other shallow policies return an error.
    pub fn fetch<P>(
        &self,
        options: fetch::Options,
        progress: P,
        should_interrupt: &AtomicBool,
    ) -> Result<fetch::Outcome>
    where
        P: gix_utils::progress::NestedProgress,
        P::SubProgress: 'static,
    {
        self.fetch_with_remote_progress(options, progress, |_, _| Ok(()), should_interrupt)
    }

    /// Like [`Self::fetch()`], additionally forwarding raw server sideband packets.
    ///
    /// The callback receives the error flag and unchanged packet bytes. Local fetches have no
    /// server sideband; network fetches retain the behavior of [`fetch::Prepare::receive_with_remote_progress()`].
    pub fn fetch_with_remote_progress<P>(
        &self,
        options: fetch::Options,
        mut progress: P,
        remote_progress: impl FnMut(bool, &[u8]) -> std::io::Result<()>,
        should_interrupt: &AtomicBool,
    ) -> Result<fetch::Outcome>
    where
        P: gix_utils::progress::NestedProgress,
        P::SubProgress: 'static,
    {
        if let Some(prepare) = PrepareLocal::new(self, options.ref_map.clone())? {
            return prepare.receive(self.repo, options, progress, should_interrupt);
        }
        let mut prepare = self
            .connect(Direction::Fetch)?
            .prepare_fetch(&mut progress, options.ref_map)?
            .with_dry_run(options.dry_run)
            .with_prune(options.prune)
            .with_missing_refs_allowed(options.missing_refs_allowed)
            .with_write_packed_refs_only(options.write_packed_refs_only)
            .with_shallow(options.shallow);
        if let Some(message) = options.reflog_message {
            prepare = prepare.with_reflog_message(message);
        }
        prepare.receive_with_remote_progress(progress, remote_progress, should_interrupt)
    }
}

/// Native local fetch preparation with a fixed advertisement and no destination borrow.
pub(crate) struct PrepareLocal {
    remote: crate::remote::connection::RemoteDetached,
    source: Repository,
    pub(crate) ref_map: fetch::RefMap,
    handshake: gix_protocol::Handshake,
}

impl PrepareLocal {
    pub(crate) fn new(remote: &Remote<'_>, options: crate::remote::ref_map::Options) -> Result<Option<Self>> {
        let Some(source) = local::open(remote)? else {
            return Ok(None);
        };
        let (ref_map, handshake) = local::ref_map(remote, &source, options)?;
        Ok(Some(Self {
            remote: remote.detached(),
            source,
            ref_map,
            handshake,
        }))
    }

    pub(crate) fn receive<P>(
        self,
        repo: &Repository,
        options: fetch::Options,
        progress: P,
        interrupt: &AtomicBool,
    ) -> Result<fetch::Outcome>
    where
        P: gix_utils::progress::NestedProgress,
        P::SubProgress: 'static,
    {
        receive(
            &self.remote.attach_for_fetch(repo),
            &self.source,
            self.ref_map,
            self.handshake,
            options,
            progress,
            interrupt,
        )
    }
}

fn receive<P>(
    remote: &Remote<'_>,
    source: &Repository,
    ref_map: fetch::RefMap,
    handshake: gix_protocol::Handshake,
    options: fetch::Options,
    mut progress: P,
    interrupt: &AtomicBool,
) -> Result<fetch::Outcome>
where
    P: gix_utils::progress::NestedProgress,
    P::SubProgress: 'static,
{
    check_interrupt(interrupt)?;
    if source.object_hash() != remote.repo.object_hash() {
        bail!(message("Source and destination repositories use different object formats").unsupported());
    }
    if !matches!(
        options.shallow,
        fetch::Shallow::NoChange | fetch::Shallow::DepthAtRemote(_)
    ) {
        bail!(message("Native local fetch supports only unchanged or absolute shallow depths").unsupported());
    }
    let mut destination = remote.repo.clone();
    destination.make_paths_absolute()?;
    destination.objects.ignore_replacements = true;
    if ref_map.refspecs.is_empty() && ref_map.extra_refspecs.is_empty() {
        bail!(message("Cannot fetch without any refspecs").validation());
    }
    if !options.missing_refs_allowed && ref_map.is_missing_required_mapping() {
        bail!(message("None of the fetch refspecs matched a remote reference").validation());
    }
    let source_shallow: BTreeSet<_> = source
        .shallow_commits()?
        .map(|ids| ids.iter().copied().collect())
        .unwrap_or_default();
    if !source_shallow.is_empty()
        && destination
            .config_snapshot()
            .try_boolean("clone.rejectShallow")?
            .unwrap_or(false)
    {
        bail!(
            message("Receiving objects from shallow remotes is prohibited due to the value of `clone.rejectShallow`")
                .validation()
        );
    }
    let shallow_path = destination.shallow_file()?;
    let shallow_lock = if !options.dry_run
        && (!source_shallow.is_empty() || options.shallow != fetch::Shallow::NoChange)
    {
        Some(
            gix_lock::File::acquire_to_update_resource(&shallow_path, gix_lock::acquire::Fail::Immediately, None, 0)
                .or_raise(|| message("Could not lock the shallow file before fetching"))?,
        )
    } else {
        None
    };
    let previous_shallow = gix_shallow::read(&shallow_path)?;
    let previous: BTreeSet<_> = previous_shallow
        .as_ref()
        .map(|ids| ids.iter().copied().collect())
        .unwrap_or_default();
    let is_implicit_tag = |mapping: &fetch::refmap::Mapping| {
        remote.fetch_tags == fetch::Tags::Included
            && matches!(mapping.spec_index, fetch::refmap::SpecIndex::Implicit(_))
            && mapping
                .spec_index
                .get(&ref_map.refspecs, &ref_map.extra_refspecs)
                .is_some_and(|spec| Some(spec.to_ref()) == remote.fetch_tags.to_refspec())
    };
    let tips: Vec<_> = ref_map
        .mappings
        .iter()
        .filter(|mapping| !is_implicit_tag(mapping))
        .filter_map(|mapping| mapping.remote.as_id().map(ToOwned::to_owned))
        .collect();
    let (boundary, traversal_boundary) = boundary(
        source,
        &destination,
        &tips,
        &source_shallow,
        &previous,
        &options.shallow,
        interrupt,
    )?;
    let mut objects = collect_objects(source, tips.iter().copied(), &traversal_boundary, interrupt)?;
    for mapping in ref_map.mappings.iter().filter(|mapping| is_implicit_tag(mapping)) {
        let fetch::refmap::Source::Ref(reference) = &mapping.remote else {
            continue;
        };
        let (_, tag_id, peeled_id) = reference.unpack();
        if let Some(target_id) = peeled_id.or(tag_id)
            && (objects.contains(target_id) || destination.objects.exists(target_id))
            && let Some(tag_id) = tag_id
        {
            let mut object_id = tag_id.to_owned();
            let mut buffer = Vec::new();
            while objects.insert(object_id) {
                check_interrupt(interrupt)?;
                let data = source.find(&object_id, &mut buffer)?;
                data.verify_checksum(&object_id)?;
                if data.kind != Kind::Tag {
                    break;
                }
                for edge in data.references() {
                    object_id = edge?.id;
                }
            }
        }
    }
    let missing: Vec<_> = objects
        .into_iter()
        .filter(|id| !destination.objects.exists(id))
        .collect();
    let mut pack = if !options.dry_run && !missing.is_empty() {
        progress.set_name("receiving objects".into());
        Some(install_pack(source, &destination, &missing, progress, interrupt)?)
    } else {
        None
    };
    if let Some(lock) = shallow_lock {
        let updates: Vec<_> = previous
            .difference(&boundary)
            .copied()
            .map(gix_shallow::Update::Unshallow)
            .chain(
                boundary
                    .difference(&previous)
                    .copied()
                    .map(gix_shallow::Update::Shallow),
            )
            .collect();
        if !updates.is_empty() {
            gix_shallow::write(lock, previous_shallow, &updates)?;
        }
    }
    check_interrupt(interrupt)?;
    let pruned = if options.prune {
        destination.prune_fetch_refs(
            &ref_map,
            fetch::prune::Options {
                dry_run: options.dry_run,
            },
        )?
    } else {
        Vec::new()
    };
    let update_refs = fetch::refs::update(
        &destination,
        options
            .reflog_message
            .unwrap_or_else(|| fetch::RefLogMessage::Prefixed { action: "fetch".into() }),
        &ref_map.mappings,
        &ref_map.refspecs,
        &ref_map.extra_refspecs,
        remote.fetch_tags,
        if options.dry_run {
            fetch::DryRun::Yes
        } else {
            fetch::DryRun::No
        },
        if options.write_packed_refs_only {
            fetch::WritePackedRefs::Only
        } else {
            fetch::WritePackedRefs::Never
        },
    )?;
    if let Some(pack) = &mut pack
        && !update_refs.edits.is_empty()
        && let Some(keep_path) = pack.keep_path.take()
    {
        std::fs::remove_file(&keep_path)
            .or_raise(|| message!("Could not remove pack keep file {}", keep_path.display()))?;
    }
    Ok(fetch::Outcome {
        ref_map,
        handshake,
        pruned,
        status: match pack {
            Some(write_pack_bundle) => fetch::Status::Change {
                negotiate: Default::default(),
                write_pack_bundle,
                update_refs,
            },
            None => fetch::Status::NoPackReceived {
                dry_run: options.dry_run,
                negotiate: None,
                update_refs,
            },
        },
    })
}

fn boundary(
    source: &Repository,
    destination: &Repository,
    tips: &[ObjectId],
    source_shallow: &BTreeSet<ObjectId>,
    previous: &BTreeSet<ObjectId>,
    shallow: &fetch::Shallow,
    interrupt: &AtomicBool,
) -> Result<(BTreeSet<ObjectId>, BTreeSet<ObjectId>)> {
    let mut boundary = previous.clone();
    let mut traversal = source_shallow.clone();
    let depth = match shallow {
        fetch::Shallow::NoChange => None,
        fetch::Shallow::DepthAtRemote(depth) => Some(depth.get()),
        _ => unreachable!("validated by caller"),
    };
    if depth.is_none() {
        traversal.extend(previous);
    }
    let mut seen = BTreeSet::new();
    let mut pending: VecDeque<_> = tips.iter().map(|id| (*id, 1u32)).collect();
    let mut buffer = Vec::new();
    while let Some((object_id, distance)) = pending.pop_front() {
        check_interrupt(interrupt)?;
        if !seen.insert(object_id) {
            continue;
        }
        let data = source.find(&object_id, &mut buffer)?;
        data.verify_checksum(&object_id)?;
        // Depth depends on graph edges, not on historical identity metadata.
        match data.kind {
            Kind::Tag => {
                for edge in data.references() {
                    pending.push_front((edge?.id, distance));
                }
            }
            Kind::Commit => {
                let mut parents = Vec::new();
                for edge in data.references() {
                    let edge = edge?;
                    if edge.kind == Kind::Commit {
                        parents.push(edge.id);
                    }
                }
                if source_shallow.contains(&object_id) {
                    if !destination.objects.exists(&object_id) || previous.contains(&object_id) {
                        boundary.insert(object_id);
                    }
                } else if depth == Some(distance) {
                    boundary.insert(object_id);
                    traversal.insert(object_id);
                } else if depth.is_none() && previous.contains(&object_id) {
                    // Preserve the client's boundary even when the source has more history.
                } else {
                    boundary.remove(&object_id);
                    pending.extend(parents.into_iter().map(|id| (id, distance.saturating_add(1))));
                }
            }
            Kind::Tree | Kind::Blob => {}
        }
    }
    Ok((boundary, traversal))
}

fn collect_objects(
    source: &Repository,
    tips: impl IntoIterator<Item = ObjectId>,
    shallow: &BTreeSet<ObjectId>,
    interrupt: &AtomicBool,
) -> Result<BTreeSet<ObjectId>> {
    gix_traverse::object::BreadthFirst::new(tips, source)
        .with_object_validation(false)
        .with_shallow_commits(shallow.iter().copied())
        .map(|entry| {
            check_interrupt(interrupt)?;
            Ok(entry?.id)
        })
        .collect()
}

fn install_pack<P>(
    source: &Repository,
    destination: &Repository,
    objects: &[ObjectId],
    mut progress: P,
    interrupt: &AtomicBool,
) -> Result<gix_pack::bundle::write::Outcome>
where
    P: gix_utils::progress::NestedProgress,
    P::SubProgress: 'static,
{
    let directory = destination.objects.store_ref().path().join("pack");
    let mut file = gix_tempfile::new(
        &directory,
        gix_tempfile::ContainingDirectory::CreateAllRaceProof {
            retries: Default::default(),
            shared_repository_permissions: 0,
        },
        gix_tempfile::AutoRemove::Tempfile,
    )
    .or_raise(|| message("Could not create temporary fetch pack"))?;
    source.write_pack(objects.iter().copied(), &mut file, interrupt)?;
    file.rewind().or_error()?;
    gix_pack::Bundle::write_to_directory(
        &mut io::BufReader::new(file),
        Some(&directory),
        &mut progress,
        interrupt,
        None::<&Repository>,
        source.object_hash(),
        gix_pack::bundle::write::Options {
            thread_limit: super::config::index_threads(destination)?,
            index_version: super::config::pack_index_version(destination)?,
            iteration_mode: gix_pack::data::input::Mode::Verify,
            alloc_limit_bytes: destination.config.alloc_limit_bytes,
            compression: destination.config.loose_compression,
        },
    )
}

fn check_interrupt(interrupt: &AtomicBool) -> Result<()> {
    if interrupt.load(Ordering::Relaxed) {
        bail!(message("Fetch interrupted").cancelled());
    }
    Ok(())
}
