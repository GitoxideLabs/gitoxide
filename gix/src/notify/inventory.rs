use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

use gix_error::{ErrorExt, ResultExt, message};

use super::{Error, Layout};
use crate::{Repository, bstr::BString};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct IndexEntry {
    path: BString,
    mode: u32,
    flags: u32,
}

pub(super) fn index_projection(index: &gix_index::State) -> Vec<IndexEntry> {
    let mut out: Vec<_> = index
        .entries()
        .iter()
        .map(|entry| IndexEntry {
            path: entry.path(index).to_owned(),
            mode: entry.mode.bits(),
            flags: (entry.flags & (gix_index::entry::Flags::SKIP_WORKTREE | gix_index::entry::Flags::STAGE_MASK))
                .bits(),
        })
        .collect();
    out.sort_unstable();
    out
}

pub(super) fn index_changes_directories(before: &[IndexEntry], after: &[IndexEntry]) -> bool {
    let has_directory =
        |entry: &IndexEntry| entry.path.contains(&b'/') || entry.mode == gix_index::entry::Mode::DIR.bits();
    let (mut left, mut right) = (0, 0);
    while left < before.len() || right < after.len() {
        match (before.get(left), after.get(right)) {
            (Some(a), Some(b)) => match a.cmp(b) {
                std::cmp::Ordering::Equal => {
                    left += 1;
                    right += 1;
                }
                std::cmp::Ordering::Less => {
                    if has_directory(a) {
                        return true;
                    }
                    left += 1;
                }
                std::cmp::Ordering::Greater => {
                    if has_directory(b) {
                        return true;
                    }
                    right += 1;
                }
            },
            (Some(a), None) => {
                if has_directory(a) {
                    return true;
                }
                left += 1;
            }
            (None, Some(b)) => {
                if has_directory(b) {
                    return true;
                }
                right += 1;
            }
            (None, None) => break,
        }
    }
    false
}

fn watch_directory(watches: &mut BTreeMap<PathBuf, bool>, path: &Path, recursive: bool) {
    let mut candidate = path;
    let mut recursive = recursive;
    while !candidate.is_dir() {
        let Some(parent) = candidate.parent() else { return };
        candidate = parent;
        recursive = false;
    }
    watches
        .entry(candidate.to_owned())
        .and_modify(|value| *value |= recursive)
        .or_insert(recursive);
}

pub(super) fn watch_parent(watches: &mut BTreeMap<PathBuf, bool>, path: &Path) {
    if let Some(parent) = path.parent() {
        watch_directory(watches, parent, false);
    }
}

pub(super) fn metadata_watches(layout: &Layout) -> BTreeMap<PathBuf, bool> {
    let mut out = BTreeMap::new();
    for root in [&layout.git_dir, &layout.common_dir] {
        watch_directory(&mut out, root, false);
        watch_parent(&mut out, root);
        watch_directory(&mut out, &root.join("refs"), true);
    }
    watch_directory(&mut out, &layout.common_dir.join("worktrees"), true);
    for name in ["rebase-apply", "rebase-merge", "sequencer"] {
        watch_directory(&mut out, &layout.git_dir.join(name), true);
    }
    for source in &layout.sources {
        watch_parent(&mut out, &source.path);
    }
    if let Some(root) = &layout.worktree {
        watch_directory(&mut out, root, false);
    }
    out
}

#[derive(Default)]
pub(super) struct Directories {
    root: PathBuf,
    pub paths: HashSet<PathBuf>,
    pub repositories: HashSet<PathBuf>,
}

impl gix_dir::walk::Delegate for Directories {
    fn emit(&mut self, _: gix_dir::EntryRef<'_>, _: Option<gix_dir::entry::Status>) -> gix_dir::walk::Action {
        std::ops::ControlFlow::Continue(())
    }

    fn can_recurse(
        &mut self,
        entry: gix_dir::EntryRef<'_>,
        deletion: Option<gix_dir::walk::ForDeletionMode>,
        root_is_repo: bool,
    ) -> bool {
        let recurse = entry
            .status
            .can_recurse(entry.disk_kind, entry.pathspec_match, deletion, root_is_repo);
        let repository_sentinel = entry.disk_kind == Some(gix_dir::entry::Kind::Repository)
            && matches!(
                entry.status,
                gix_dir::entry::Status::Tracked | gix_dir::entry::Status::Untracked
            );
        if repository_sentinel {
            self.repositories
                .insert(self.root.join(gix_path::from_bstr(entry.rela_path.as_ref())));
        }
        if recurse || repository_sentinel {
            self.paths
                .insert(self.root.join(gix_path::from_bstr(entry.rela_path.as_ref())));
        }
        recurse
    }
}

pub(super) fn worktree_directories(repo: &Repository, index: &gix_index::State) -> Result<Directories, Error> {
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
    let git_dir = super::absolute_path(repo.git_dir(), repo.current_dir(), caps.precompose_unicode)?;
    let lookup = caps.ignore_case.then(|| index.prepare_icase_backing());
    let mut directories = Directories {
        root: root.to_owned(),
        paths: HashSet::from([root.to_owned()]),
        repositories: HashSet::new(),
    };
    gix_dir::walk(
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
    )
    .or_raise(|| message("could not enumerate directories for filesystem monitoring"))?;
    Ok(directories)
}
