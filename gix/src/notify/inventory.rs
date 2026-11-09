use std::{
    collections::{BTreeMap, HashSet},
    hash::{DefaultHasher, Hasher},
    path::{Path, PathBuf},
};

use gix_error::{ErrorExt, Result, ResultExt, message};
use gix_hashtable::hash_table::HashTable;

use gix_notify::Watch;

use super::{DependencyKind, Layout};
use crate::Repository;

pub(super) fn directory_lookup(paths: impl IntoIterator<Item = PathBuf>) -> HashTable<PathBuf> {
    let paths = paths.into_iter();
    let mut out = HashTable::with_capacity(paths.size_hint().0);
    for path in paths {
        out.insert_unique(path_hash(&path), path, |path| path_hash(path));
    }
    out
}

pub(super) fn path_hash(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    // Like the index's icase lookup, fold hashes but resolve collisions using the current case rules.
    for component in path.components() {
        let bytes = component.as_os_str().as_encoded_bytes();
        hasher.write_usize(bytes.len());
        for byte in bytes {
            // Windows prefix components retain raw separators despite comparing by parsed prefix.
            let byte = if cfg!(windows) && *byte == b'\\' {
                b'/'
            } else {
                byte.to_ascii_lowercase()
            };
            hasher.write_u8(byte);
        }
    }
    hasher.finish()
}

pub(super) fn index_fingerprint(index: &gix_index::State) -> Result<gix_hash::ObjectId> {
    use gix_index::entry::Flags;

    // Ordinary staging does not change directory coverage. Trackedness is checked only at ignore boundaries.
    let mut hash = gix_hash::hasher(gix_hash::Kind::longest());
    for entry in index.entries() {
        let path = entry.path(index);
        let is_ignore = path
            .rsplit(|byte| *byte == b'/')
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case(b".gitignore"));
        if !is_ignore && !matches!(entry.mode, gix_index::entry::Mode::COMMIT | gix_index::entry::Mode::DIR) {
            continue;
        }
        let mut metadata = [0; 17];
        metadata[..8].copy_from_slice(&(path.len() as u64).to_le_bytes());
        metadata[8..12].copy_from_slice(&entry.mode.bits().to_le_bytes());
        metadata[12..16].copy_from_slice(
            &(entry.flags & (Flags::SKIP_WORKTREE | Flags::STAGE_MASK | Flags::ASSUME_VALID | Flags::UPTODATE))
                .bits()
                .to_le_bytes(),
        );
        metadata[16] = if is_ignore { entry.id.as_slice().len() as u8 } else { 0 };
        hash.update(&metadata);
        hash.update(path);
        // Ignore rules may come from the index; ordinary blob IDs and stat data don't affect coverage.
        if is_ignore {
            hash.update(entry.id.as_slice());
        }
    }
    hash.try_finalize()
}

fn watch_directory(layout: &Layout, watches: &mut BTreeMap<PathBuf, bool>, path: &Path, recursive: bool) {
    let mut candidate = path;
    let mut recursive = recursive;
    while layout.contains(candidate) {
        if candidate.is_dir() {
            watches
                .entry(candidate.to_owned())
                .and_modify(|value| *value |= recursive)
                .or_insert(recursive);
            return;
        }
        let Some(parent) = candidate.parent() else { return };
        candidate = parent;
        recursive = false;
    }
}

pub(super) fn watch_parent(layout: &Layout, watches: &mut BTreeMap<PathBuf, bool>, path: &Path) {
    if let Some(parent) = path.parent() {
        if layout.contains(path) {
            watch_directory(layout, watches, parent, false);
        } else if layout
            .dependencies
            .iter()
            .any(|dependency| dependency.path == path && dependency.kind == DependencyKind::Ignore)
        {
            // Global ignore files can be replaced atomically or created below missing parents.
            if let Some(parent) = parent.ancestors().find(|parent| parent.is_dir()) {
                watches.entry(parent.to_owned()).or_insert(false);
            }
        }
    }
}

pub(super) fn metadata_watches(layout: &Layout) -> Result<BTreeMap<PathBuf, bool>> {
    let mut out = BTreeMap::new();
    for root in [&layout.git_dir, &layout.common_dir] {
        watch_directory(layout, &mut out, root, false);
        watch_directory(layout, &mut out, &root.join("refs"), true);
        // Keep both storage layouts observable across backend switches. Stacks are flat, and
        // watching their directory covers tables.list replacement and table compaction.
        watch_directory(layout, &mut out, &root.join("reftable"), false);
    }
    // Git removes these administrative entries even when the linked worktree is outside our roots.
    let worktrees = layout.common_dir.join("worktrees");
    watch_directory(layout, &mut out, &worktrees, false);
    match std::fs::read_dir(&worktrees) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.or_raise(|| message("could not read a linked-worktree administrative entry"))?;
                if entry
                    .file_type()
                    .or_raise(|| message!("could not inspect linked-worktree directory {}", entry.path().display()))?
                    .is_dir()
                {
                    // Also cover incomplete worktrees before their gitdir file is created.
                    watch_directory(layout, &mut out, &entry.path(), false);
                    // A table-backed HEAD never updates the loose HEAD placeholder.
                    watch_directory(layout, &mut out, &entry.path().join("reftable"), false);
                }
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.and_raise(message!("could not list linked worktrees in {}", worktrees.display()))),
    }
    merge_watches(&mut out, private_metadata_watches(layout));
    Ok(out)
}

pub(super) fn merge_watches(into: &mut BTreeMap<PathBuf, bool>, watches: BTreeMap<PathBuf, bool>) {
    for (path, recursive) in watches {
        into.entry(path)
            .and_modify(|mode| *mode |= recursive)
            .or_insert(recursive);
    }
}

/// Per-checkout metadata and dependencies, without rediscovering shared worktree membership.
pub(super) fn private_metadata_watches(layout: &Layout) -> BTreeMap<PathBuf, bool> {
    let mut out = BTreeMap::new();
    watch_directory(layout, &mut out, &layout.git_dir, false);
    watch_directory(layout, &mut out, &layout.git_dir.join("refs"), true);
    watch_directory(layout, &mut out, &layout.git_dir.join("reftable"), false);
    for name in ["rebase-apply", "rebase-merge", "sequencer"] {
        watch_directory(layout, &mut out, &layout.git_dir.join(name), true);
    }
    for dependency in &layout.dependencies {
        watch_parent(layout, &mut out, &dependency.path);
    }
    if let Some(root) = &layout.worktree {
        watch_directory(layout, &mut out, root, false);
    }
    out
}

/// Only the shallow administrative dependencies needed to keep ignore-aware coverage current.
pub(super) fn dependency_watches(layout: &Layout) -> BTreeMap<PathBuf, bool> {
    let mut out = BTreeMap::new();
    for root in [&layout.git_dir, &layout.common_dir] {
        watch_directory(layout, &mut out, root, false);
        // Mirror loose HEAD dependencies without reading refs or discovering other worktrees.
        watch_directory(layout, &mut out, &root.join("reftable"), false);
    }
    for dependency in &layout.dependencies {
        if dependency.kind != DependencyKind::Attributes {
            watch_parent(layout, &mut out, &dependency.path);
        }
    }
    watch_parent(layout, &mut out, &layout.index);
    if let Some(root) = &layout.worktree {
        watch_directory(layout, &mut out, root, false);
    }
    out
}

pub(super) fn recursive_root_watch(root: &Path) -> Result<Watch> {
    let path = root
        .canonicalize()
        .or_raise(|| message!("could not resolve worktree notification root {}", root.display()))?;
    Ok(Watch { path, recursive: true })
}

#[derive(Default)]
pub(super) struct Directories {
    root: PathBuf,
    pub watches: BTreeMap<PathBuf, bool>,
    pub repositories: HashSet<PathBuf>,
    /// Known directory names, with whether their descendants have been discovered.
    pub paths: BTreeMap<PathBuf, bool>,
    ignored: std::collections::BTreeSet<crate::bstr::BString>,
    excluded: HashSet<PathBuf>,
    disk_repositories: HashSet<PathBuf>,
    #[cfg(test)]
    pub read_dir_calls: u32,
    recursive: HashSet<PathBuf>,
    changed: HashSet<PathBuf>,
    error: Option<gix_error::Error>,
}

impl gix_dir::walk::Delegate for Directories {
    fn emit(&mut self, _: gix_dir::EntryRef<'_>, _: Option<gix_dir::entry::Status>) -> gix_dir::walk::Action {
        if self.error.is_some() {
            std::ops::ControlFlow::Break(())
        } else {
            std::ops::ControlFlow::Continue(())
        }
    }

    fn can_recurse(
        &mut self,
        entry: gix_dir::EntryRef<'_>,
        deletion: Option<gix_dir::walk::ForDeletionMode>,
        root_is_repo: bool,
    ) -> bool {
        if self.error.is_some() {
            return false;
        }
        let recurse = entry
            .status
            .can_recurse(entry.disk_kind, entry.pathspec_match, deletion, root_is_repo);
        let repository_sentinel = !entry.rela_path.is_empty()
            && entry.disk_kind == Some(gix_dir::entry::Kind::Repository)
            && matches!(
                entry.status,
                gix_dir::entry::Status::Tracked | gix_dir::entry::Status::Untracked
            );
        let path = match gix_path::from_bstr(entry.rela_path.as_ref()) {
            Ok(path) => self.root.join(path),
            Err(err) => {
                self.error = Some(err);
                return false;
            }
        };
        if entry.property != Some(gix_dir::entry::Property::DotGit) {
            self.paths
                .entry(path.clone())
                .and_modify(|seen| *seen |= recurse)
                .or_insert(recurse);
        }
        if repository_sentinel && path.join(".git").symlink_metadata().is_ok() {
            self.disk_repositories.insert(path.clone());
        }
        if recurse
            && path != self.root
            && self.recursive.iter().any(|root| path.starts_with(root))
            && !self
                .changed
                .iter()
                .any(|changed| changed.starts_with(&path) || path.starts_with(changed))
        {
            return false;
        }
        recurse
    }
}

pub(super) fn worktree_directories(repo: &Repository, index: &gix_index::State) -> Result<Directories> {
    worktree_directories_reusing(repo, index, None, &HashSet::new())
}

pub(super) fn worktree_directories_reusing(
    repo: &Repository,
    index: &gix_index::State,
    previous: Option<&Directories>,
    changed: &HashSet<PathBuf>,
) -> Result<Directories> {
    let root = repo
        .workdir()
        .ok_or_else(|| message("cannot enumerate a bare worktree").raise())?;
    let caps = repo
        .filesystem_options()
        .or_raise(|| message("could not read filesystem options"))?;
    let root = super::absolute_path(root, repo.current_dir(), caps.precompose_unicode)?;
    let root = root.as_path();
    let mut excludes = repo
        .excludes(
            index,
            None,
            gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
        )
        .or_raise(|| message("could not prepare ignore rules for filesystem monitoring"))?;
    // Watch coverage is independent of command-line/environment pathspec settings.
    let mut pathspec = gix_pathspec::Search::from_specs(std::iter::empty(), None, root)
        .or_raise(|| message("could not prepare unrestricted monitor pathspec"))?;
    let git_dir = gix_path::realpath_opts(repo.git_dir(), repo.current_dir(), gix_path::realpath::MAX_SYMLINKS)
        .or_raise(|| message("could not resolve administrative directory for monitoring"))?;
    let lookup = caps.ignore_case.then(|| index.prepare_icase_backing());
    let mut directories = Directories {
        root: root.to_owned(),
        watches: BTreeMap::from([(root.to_owned(), false)]),
        repositories: HashSet::new(),
        paths: previous.map_or_else(BTreeMap::new, |previous| previous.paths.clone()),
        ignored: Default::default(),
        excluded: Default::default(),
        disk_repositories: previous.map_or_else(HashSet::new, |previous| previous.disk_repositories.clone()),
        #[cfg(test)]
        read_dir_calls: 0,
        recursive: previous
            .into_iter()
            .flat_map(|previous| &previous.watches)
            .filter(|(_, recursive)| **recursive)
            .map(|(path, _)| path.clone())
            .collect(),
        changed: changed.clone(),
        error: None,
    };
    directories
        .paths
        .retain(|path, _| !changed.iter().any(|changed| path.starts_with(changed)));
    directories
        .disk_repositories
        .retain(|path| !changed.iter().any(|changed| path.starts_with(changed)));
    let result = gix_dir::walk(
        root,
        gix_dir::walk::Context {
            should_interrupt: None,
            git_dir_realpath: &git_dir,
            current_dir: repo.current_dir(),
            index,
            ignore_case_index_lookup: lookup.as_ref(),
            pathspec: &mut pathspec,
            pathspec_attributes: &mut |_, _, _, _| false,
            excludes: Some(&mut excludes.inner),
            objects: &repo.objects,
            explicit_traversal_root: Some(root),
        },
        gix_dir::walk::Options {
            precompose_unicode: caps.precompose_unicode,
            ignore_case: caps.ignore_case,
            ..Default::default()
        },
        &mut directories,
    );
    if let Some(err) = directories.error.take() {
        return Err(err.and_raise(message(
            "could not represent a directory path for filesystem monitoring",
        )));
    }
    let _walk = result.or_raise(|| message("could not enumerate directories for filesystem monitoring"))?;
    directories.recheck(repo, index)?;
    #[cfg(test)]
    {
        directories.read_dir_calls = _walk.0.read_dir_calls;
    }
    directories.recursive.clear();
    directories.changed.clear();
    Ok(directories)
}

impl Directories {
    /// Reuse the discovery walk's directory list when the backend would otherwise walk it again.
    pub(super) fn watches_for_backend(&self, native_recursive: bool) -> BTreeMap<PathBuf, bool> {
        if native_recursive {
            return self.watches.clone();
        }
        self.paths
            .keys()
            .filter(|path| {
                self.watches.contains_key(*path)
                    || path
                        .ancestors()
                        .skip(1)
                        .any(|parent| self.watches.get(parent) == Some(&true))
            })
            .map(|path| (path.clone(), false))
            .collect()
    }

    pub(super) fn index_fingerprint(&self, index: &gix_index::State, ignore_case: bool) -> Result<gix_hash::ObjectId> {
        let mut hash = gix_hash::hasher(gix_hash::Kind::longest());
        hash.update(index_fingerprint(index)?.as_slice());
        let lookup = (ignore_case && !self.ignored.is_empty()).then(|| index.prepare_icase_backing());
        for path in &self.ignored {
            let tracked = lookup.as_ref().map_or_else(
                || index.path_is_directory(path.as_ref()),
                |lookup| index.path_is_directory_icase(path.as_ref(), ignore_case, lookup),
            );
            hash.update(&(path.len() as u64).to_le_bytes());
            hash.update(path);
            hash.update(&[u8::from(tracked)]);
        }
        hash.try_finalize()
    }

    /// Re-evaluate known directories without enumerating their contents. Reenabled subtrees need
    /// local discovery because their contents could change while they were not watched.
    pub(super) fn recheck(&mut self, repo: &Repository, index: &gix_index::State) -> Result<HashSet<PathBuf>> {
        #[cfg(test)]
        {
            self.read_dir_calls = 0;
        }
        let ignore_case = repo.filesystem_options()?.ignore_case;
        let mut lookup = None;
        let mut excludes = repo.excludes(
            index,
            None,
            gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
        )?;
        self.watches = BTreeMap::from([(self.root.clone(), false)]);
        self.ignored.clear();
        let previously_excluded = std::mem::take(&mut self.excluded);
        self.repositories = self.disk_repositories.clone();
        let mut gitlinks = Vec::new();
        let mut sparse = Vec::new();
        for entry in index
            .entries()
            .iter()
            .filter(|entry| matches!(entry.mode, gix_index::entry::Mode::COMMIT | gix_index::entry::Mode::DIR))
        {
            let path = self.root.join(gix_path::from_bstr(entry.path(index))?);
            if entry.mode == gix_index::entry::Mode::COMMIT {
                gitlinks.push(path);
            } else if entry.flags.contains(gix_index::entry::Flags::SKIP_WORKTREE) {
                sparse.push(path);
            }
        }
        let gitlinks = directory_lookup(gitlinks);
        let sparse = directory_lookup(sparse);
        let mut discover = HashSet::new();
        for (path, discovered) in &self.paths {
            if path == &self.root
                || path
                    .ancestors()
                    .skip(1)
                    .any(|parent| self.excluded.contains(parent) || self.repositories.contains(parent))
            {
                continue;
            }
            let relative = gix_path::into_bstr(
                path.strip_prefix(&self.root)
                    .expect("inventory paths are inside the root"),
            )?;
            let relative = gix_path::to_unix_separators_on_windows(relative);
            let ignored = excludes
                .at_entry(relative.as_ref(), Some(gix_index::entry::Mode::DIR))?
                .is_excluded();
            let tracked = ignored
                && if ignore_case {
                    index.path_is_directory_icase(
                        relative.as_ref(),
                        true,
                        lookup.get_or_insert_with(|| index.prepare_icase_backing()),
                    )
                } else {
                    index.path_is_directory(relative.as_ref())
                };
            if ignored {
                self.ignored.insert(relative.into_owned());
            }
            let is_gitlink = gitlinks
                .find(path_hash(path), |gitlink| super::same_path(gitlink, path, ignore_case))
                .is_some();
            let excluded = ignored && !tracked
                || sparse
                    .find(path_hash(path), |directory| {
                        super::same_path(directory, path, ignore_case)
                    })
                    .is_some();
            if excluded {
                self.excluded.insert(path.clone());
            } else {
                if is_gitlink {
                    self.repositories.insert(path.clone());
                }
                self.watches
                    .entry(path.clone())
                    .or_insert(!self.repositories.contains(path));
                if (!discovered || previously_excluded.contains(path)) && !self.repositories.contains(path) {
                    discover.insert(path.clone());
                }
            }
            if excluded || self.repositories.contains(path) {
                for parent in path
                    .ancestors()
                    .skip(1)
                    .take_while(|parent| parent.starts_with(&self.root))
                {
                    self.watches.insert(parent.to_owned(), false);
                }
            }
        }
        let watches = std::mem::take(&mut self.watches);
        self.watches = watches
            .iter()
            .filter(|(path, _)| {
                !path
                    .ancestors()
                    .skip(1)
                    .any(|parent| watches.get(parent) == Some(&true))
            })
            .map(|(path, recursive)| (path.clone(), *recursive))
            .collect();
        Ok(discover)
    }
}
