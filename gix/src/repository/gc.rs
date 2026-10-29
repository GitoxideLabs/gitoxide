//! Repack local objects and expire unreachable objects without changing references.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    io::{BufRead, Seek},
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime},
};

use crate::{
    ObjectId, Repository, Result,
    bstr::ByteSlice,
    error::{ResultExt, bail, message},
};

/// Settings for [`Repository::collect_garbage()`].
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Preserve unreachable objects newer than this time, together with their dependencies.
    ///
    /// A grace period protects concurrent writers. Using the current time or a future time
    /// requires external coordination with every process that may write objects or references.
    pub expire: SystemTime,
    /// Compute object retention without writing packs or removing objects.
    pub dry_run: bool,
}

/// Object retention and completed maintenance work.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Number of distinct local objects observed before packing.
    pub objects_before: usize,
    /// Local objects reachable from repository roots or explicitly kept packs.
    pub reachable_objects: usize,
    /// Unreachable local objects retained by age or dependencies of recent objects.
    pub retained_unreachable_objects: usize,
    /// Distinct local objects eligible for expiration.
    pub expired_objects: usize,
    /// Packs installed, excluding packs that already had identical contents.
    pub written_packs: usize,
    /// Old pack files removed after replacements were installed.
    pub removed_packs: usize,
    /// Loose object files removed, including redundant packed copies.
    pub removed_loose_objects: usize,
}

/// Results of [`Repository::gc()`], including reference and reflog maintenance.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MaintenanceOutcome {
    /// Object retention and repacking results.
    pub objects: Outcome,
    /// Reflog entries retained or expired before collecting objects.
    pub reflogs: crate::repository::reflog::Outcome,
    /// Reference packing results, absent when `gc.packRefs` disables packing.
    pub references: Option<crate::repository::pack_refs::Outcome>,
}

#[derive(Default)]
struct Inventory {
    times: BTreeMap<ObjectId, SystemTime>,
    loose: BTreeSet<ObjectId>,
    kept: BTreeSet<ObjectId>,
    packs: Vec<Pack>,
}

struct Pack {
    path: PathBuf,
    metadata: fs::Metadata,
    keep: bool,
}

impl Repository {
    /// Pack local objects and remove expired unreachable objects.
    ///
    /// Retain the complete graph of repository references, reflogs, every worktree's index and
    /// HEAD, and operation recovery roots. Also retain objects newer than `options.expire`,
    /// objects named by trusted `gc.recentObjectsHook` commands, and their older dependencies.
    /// Kept packs and their dependencies are protected. Alternates are read but never modified.
    /// Reference namespaces and replacement refs do not limit maintenance.
    ///
    /// Reachable objects are packed separately from retained unreachable objects. The latter
    /// use Git-compatible cruft packs with `.mtimes` tables, preserving their original ages.
    /// Pack and index checksums are validated before their contents influence deletion. New
    /// packs are installed before old packs or redundant loose objects are removed. Objects
    /// first observed after the initial snapshot and packs with changed metadata are retained.
    ///
    /// This operation holds `gc.pid.lock` and honors recent `gc.pid` files to exclude other
    /// collectors, but does not lock out ordinary writers. Choose an appropriate grace period.
    /// Errors or cancellation may leave completed packs or partially completed cleanup; object
    /// deletion starts only after all required packs and timestamps have been installed.
    ///
    /// Reflogs and references are not expired or packed by this operation. Precious object stores
    /// and partial clones currently reject collection rather than risk losing protected objects.
    pub fn collect_garbage(&self, options: Options, should_interrupt: &AtomicBool) -> Result<Outcome> {
        Ok(self.run_gc(options, false, should_interrupt)?.objects)
    }

    /// Expire reflogs, compact shared references, and collect unreachable objects.
    ///
    /// Hold the garbage-collection lock for the entire operation. Reference packing
    /// honors `gc.packRefs` (true by default, or `notbare`), and reflog expiry uses
    /// the configured age and reachability policies. Object retention is computed
    /// after expired reflogs have been removed. See [`Self::collect_garbage()`] for
    /// object safety, grace periods, and limitations, and [`Self::expire_reflogs()`]
    /// for reflog policy.
    ///
    /// A dry run writes nothing. Its object counts conservatively include objects
    /// protected by reflog entries selected for expiry, since those entries remain
    /// present during the simulation. Errors can leave earlier maintenance stages
    /// completed; every stage preserves live references and objects.
    pub fn gc(&self, options: Options, should_interrupt: &AtomicBool) -> Result<MaintenanceOutcome> {
        self.run_gc(options, true, should_interrupt)
    }

    fn run_gc(&self, options: Options, full: bool, should_interrupt: &AtomicBool) -> Result<MaintenanceOutcome> {
        check_interrupt(should_interrupt)?;
        let mut repo = self.clone();
        repo.make_paths_absolute()?;
        repo.clear_namespace();
        repo.objects.ignore_replacements = true;
        if repo.config_snapshot().try_boolean("extensions.preciousObjects")? == Some(true) {
            bail!(message("Cannot collect a repository with precious objects").unsupported());
        }
        if repo.config.resolved.string("extensions.partialClone").is_some() {
            bail!(message("Cannot collect partial-clone promisor objects").unsupported());
        }
        for section in repo.config.resolved.sections() {
            if section.header().name().eq_ignore_ascii_case(b"remote")
                && repo
                    .config
                    .resolved
                    .boolean_by("remote", section.header().subsection_name(), "promisor")?
                    == Some(true)
            {
                bail!(message("Cannot collect partial-clone promisor objects").unsupported());
            }
        }
        let _gc_lock = gix_lock::Marker::acquire_to_hold_resource(
            repo.common_dir().join("gc.pid"),
            gix_lock::acquire::Fail::Immediately,
            None,
            0,
        )?;
        if let Some(metadata) = regular_file(&repo.common_dir().join("gc.pid"))?
            && SystemTime::now()
                .duration_since(metadata.modified().or_error()?)
                .unwrap_or_default()
                <= Duration::from_secs(12 * 60 * 60)
        {
            bail!(message("Another garbage collector may be running (gc.pid exists)").retryable());
        }
        let objects_directory = repo.objects.store_ref().path();
        let inventory = inventory(&repo, objects_directory, should_interrupt)?;
        let mut maintenance = MaintenanceOutcome::default();
        if full {
            let config = repo.config_snapshot();
            let pack_refs = if config.string("gc.packRefs").is_some_and(|value| value == "notbare") {
                !repo.is_bare()
            } else {
                config.try_boolean("gc.packRefs")?.unwrap_or(true)
            };
            if pack_refs {
                maintenance.references = Some(repo.pack_references(
                    crate::repository::pack_refs::Options {
                        all: true,
                        dry_run: options.dry_run,
                        ..Default::default()
                    },
                    should_interrupt,
                )?);
            }
            maintenance.reflogs = repo.expire_reflogs(
                crate::repository::reflog::Options {
                    dry_run: options.dry_run,
                    ..Default::default()
                },
                should_interrupt,
            )?;
        }
        let shallow: Vec<_> = repo
            .shallow_commits()?
            .map(|ids| ids.iter().copied().collect())
            .unwrap_or_default();
        let roots = repo.object_reachability_roots(should_interrupt)?;
        let mut reachable = BTreeSet::new();
        for entry in
            gix_traverse::object::BreadthFirst::new(roots.into_iter().chain(inventory.kept.iter().copied()), &repo)
                .with_shallow_commits(shallow.iter().copied())
        {
            check_interrupt(should_interrupt)?;
            reachable.insert(entry?.id);
        }
        let hook_objects = recent_hook_objects(&repo, should_interrupt)?;
        let mut recent: BTreeSet<_> = inventory
            .times
            .iter()
            .filter_map(|(&object_id, &time)| {
                (!reachable.contains(&object_id) && time > options.expire).then_some(object_id)
            })
            .collect();
        // Hooks can introduce objects after the initial inventory. Retain their graphs too,
        // but ignore names that still do not exist, as Git does.
        for object_id in hook_objects {
            if !reachable.contains(&object_id) && repo.try_find_object(object_id)?.is_some() {
                recent.insert(object_id);
            }
        }
        let mut retained = reachable.clone();
        for entry in gix_traverse::object::BreadthFirst::new(recent, &repo)
            .with_shallow_commits(shallow)
            .with_missing_objects_ignored(true)
        {
            check_interrupt(should_interrupt)?;
            retained.insert(entry?.id);
        }
        let live: Vec<_> = reachable
            .iter()
            .copied()
            .filter(|id| {
                !inventory.kept.contains(id)
                    && (inventory.times.contains_key(id) || *id == repo.object_hash().empty_tree())
            })
            .collect();
        let cruft: Vec<_> = inventory
            .times
            .keys()
            .copied()
            .filter(|id| retained.contains(id) && !reachable.contains(id))
            .collect();
        let mut outcome = Outcome {
            objects_before: inventory.times.len(),
            reachable_objects: inventory.times.keys().filter(|id| reachable.contains(*id)).count(),
            retained_unreachable_objects: cruft.len(),
            expired_objects: inventory.times.keys().filter(|id| !retained.contains(*id)).count(),
            ..Outcome::default()
        };
        if options.dry_run {
            maintenance.objects = outcome;
            return Ok(maintenance);
        }
        check_interrupt(should_interrupt)?;
        let pack_directory = objects_directory.join("pack");
        fs::create_dir_all(&pack_directory).or_raise(|| message("Could not create pack directory"))?;
        let mut installed = Vec::new();
        if !live.is_empty() {
            installed.push(install_pack(&repo, &pack_directory, &live, None, should_interrupt)?);
        }
        if !cruft.is_empty() {
            installed.push(install_pack(
                &repo,
                &pack_directory,
                &cruft,
                Some(&inventory.times),
                should_interrupt,
            )?);
        }
        outcome.written_packs = installed.iter().filter(|pack| pack.keep_path.is_some()).count();
        let installed_paths: BTreeSet<_> = installed.iter().filter_map(|pack| pack.data_path.as_deref()).collect();
        // Removing an index before its packs lets future readers fall back to the individual indices.
        let _midx_lock = gix_lock::Marker::acquire_to_hold_resource(
            pack_directory.join("multi-pack-index"),
            gix_lock::acquire::Fail::Immediately,
            None,
            0,
        )?;
        for name in ["multi-pack-index", "multi-pack-index.d/multi-pack-index-chain"] {
            remove_regular_file(&pack_directory.join(name))?;
        }
        for pack in &inventory.packs {
            check_interrupt(should_interrupt)?;
            if pack.keep
                || installed_paths.contains(pack.path.as_path())
                || pack.path.with_extension("keep").try_exists().or_error()?
            {
                continue;
            }
            let Some(metadata) = regular_file(&pack.path)? else {
                continue;
            };
            if metadata.len() != pack.metadata.len()
                || metadata.modified().or_error()? != pack.metadata.modified().or_error()?
            {
                continue;
            }
            // Readers can still discover every retained object in the replacement indices.
            remove_regular_file(&pack.path.with_extension("idx"))?;
            remove_regular_file(&pack.path)?;
            for extension in ["rev", "bitmap", "mtimes"] {
                remove_regular_file(&pack.path.with_extension(extension))?;
            }
            outcome.removed_packs += 1;
        }
        let loose = gix_odb::loose::Store::at(objects_directory, repo.object_hash());
        let pruned = loose.prune(
            |object_id| !inventory.loose.contains(object_id),
            gix_odb::loose::prune::Options {
                expire: options.expire,
                dry_run: false,
            },
            should_interrupt,
        )?;
        outcome.removed_loose_objects = pruned.pruned_objects;
        // Keep files created by this operation are relinquished only after every cleanup step succeeds.
        for pack in installed {
            if let Some(path) = pack.keep_path {
                remove_regular_file(&path)?;
            }
        }
        maintenance.objects = outcome;
        Ok(maintenance)
    }
}

fn inventory(repo: &Repository, directory: &Path, interrupt: &AtomicBool) -> Result<Inventory> {
    let mut out = Inventory::default();
    let loose = gix_odb::loose::Store::at(directory, repo.object_hash());
    for object_id in loose.iter() {
        check_interrupt(interrupt)?;
        let object_id = object_id.or_raise(|| message("Could not enumerate loose objects"))?;
        let path = loose.object_path(&object_id);
        let Some(metadata) = regular_file(&path)? else { continue };
        out.times.insert(object_id, metadata.modified().or_error()?);
        out.loose.insert(object_id);
    }
    let pack_directory = directory.join("pack");
    let entries = match fs::read_dir(&pack_directory) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(out),
        Err(err) => return Err(err).or_raise(|| message("Could not enumerate packs")),
    };
    for entry in entries {
        check_interrupt(interrupt)?;
        let path = entry.or_error()?.path();
        if path.extension().is_none_or(|ext| ext != "idx") || regular_file(&path)?.is_none() {
            continue;
        }
        let pack_path = path.with_extension("pack");
        let Some(metadata) = regular_file(&pack_path)? else {
            continue;
        };
        if pack_path.with_extension("promisor").try_exists().or_error()? {
            bail!(message("Cannot collect partial-clone promisor objects").unsupported());
        }
        let index = gix_pack::index::File::at(&path, repo.object_hash())?;
        index.verify_checksum(&mut crate::progress::Discard, interrupt)?;
        let pack = gix_pack::data::File::at(&pack_path, repo.object_hash())?;
        let pack_hash = pack.verify_checksum(&mut crate::progress::Discard, interrupt)?;
        pack_hash.verify(&index.pack_checksum())?;
        let mtimes_path = path.with_extension("mtimes");
        let mtimes = match fs::read(&mtimes_path) {
            Ok(bytes) => Some(bytes),
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(err) => return Err(err).or_raise(|| message!("Could not read {}", mtimes_path.display())),
        };
        let times = mtimes
            .as_deref()
            .map(|data| gix_pack::mtimes::Table::from_bytes(data, index.num_objects(), &pack_hash))
            .transpose()?;
        let keep = pack_path.with_extension("keep").try_exists().or_error()?;
        for (position, entry) in index.iter().enumerate() {
            check_interrupt(interrupt)?;
            let time = match times.as_ref() {
                Some(times) => {
                    SystemTime::UNIX_EPOCH
                        + Duration::from_secs(u64::from(
                            times.get(position as u32).expect("table has one time per index entry"),
                        ))
                }
                None => metadata.modified().or_error()?,
            };
            out.times
                .entry(entry.oid)
                .and_modify(|prior| *prior = (*prior).max(time))
                .or_insert(time);
            if keep {
                out.kept.insert(entry.oid);
            }
        }
        out.packs.push(Pack {
            path: pack_path,
            metadata,
            keep,
        });
    }
    Ok(out)
}

fn install_pack(
    repo: &Repository,
    directory: &Path,
    object_ids: &[ObjectId],
    times: Option<&BTreeMap<ObjectId, SystemTime>>,
    interrupt: &AtomicBool,
) -> Result<gix_pack::bundle::write::Outcome> {
    let mut file = gix_tempfile::new(
        directory,
        gix_tempfile::ContainingDirectory::Exists,
        gix_tempfile::AutoRemove::Tempfile,
    )
    .or_raise(|| message("Could not create temporary generated pack"))?;
    repo.write_pack(
        object_ids.iter().copied(),
        &mut file,
        crate::progress::Discard,
        interrupt,
    )?;
    file.rewind().or_error()?;
    let outcome = gix_pack::Bundle::write_to_directory(
        &mut io::BufReader::new(file),
        Some(directory),
        &mut crate::progress::Discard,
        interrupt,
        None::<&Repository>,
        repo.object_hash(),
        gix_pack::bundle::write::Options {
            alloc_limit_bytes: repo.config.alloc_limit_bytes,
            ..Default::default()
        },
    )?;
    let index_path = outcome.index_path.as_ref().expect("nonempty pack has an index");
    if let Some(times) = times {
        let index = gix_pack::index::File::at(index_path, repo.object_hash())?;
        let times: Vec<_> = index
            .iter()
            .map(|entry| {
                let seconds = times[&entry.oid]
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                u32::try_from(seconds)
                    .or_raise(|| message("Cruft timestamps beyond the year 2106 cannot be represented"))
            })
            .collect::<Result<_>>()?;
        let mut lock = gix_lock::File::acquire_to_update_resource(
            index_path.with_extension("mtimes"),
            gix_lock::acquire::Fail::Immediately,
            None,
            0,
        )?;
        gix_pack::mtimes::write(&times, &index.pack_checksum(), &mut lock)?;
        lock.commit()
            .or_raise(|| message("Could not install cruft timestamps"))?;
    } else {
        // An identical pack may previously have contained unreachable objects.
        remove_regular_file(&index_path.with_extension("mtimes"))?;
    }
    Ok(outcome)
}

fn recent_hook_objects(repo: &Repository, interrupt: &AtomicBool) -> Result<BTreeSet<ObjectId>> {
    let mut out = BTreeSet::new();
    for program in repo
        .config
        .resolved
        .strings_filter("gc.recentObjectsHook", repo.filter_config_section())
        .unwrap_or_default()
    {
        check_interrupt(interrupt)?;
        let program = gix_path::from_bstr(program.as_bstr())?.into_owned();
        let mut context = repo.command_context()?;
        context.no_replace_objects = Some(true);
        context.ref_namespace = None;
        let mut command: std::process::Command = gix_command::prepare(program)
            .with_shell()
            .with_context(context)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .try_into()?;
        command.current_dir(repo.workdir().unwrap_or_else(|| repo.git_dir()));
        let mut child = command
            .spawn()
            .or_raise(|| message("Could not start gc.recentObjectsHook"))?;
        let result: Result<()> = (|| {
            let stdout = child.stdout.take().expect("stdout was piped");
            for line in io::BufReader::new(stdout).split(b'\n') {
                check_interrupt(interrupt)?;
                let mut line = line.or_raise(|| message("Could not read gc.recentObjectsHook output"))?;
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                let object_id =
                    ObjectId::from_hex(&line).or_raise(|| message("Invalid object ID from gc.recentObjectsHook"))?;
                if object_id.kind() != repo.object_hash() {
                    bail!(message("Wrong object format from gc.recentObjectsHook").validation());
                }
                out.insert(object_id);
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = child.kill();
        }
        let status = child
            .wait()
            .or_raise(|| message("Could not wait for gc.recentObjectsHook"))?;
        result?;
        if !status.success() {
            bail!(message!("gc.recentObjectsHook failed with {status}"));
        }
    }
    Ok(out)
}

fn regular_file(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(Some(metadata)),
        Ok(_) => bail!(message!("Expected a regular file at {}", path.display()).validation()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).or_raise(|| message!("Could not inspect {}", path.display())),
    }
}

fn remove_regular_file(path: &Path) -> Result<()> {
    if regular_file(path)?.is_some() {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err).or_raise(|| message!("Could not remove {}", path.display())),
        }
    }
    Ok(())
}

fn check_interrupt(interrupt: &AtomicBool) -> Result<()> {
    if interrupt.load(Ordering::Relaxed) {
        bail!(message("Object garbage collection interrupted").cancelled());
    }
    Ok(())
}
