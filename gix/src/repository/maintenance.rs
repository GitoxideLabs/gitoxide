use std::{
    collections::BTreeSet,
    fs, io,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    ObjectId, Repository, Result,
    bstr::ByteSlice,
    error::{ResultExt, bail, message},
};

impl Repository {
    /// Collect object graph roots that repository maintenance must retain.
    ///
    /// Include all references without peeling tags, every worktree's HEAD, index entries,
    /// valid cached index trees, resolve-undo stages, and both sides of every reflog entry.
    /// Rebase autostashes and original heads are included for all worktrees, even when their
    /// checkout directories are unavailable. Reference namespaces and object replacements
    /// do not restrict this view. Missing objects mentioned only by reflogs are ignored,
    /// matching Git's treatment of reflogs that refer to already-pruned objects.
    ///
    /// The returned IDs are sorted and deduplicated. This only gathers roots: traverse their
    /// full graph with [`gix_traverse::object::BreadthFirst`], respecting shallow boundaries,
    /// before deleting any objects. Recent unreachable objects and their dependencies need
    /// protection in addition to these roots. No files or references are changed.
    ///
    /// Invalid references, indices, state files, or reflogs produce an error. This is a
    /// snapshot, not a lock against concurrent writers; callers must use suitable expiry
    /// protection or synchronization during maintenance.
    pub fn object_reachability_roots(&self, should_interrupt: &AtomicBool) -> Result<Vec<ObjectId>> {
        let mut roots = BTreeSet::new();
        check_interrupt(should_interrupt)?;
        for repo in self.worktrees_including_main()? {
            check_interrupt(should_interrupt)?;
            let mut repo = repo?;
            repo.clear_namespace();
            repo.objects.ignore_replacements = true;
            for reference in repo.references()?.all()? {
                check_interrupt(should_interrupt)?;
                let reference = reference?;
                // Symbolic referents are included by the same iteration, or by HEAD handling below.
                if let Some(object_id) = reference.try_id() {
                    roots.insert(object_id.detach());
                }
            }
            match repo.head()?.kind {
                crate::head::Kind::Detached { target, .. } => {
                    roots.insert(target);
                }
                crate::head::Kind::Symbolic(reference) => {
                    if let Some(object_id) = reference.target.try_id() {
                        roots.insert(object_id.to_owned());
                    }
                }
                crate::head::Kind::Unborn(_) => {}
            }
            collect_index(&repo, &repo.index_path(), &mut roots)?;
            let physical_index = repo.git_dir().join("index");
            if repo.index_path() != physical_index {
                collect_index(&repo, &physical_index, &mut roots)?;
            }
            collect_reflogs(&repo, &repo.git_dir().join("logs"), &mut roots, should_interrupt)?;
            for name in [
                "rebase-apply/autostash",
                "rebase-apply/orig-head",
                "rebase-merge/autostash",
                "rebase-merge/orig-head",
            ] {
                let path = repo.git_dir().join(name);
                let bytes = match fs::read(&path) {
                    Ok(bytes) => bytes,
                    Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                    Err(err) => return Err(err).or_raise(|| message!("Could not read {}", path.display())),
                };
                let bytes = bytes.trim();
                if !bytes.is_empty() {
                    let object_id =
                        ObjectId::from_hex(bytes).or_raise(|| message!("Invalid object ID in {}", path.display()))?;
                    if object_id.kind() != repo.object_hash() || object_id.is_null() {
                        bail!(message!("Invalid object ID in {}", path.display()).validation());
                    }
                    roots.insert(object_id);
                }
            }
        }
        Ok(roots.into_iter().collect())
    }
}

fn collect_index(repo: &Repository, path: &Path, roots: &mut BTreeSet<ObjectId>) -> Result<()> {
    let index = match gix_index::File::at(
        path,
        repo.object_hash(),
        false,
        gix_index::decode::Options {
            alloc_limit_bytes: repo.config.alloc_limit_bytes,
            ..Default::default()
        },
    ) {
        Ok(index) => index,
        Err(err)
            if err
                .downcast_any_ref::<gix_index::file::init::OpenError>()
                .is_some_and(|err| err.path == path && err.source.kind() == io::ErrorKind::NotFound) =>
        {
            return Ok(());
        }
        Err(err) => return Err(err),
    };
    for entry in index.entries() {
        if !entry.mode.is_submodule() && !entry.id.is_null() {
            roots.insert(entry.id);
        }
    }
    let mut trees: Vec<_> = index.tree().into_iter().collect();
    while let Some(tree) = trees.pop() {
        if tree.num_entries.is_some() && !tree.id.is_null() {
            roots.insert(tree.id);
        }
        trees.extend(&tree.children);
    }
    for path in index.resolve_undo().into_iter().flatten() {
        for stage in path.stages.iter().flatten() {
            // Retain symbolic-link blobs too. Gitlinks name objects in another repository.
            if stage.mode & 0o170000 != 0o160000 && !stage.id.is_null() {
                roots.insert(stage.id);
            }
        }
    }
    Ok(())
}

fn collect_reflogs(
    repo: &Repository,
    directory: &Path,
    roots: &mut BTreeSet<ObjectId>,
    should_interrupt: &AtomicBool,
) -> Result<()> {
    let mut directories = vec![directory.to_owned()];
    while let Some(directory) = directories.pop() {
        check_interrupt(should_interrupt)?;
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(err).or_raise(|| message!("Could not enumerate reflogs in {}", directory.display()));
            }
        };
        for entry in entries {
            check_interrupt(should_interrupt)?;
            let entry = entry.or_raise(|| message!("Could not enumerate reflogs in {}", directory.display()))?;
            let kind = entry
                .file_type()
                .or_raise(|| message!("Could not inspect {}", entry.path().display()))?;
            if kind.is_dir() {
                directories.push(entry.path());
                continue;
            }
            if !kind.is_file() {
                bail!(message!("Expected a regular reflog file at {}", entry.path().display()).validation());
            }
            let bytes =
                fs::read(entry.path()).or_raise(|| message!("Could not read reflog {}", entry.path().display()))?;
            for line in gix_ref::file::log::iter::forward(&bytes) {
                check_interrupt(should_interrupt)?;
                let line = line.or_raise(|| message!("Could not decode reflog {}", entry.path().display()))?;
                for object_id in [line.previous_oid(), line.new_oid()] {
                    if object_id.is_null() || roots.contains(&object_id) {
                        continue;
                    }
                    if object_id.kind() != repo.object_hash() {
                        bail!(message!("Wrong object format in reflog {}", entry.path().display()).validation());
                    }
                    if repo.try_find_object(object_id)?.is_some() {
                        roots.insert(object_id);
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_interrupt(should_interrupt: &AtomicBool) -> Result<()> {
    if should_interrupt.load(Ordering::Relaxed) {
        bail!(message("Collecting object reachability roots interrupted").cancelled());
    }
    Ok(())
}
