use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use bstr::BStr;
use gix_dir::{Entry, entry, walk};
use gix_testtools::scripted_fixture_read_only;

pub fn fixture_in(filename: &str, name: &str) -> PathBuf {
    let root = scripted_fixture_read_only(format!("{filename}.sh")).expect("script works");
    root.join(name)
}

pub fn fixture(name: &str) -> PathBuf {
    fixture_in("many", name)
}

/// Default options
pub fn options() -> walk::Options<'static> {
    walk::Options::default()
}

/// Default options
pub fn options_emit_all() -> walk::Options<'static> {
    walk::Options {
        precompose_unicode: false,
        ignore_case: false,
        recurse_repositories: false,
        for_deletion: None,
        classify_untracked_bare_repositories: false,
        emit_pruned: true,
        emit_ignored: Some(walk::EmissionMode::Matching),
        emit_tracked: true,
        emit_untracked: walk::EmissionMode::Matching,
        emit_empty_directories: true,
        emit_collapsed: None,
        symlinks_to_directories_are_ignored_like_directories: false,
        worktree_relative_worktree_dirs: None,
    }
}

pub fn entry(
    rela_path: impl AsRef<BStr>,
    status: entry::Status,
    disk_kind: entry::Kind,
) -> (Entry, Option<entry::Status>) {
    entryps(rela_path, status, disk_kind, entry::PathspecMatch::Always)
}

pub fn entry_nomatch(
    rela_path: impl AsRef<BStr>,
    status: entry::Status,
    disk_kind: entry::Kind,
) -> (Entry, Option<entry::Status>) {
    (
        Entry {
            rela_path: rela_path.as_ref().to_owned(),
            status,
            property: None,
            disk_kind: Some(disk_kind),
            index_kind: index_kind_from_status(status, disk_kind),
            pathspec_match: None,
        },
        None,
    )
}

pub fn entry_nokind(rela_path: impl AsRef<BStr>, status: entry::Status) -> (Entry, Option<entry::Status>) {
    (
        Entry {
            rela_path: rela_path.as_ref().to_owned(),
            status,
            property: None,
            disk_kind: None,
            index_kind: None,
            pathspec_match: None,
        },
        None,
    )
}

pub fn entryps(
    rela_path: impl AsRef<BStr>,
    status: entry::Status,
    disk_kind: entry::Kind,
    pathspec_match: entry::PathspecMatch,
) -> (Entry, Option<entry::Status>) {
    (
        Entry {
            rela_path: rela_path.as_ref().to_owned(),
            status,
            property: None,
            disk_kind: Some(disk_kind),
            index_kind: index_kind_from_status(status, disk_kind),
            pathspec_match: Some(pathspec_match),
        },
        None,
    )
}

pub fn entry_dirstat(
    rela_path: impl AsRef<BStr>,
    status: entry::Status,
    disk_kind: entry::Kind,
    dir_status: entry::Status,
) -> (Entry, Option<entry::Status>) {
    (
        Entry {
            rela_path: rela_path.as_ref().to_owned(),
            status,
            property: None,
            disk_kind: Some(disk_kind),
            index_kind: index_kind_from_status(status, disk_kind),
            pathspec_match: Some(entry::PathspecMatch::Always),
        },
        Some(dir_status),
    )
}

/// These are entries that have been collapsed into a single directory.
pub fn entryps_dirstat(
    rela_path: impl AsRef<BStr>,
    status: entry::Status,
    disk_kind: entry::Kind,
    pathspec_match: entry::PathspecMatch,
    dir_status: entry::Status,
) -> (Entry, Option<entry::Status>) {
    (
        Entry {
            rela_path: rela_path.as_ref().to_owned(),
            status,
            property: None,
            disk_kind: Some(disk_kind),
            index_kind: index_kind_from_status(status, disk_kind),
            pathspec_match: Some(pathspec_match),
        },
        Some(dir_status),
    )
}

fn index_kind_from_status(status: entry::Status, disk_kind: entry::Kind) -> Option<entry::Kind> {
    matches!(status, entry::Status::Tracked).then_some(disk_kind)
}

pub trait EntryExt {
    fn with_index_kind(self, index_kind: entry::Kind) -> Self;
    fn with_property(self, flags: entry::Property) -> Self;
    fn with_match(self, m: entry::PathspecMatch) -> Self;
    fn no_match(self) -> Self;
    fn no_kind(self) -> Self;
    fn no_index_kind(self) -> Self;
}

impl EntryExt for (Entry, Option<entry::Status>) {
    fn with_index_kind(mut self, index_kind: entry::Kind) -> Self {
        self.0.index_kind = index_kind.into();
        self
    }
    fn with_property(mut self, property: entry::Property) -> Self {
        self.0.property = property.into();
        self
    }
    fn with_match(mut self, m: entry::PathspecMatch) -> Self {
        self.0.pathspec_match = Some(m);
        self
    }

    fn no_match(mut self) -> Self {
        self.0.pathspec_match = None;
        self
    }
    fn no_kind(mut self) -> Self {
        self.0.disk_kind = None;
        self
    }
    fn no_index_kind(mut self) -> Self {
        self.0.index_kind = None;
        self
    }
}

pub fn collect(
    worktree_root: &Path,
    root: Option<&Path>,
    cb: impl FnOnce(&mut dyn walk::Delegate, walk::Context) -> Result<(walk::Outcome, PathBuf), walk::Error>,
) -> ((walk::Outcome, PathBuf), Entries) {
    try_collect(worktree_root, root, cb).unwrap()
}

pub fn collect_filtered(
    worktree_root: &Path,
    root: Option<&Path>,
    cb: impl FnOnce(&mut dyn walk::Delegate, walk::Context) -> Result<(walk::Outcome, PathBuf), walk::Error>,
    patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
) -> ((walk::Outcome, PathBuf), Entries) {
    try_collect_filtered(worktree_root, root, cb, patterns).unwrap()
}

pub fn try_collect(
    worktree_root: &Path,
    root: Option<&Path>,
    cb: impl FnOnce(&mut dyn walk::Delegate, walk::Context) -> Result<(walk::Outcome, PathBuf), walk::Error>,
) -> Result<((walk::Outcome, PathBuf), Entries), walk::Error> {
    try_collect_filtered(worktree_root, root, cb, None::<&str>)
}

pub fn try_collect_filtered(
    worktree_root: &Path,
    root: Option<&Path>,
    cb: impl FnOnce(&mut dyn walk::Delegate, walk::Context) -> Result<(walk::Outcome, PathBuf), walk::Error>,
    patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
) -> Result<((walk::Outcome, PathBuf), Entries), walk::Error> {
    try_collect_filtered_opts_collect(worktree_root, root, cb, patterns, Default::default())
}

pub fn try_collect_filtered_opts_collect(
    worktree_root: &Path,
    root: Option<&Path>,
    cb: impl FnOnce(&mut dyn walk::Delegate, walk::Context) -> Result<(walk::Outcome, PathBuf), walk::Error>,
    patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
    options: Options<'_>,
) -> Result<((walk::Outcome, PathBuf), Entries), walk::Error> {
    let mut dlg = gix_dir::walk::delegate::Collect::default();
    let outcome = try_collect_filtered_opts(worktree_root, root, None, None, cb, patterns, &mut dlg, options)?;
    Ok((outcome, dlg.into_entries_by_path()))
}

pub fn try_collect_filtered_opts_collect_with_root(
    worktree_root: &Path,
    root: Option<&Path>,
    explicit_traversal_root: Option<&Path>,
    cb: impl FnOnce(&mut dyn walk::Delegate, walk::Context) -> Result<(walk::Outcome, PathBuf), walk::Error>,
    patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
    options: Options<'_>,
) -> Result<((walk::Outcome, PathBuf), Entries), walk::Error> {
    let mut dlg = gix_dir::walk::delegate::Collect::default();
    let outcome = try_collect_filtered_opts(
        worktree_root,
        root,
        explicit_traversal_root,
        None,
        cb,
        patterns,
        &mut dlg,
        options,
    )?;
    Ok((outcome, dlg.into_entries_by_path()))
}

pub fn collect_filtered_with_cwd(
    worktree_root: &Path,
    root: Option<&Path>,
    cwd_suffix: Option<&str>,
    cb: impl FnOnce(&mut dyn walk::Delegate, walk::Context) -> Result<(walk::Outcome, PathBuf), walk::Error>,
    patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
) -> ((walk::Outcome, PathBuf), Entries) {
    let mut dlg = gix_dir::walk::delegate::Collect::default();
    let outcome = try_collect_filtered_opts(
        worktree_root,
        root,
        None,
        cwd_suffix,
        cb,
        patterns,
        &mut dlg,
        Default::default(),
    )
    .expect("success");
    (outcome, dlg.into_entries_by_path())
}

#[allow(clippy::too_many_arguments)]
pub fn try_collect_filtered_opts(
    worktree_root: &Path,
    root: Option<&Path>,
    explicit_traversal_root: Option<&Path>,
    append_to_cwd: Option<&str>,
    cb: impl FnOnce(&mut dyn walk::Delegate, walk::Context) -> Result<(walk::Outcome, PathBuf), walk::Error>,
    patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
    delegate: &mut dyn gix_dir::walk::Delegate,
    Options {
        fresh_index,
        git_dir,
        should_interrupt,
    }: Options<'_>,
) -> Result<(walk::Outcome, PathBuf), walk::Error> {
    let git_dir = worktree_root.join(git_dir.unwrap_or(".git"));
    let mut index = std::fs::read(git_dir.join("index")).ok().map_or_else(
        || gix_index::State::new(gix_index::hash::Kind::Sha1),
        |bytes| {
            gix_index::State::from_bytes(
                &bytes,
                std::time::UNIX_EPOCH.into(),
                gix_index::hash::Kind::Sha1,
                Default::default(),
            )
            .map(|t| t.0)
            .expect("valid index")
        },
    );
    if fresh_index {
        index
            .entries_mut()
            .iter_mut()
            .filter(|e| {
                // relevant for partial checkouts, all related entries will have skip-worktree set,
                // which also means they will never be up-to-date.
                !e.flags.contains(gix_index::entry::Flags::SKIP_WORKTREE)
            })
            .for_each(|e| {
                // pretend that the index was refreshed beforehand so we know what's uptodate.
                e.flags |= gix_index::entry::Flags::UPTODATE;
            });
    }
    let mut search = gix_pathspec::Search::from_specs(
        patterns.into_iter().map(|spec| {
            gix_pathspec::parse(spec.as_ref(), gix_pathspec::Defaults::default()).expect("tests use valid pattern")
        }),
        root.map(|root| root.strip_prefix(worktree_root).expect("root is within worktree root"))
            .or_else(|| append_to_cwd.map(Path::new)),
        "we don't provide absolute pathspecs, thus need no worktree root".as_ref(),
    )
    .expect("search creation can't fail");
    let mut stack = gix_worktree::Stack::from_state_and_ignore_case(
        worktree_root,
        false, /* ignore case */
        gix_worktree::stack::State::IgnoreStack(gix_worktree::stack::state::Ignore::new(
            Default::default(),
            Default::default(),
            None,
            gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
            gix_ignore::search::Ignore { support_precious: true },
        )),
        &index,
        index.path_backing(),
    );

    let mut cwd = worktree_root.to_owned();
    if let Some(suffix) = append_to_cwd {
        assert!(
            worktree_root.is_absolute(),
            "BUG: need absolute worktree root for CWD checks to work"
        );
        cwd.push(suffix);
    }
    let git_dir_realpath = gix_path::realpath_opts(&git_dir, &cwd, gix_path::realpath::MAX_SYMLINKS).unwrap();
    let lookup = index.prepare_icase_backing();
    cb(
        delegate,
        walk::Context {
            git_dir_realpath: &git_dir_realpath,
            current_dir: &cwd,
            index: &index,
            ignore_case_index_lookup: Some(&lookup),
            pathspec: &mut search,
            pathspec_attributes: &mut |_, _, _, _| panic!("we do not use pathspecs that require attributes access."),
            excludes: Some(&mut stack),
            objects: &gix_object::find::Never,
            explicit_traversal_root,
            should_interrupt,
        },
    )
}

pub struct Options<'a> {
    pub fresh_index: bool,
    pub git_dir: Option<&'a str>,
    pub should_interrupt: Option<&'a AtomicBool>,
}

impl<'a> Options<'a> {
    pub fn git_dir(dir: &'a str) -> Self {
        Options {
            git_dir: Some(dir),
            ..Default::default()
        }
    }
}

impl Default for Options<'_> {
    fn default() -> Self {
        Options {
            fresh_index: true,
            git_dir: None,
            should_interrupt: None,
        }
    }
}

type Entries = Vec<(Entry, Option<entry::Status>)>;
