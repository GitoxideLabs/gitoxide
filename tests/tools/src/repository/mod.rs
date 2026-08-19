//! Stable snapshots of the Git and filesystem state of a test repository.

use std::{
    borrow::Cow,
    collections::BTreeMap,
    ffi::OsStr,
    fmt, fs,
    path::{Path, PathBuf},
};

use bstr::{BStr, BString, ByteSlice};
use gix_hash::ObjectId;

use crate::Result;

#[cfg(not(feature = "repo-snapshot"))]
mod git;
#[cfg(feature = "repo-snapshot")]
mod gix;

/// All relevant observable state of a repository and its worktree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    /// The current HEAD, including whether it is attached, detached, or unborn.
    pub head: Head,
    /// The contents of the repository's common `config` file, with locations normalized and generated keys removed.
    pub config: BString,
    /// Every reference below `refs/`, sorted by name.
    pub references: Vec<Reference>,
    /// Raw commit objects reachable from HEAD or any reference, sorted by object ID.
    ///
    /// The raw data retains the tree, parents, identities, dates, headers, and message.
    pub commits: Vec<Commit>,
    /// Every index entry and stage, sorted in index order.
    pub index: Vec<IndexEntry>,
    /// The tree represented by a conflict-free index, computed without writing objects.
    pub index_tree: Option<ObjectId>,
    /// Exact filesystem entries below the worktree, excluding `.git` administration entries.
    pub worktree: Vec<WorktreeEntry>,
    normalization_root: PathBuf,
    show_object_ids: bool,
}

/// The state of `HEAD`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Head {
    /// HEAD names a branch which does not exist yet.
    Unborn(BString),
    /// HEAD names a branch and resolves to the given object.
    Symbolic {
        /// The full branch name.
        name: BString,
        /// The fully peeled commit currently reached through the branch.
        id: ObjectId,
    },
    /// HEAD directly names the given object.
    Detached(ObjectId),
}

/// A reference and its immediate target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reference {
    /// Full reference name.
    pub name: BString,
    /// Direct or symbolic target.
    pub target: ReferenceTarget,
}

/// A reference target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReferenceTarget {
    /// Another reference name.
    Symbolic(BString),
    /// An object ID.
    Object(ObjectId),
}

/// A raw commit object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Commit {
    /// Commit object ID.
    pub id: ObjectId,
    /// Complete decoded object bytes, excluding the loose-object header.
    pub data: Vec<u8>,
}

/// One index entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexEntry {
    /// Git tree mode as stored in the index.
    pub mode: u32,
    /// Blob or submodule object ID.
    pub id: ObjectId,
    /// Conflict stage, with zero denoting an ordinary entry.
    pub stage: u8,
    /// Repository-relative byte path.
    pub path: BString,
}

/// One filesystem entry in the worktree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeEntry {
    /// Worktree-relative path.
    pub path: PathBuf,
    /// Entry kind and contents.
    pub kind: WorktreeEntryKind,
    /// Unix permission bits when available.
    pub unix_mode: Option<u32>,
}

/// The kind and exact contents of a worktree entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorktreeEntryKind {
    /// A directory.
    Directory,
    /// A regular file.
    File(Vec<u8>),
    /// A symbolic link with its target.
    Symlink(PathBuf),
}

/// Capture repository state at `path` without modifying references, index, objects, or worktree.
///
/// Location-bearing config values are normalized relative to the common Git directory, using `<normalized>` in place
/// of the directory itself. Locations outside the repository which cannot be made relative become `<normalized>`.
/// Git- and platform-generated config keys are omitted.
pub fn snapshot(path: impl AsRef<Path>) -> Result<State> {
    #[cfg(feature = "repo-snapshot")]
    let mut state = gix::snapshot(path.as_ref())?;
    #[cfg(not(feature = "repo-snapshot"))]
    let mut state = git::snapshot(path.as_ref())?;
    state.config = normalize_config_paths(state.config.as_bstr(), &state.normalization_root)?;
    state.config = remove_generated_config(state.config.as_bstr())?;
    state.show_object_ids = true;
    Ok(state)
}

/// Capture portable repository state at `path` without modifying references, index, objects, or worktree.
///
/// Unlike [`snapshot()`], this omits filesystem metadata and object IDs which aren't stable across all supported
/// platforms and object formats. In particular, [`WorktreeEntry::unix_mode`] is always `None`, and its display
/// representation omits the object-ID mapping. Git index modes remain available as they are part of the repository
/// itself and have platform-independent meaning. Config locations use the same normalization as [`snapshot()`].
pub fn snapshot_portable(path: impl AsRef<Path>) -> Result<State> {
    let mut state = snapshot(path)?;
    for entry in &mut state.worktree {
        entry.unix_mode = None;
    }
    state.config = normalize_config_indentation(state.config);
    state.show_object_ids = false;
    Ok(state)
}

#[cfg(feature = "repo-snapshot")]
fn normalize_config_paths(input: &BStr, root: &Path) -> Result<BString> {
    let mut config = gix_config::File::try_from(input)?;
    for (section_name, value_name) in [
        ("core", "worktree"),
        ("remote", "url"),
        ("remote", "pushurl"),
        ("submodule", "url"),
        ("include", "path"),
        ("includeIf", "path"),
    ] {
        let subsections: std::collections::BTreeSet<_> = config
            .sections_and_ids_by_name(section_name)
            .into_iter()
            .flatten()
            .map(|(section, _)| section.header().subsection_name().map(ToOwned::to_owned))
            .collect();
        for subsection in subsections {
            if let Ok(mut values) =
                config.raw_values_mut_by(section_name, subsection.as_ref().map(|name| name.as_bstr()), value_name)
            {
                let normalized: Vec<_> = values
                    .get()?
                    .into_iter()
                    .map(|value| normalize_config_path(value.as_bstr(), root))
                    .collect();
                for (index, value) in normalized.into_iter().enumerate() {
                    values.set_at(index, value)?;
                }
            }
        }
    }
    Ok(config.into())
}

fn normalize_config_path(value: &BStr, root: &Path) -> BString {
    let path = gix_path::from_bstr(value).into_owned();
    let relative = if path.is_absolute() {
        path.strip_prefix(root)
            .map(Path::to_owned)
            .ok()
            .or_else(|| {
                let root = root.canonicalize().ok()?;
                path.canonicalize().ok()?.strip_prefix(root).map(Path::to_owned).ok()
            })
            .or_else(|| relative_to_repository_sibling(&path, root))
    } else {
        if value.contains_str("://")
            || value
                .find_byte(b':')
                .is_some_and(|colon| !value[..colon].contains(&b'/'))
        {
            return "<normalized>".into();
        }
        Some(path)
    };
    let Some(relative) = relative else {
        return "<normalized>".into();
    };
    let relative = portable_path(&relative);
    if relative.is_empty() {
        return "<normalized>".into();
    }
    let mut out = b"<normalized>/".to_vec();
    out.extend_from_slice(&relative);
    out.into()
}

fn relative_to_repository_sibling(path: &Path, git_dir: &Path) -> Option<PathBuf> {
    let repository = if git_dir.file_name() == Some(OsStr::new(".git")) {
        git_dir.parent()?
    } else {
        git_dir
    };
    relative_to_repository_sibling_inner(path, repository).or_else(|| {
        let path = comparable_realpath(path)?;
        let repository = comparable_realpath(repository)?;
        relative_to_repository_sibling_inner(&path, &repository)
    })
}

fn relative_to_repository_sibling_inner(path: &Path, repository: &Path) -> Option<PathBuf> {
    path.strip_prefix(repository.parent()?).ok()?;
    relative_path(repository, path)
}

fn comparable_realpath(path: &Path) -> Option<PathBuf> {
    let realpath = gix_path::realpath(path).ok()?;
    #[cfg(windows)]
    {
        // Make equivalent existing paths such as `D:\a\gitoxide\source` and
        // `\\?\D:\a\gitoxide\source` comparable. Keep `realpath` for missing components.
        Some(realpath.canonicalize().unwrap_or(realpath))
    }
    #[cfg(not(windows))]
    {
        Some(realpath)
    }
}

fn relative_path(from: &Path, to: &Path) -> Option<PathBuf> {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(left, right)| left == right).count();
    if common == 0 {
        return None;
    }
    let mut out = PathBuf::new();
    for _ in &from[common..] {
        out.push("..");
    }
    for component in &to[common..] {
        out.push(component.as_os_str());
    }
    Some(out)
}

#[cfg(not(feature = "repo-snapshot"))]
fn normalize_config_paths(input: &BStr, root: &Path) -> Result<BString> {
    let mut out = Vec::with_capacity(input.len());
    let mut section = b"".as_slice();
    for line in input.lines_with_terminator() {
        if let Some(name) = config_section_name(line) {
            section = name;
            out.extend_from_slice(line);
            continue;
        }
        let Some((key, value_start, value_end)) = config_key_and_value(line) else {
            out.extend_from_slice(line);
            continue;
        };
        if is_location_key(section, key) {
            out.extend_from_slice(&line[..value_start]);
            out.extend_from_slice(&normalize_config_path(line[value_start..value_end].as_bstr(), root));
            out.extend_from_slice(&line[value_end..]);
        } else {
            out.extend_from_slice(line);
        }
    }
    Ok(out.into())
}

#[cfg(not(feature = "repo-snapshot"))]
fn remove_generated_config(input: &BStr) -> Result<BString> {
    let mut out = Vec::with_capacity(input.len());
    let mut header = None;
    let mut body = Vec::new();
    for line in input.lines_with_terminator() {
        if config_section_name(line).is_some() {
            if let Some(previous) = header.replace(line) {
                write_portable_config_section(previous, &body, &mut out);
                body.clear();
            }
        } else if header.is_some() {
            body.push(line);
        } else {
            out.extend_from_slice(line);
        }
    }
    if let Some(header) = header {
        write_portable_config_section(header, &body, &mut out);
    }
    Ok(out.into())
}

#[cfg(not(feature = "repo-snapshot"))]
fn write_portable_config_section(header: &[u8], body: &[&[u8]], out: &mut Vec<u8>) {
    let section = config_section_name(header).expect("caller provides a section header");
    let retained: Vec<_> = body
        .iter()
        .copied()
        .filter(|line| match config_key_and_value(line) {
            Some((key, _, _)) => !is_generated_config_key(section, key),
            None => true,
        })
        .collect();
    let has_values = retained.iter().any(|line| config_key_and_value(line).is_some());
    if !has_values && (section.eq_ignore_ascii_case(b"core") || section.eq_ignore_ascii_case(b"extensions")) {
        return;
    }
    out.extend_from_slice(header);
    for line in retained {
        out.extend_from_slice(line);
    }
}

#[cfg(not(feature = "repo-snapshot"))]
fn config_section_name(line: &[u8]) -> Option<&[u8]> {
    let start = line.iter().take_while(|byte| byte.is_ascii_whitespace()).count();
    let body = line[start..].strip_prefix(b"[")?.split(|byte| *byte == b']').next()?;
    body.split(|byte| byte.is_ascii_whitespace() || *byte == b'\"').next()
}

#[cfg(not(feature = "repo-snapshot"))]
fn config_key_and_value(line: &[u8]) -> Option<(&[u8], usize, usize)> {
    let mut start = 0;
    while line.get(start).is_some_and(u8::is_ascii_whitespace) {
        start += 1;
    }
    if matches!(line.get(start), None | Some(b'#' | b';' | b'[')) {
        return None;
    }
    let key_end = line[start..]
        .iter()
        .position(|byte| byte.is_ascii_whitespace() || *byte == b'=')?
        + start;
    let mut value_start = key_end;
    while line.get(value_start).is_some_and(u8::is_ascii_whitespace) {
        value_start += 1;
    }
    if line.get(value_start) == Some(&b'=') {
        value_start += 1;
        while line.get(value_start).is_some_and(u8::is_ascii_whitespace) {
            value_start += 1;
        }
    }
    let mut value_end = line.len();
    while matches!(line.get(value_end.wrapping_sub(1)), Some(b'\n' | b'\r')) {
        value_end -= 1;
    }
    Some((&line[start..key_end], value_start, value_end))
}

#[cfg(not(feature = "repo-snapshot"))]
fn is_location_key(section: &[u8], key: &[u8]) -> bool {
    matches_location(section, key, b"core", b"worktree")
        || matches_location(section, key, b"remote", b"url")
        || matches_location(section, key, b"remote", b"pushurl")
        || matches_location(section, key, b"submodule", b"url")
        || matches_location(section, key, b"include", b"path")
        || matches_location(section, key, b"includeIf", b"path")
}

#[cfg(not(feature = "repo-snapshot"))]
fn matches_location(section: &[u8], key: &[u8], expected_section: &[u8], expected_key: &[u8]) -> bool {
    section.eq_ignore_ascii_case(expected_section) && key.eq_ignore_ascii_case(expected_key)
}

#[cfg(not(feature = "repo-snapshot"))]
fn is_generated_config_key(section: &[u8], key: &[u8]) -> bool {
    if section.eq_ignore_ascii_case(b"core") {
        [
            b"repositoryformatversion".as_slice(),
            b"filemode",
            b"logallrefupdates",
            b"ignorecase",
            b"precomposeunicode",
            b"symlinks",
        ]
        .iter()
        .any(|name| key.eq_ignore_ascii_case(name))
    } else if section.eq_ignore_ascii_case(b"extensions") {
        [b"objectformat".as_slice(), b"compatobjectformat"]
            .iter()
            .any(|name| key.eq_ignore_ascii_case(name))
    } else {
        false
    }
}

#[cfg(feature = "repo-snapshot")]
fn remove_generated_config(input: &BStr) -> Result<BString> {
    let mut config = gix_config::File::try_from(input)?;
    for (section_name, value_names) in [
        (
            "core",
            &[
                "repositoryformatversion",
                "filemode",
                "logallrefupdates",
                "ignorecase",
                "precomposeunicode",
                "symlinks",
            ][..],
        ),
        ("extensions", &["objectformat", "compatobjectformat"][..]),
    ] {
        let section_ids: Vec<_> = config
            .sections_and_ids_by_name(section_name)
            .into_iter()
            .flatten()
            .map(|(_, id)| id)
            .collect();
        for id in section_ids {
            let mut section = config.section_mut_by_id(id).expect("ID came from this config");
            for value_name in value_names {
                while section.remove(value_name).is_some() {}
            }
            if section.num_values() == 0 {
                config.remove_section_by_id(id);
            }
        }
    }

    Ok(config.into())
}

fn normalize_config_indentation(config: BString) -> BString {
    let mut out = Vec::with_capacity(config.len());
    let mut in_indentation = true;
    for byte in config.iter().copied() {
        match byte {
            b'\t' if in_indentation => out.extend_from_slice(b"    "),
            b'\n' => {
                out.push(byte);
                in_indentation = true;
            }
            b' ' if in_indentation => out.push(byte),
            _ => {
                out.push(byte);
                in_indentation = false;
            }
        }
    }
    out.into()
}

/// Stable, human-readable names for objects referenced by a repository snapshot.
///
/// Commit IDs become `C…`, tree IDs `T…`, blobs `B…`, and gitlinks `S…`. Any other object visible in the snapshot,
/// such as an annotated tag or a parent beyond a shallow boundary, becomes `O…`. This makes the rendered state readable
/// and largely independent of the selected object-hash format. `commits` records their deterministic parent-before-child
/// display order; object IDs break ties between unrelated commits.
struct Aliases {
    by_id: BTreeMap<ObjectId, String>,
    commits: Vec<ObjectId>,
}

impl Aliases {
    fn new(state: &State) -> Self {
        let parents: BTreeMap<_, Vec<_>> = state
            .commits
            .iter()
            .map(|commit| {
                let parents = commit
                    .data
                    .lines()
                    .filter_map(|line| line.strip_prefix(b"parent "))
                    .filter_map(|hex| ObjectId::from_hex(hex).ok())
                    .collect();
                (commit.id, parents)
            })
            .collect();
        let mut commits: Vec<_> = parents.keys().copied().collect();
        let mut depths = BTreeMap::new();
        commits.sort_by_key(|id| (commit_depth(*id, &parents, &mut depths), *id));

        let mut by_id = BTreeMap::new();
        for (index, id) in commits.iter().enumerate() {
            by_id.insert(*id, format!("C{index}"));
        }
        let trees = state
            .commits
            .iter()
            .flat_map(|commit| commit.data.lines())
            .filter_map(|line| line.strip_prefix(b"tree "))
            .filter_map(|hex| ObjectId::from_hex(hex).ok())
            .chain(state.index_tree);
        let mut tree_index = 0;
        for id in trees {
            by_id.entry(id).or_insert_with(|| {
                let alias = format!("T{tree_index}");
                tree_index += 1;
                alias
            });
        }
        let mut blob_index = 0;
        let mut gitlink_index = 0;
        for entry in &state.index {
            by_id.entry(entry.id).or_insert_with(|| {
                if entry.mode == 0o160000 {
                    let alias = format!("S{gitlink_index}");
                    gitlink_index += 1;
                    alias
                } else {
                    let alias = format!("B{blob_index}");
                    blob_index += 1;
                    alias
                }
            });
        }

        let mut other_index = 0;
        let mut insert_other = |id| {
            by_id.entry(id).or_insert_with(|| {
                let alias = format!("O{other_index}");
                other_index += 1;
                alias
            });
        };
        match &state.head {
            Head::Symbolic { id, .. } | Head::Detached(id) => insert_other(*id),
            Head::Unborn(_) => {}
        }
        for reference in &state.references {
            if let ReferenceTarget::Object(id) = &reference.target {
                insert_other(*id);
            }
        }
        for commit in &state.commits {
            for id in commit
                .data
                .lines()
                .filter_map(|line| line.strip_prefix(b"parent "))
                .filter_map(|hex| ObjectId::from_hex(hex).ok())
            {
                insert_other(id);
            }
        }
        Self { by_id, commits }
    }

    fn id(&self, id: ObjectId) -> String {
        self.by_id.get(&id).cloned().unwrap_or_else(|| id.to_string())
    }

    fn head(&self, head: &Head) -> String {
        match head {
            Head::Unborn(name) => format!("unborn {}", name.as_bstr()),
            Head::Symbolic { name, id } => format!("{} -> {}", name.as_bstr(), self.id(*id)),
            Head::Detached(id) => format!("detached {}", self.id(*id)),
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let aliases = Aliases::new(self);
        let State {
            head,
            config,
            references,
            commits,
            index,
            index_tree,
            worktree,
            normalization_root: _,
            show_object_ids,
        } = self;
        writeln!(out, "HEAD {}", aliases.head(head))?;
        write!(out, "\n[config]\n{}", config.as_bstr())?;
        if !config.ends_with_str("\n") {
            writeln!(out)?;
        }
        writeln!(out, "\n[refs]")?;
        for reference in references {
            let target = match &reference.target {
                ReferenceTarget::Symbolic(name) => format!("-> {}", name.as_bstr()),
                ReferenceTarget::Object(id) => aliases.id(*id),
            };
            writeln!(out, "{} = {target}", reference.name.as_bstr())?;
        }

        writeln!(out, "\n[commits]")?;
        for commit in aliases.commits.iter().map(|id| {
            commits
                .iter()
                .find(|commit| commit.id == *id)
                .expect("aliases only contain captured commits")
        }) {
            writeln!(out, "{}", aliases.id(commit.id))?;
            for line in commit.data.lines() {
                if line.is_empty() {
                    writeln!(out)?;
                } else if let Some((name, value)) = line.split_once_str(b" ")
                    && matches!(name, b"tree" | b"parent")
                    && let Ok(id) = ObjectId::from_hex(value)
                {
                    writeln!(out, "  {} {}", name.as_bstr(), aliases.id(id))?;
                } else {
                    writeln!(out, "  {}", line.as_bstr())?;
                }
            }
            writeln!(out)?;
        }

        writeln!(out, "[index]")?;
        match index_tree {
            Some(id) => writeln!(out, "tree = {}", aliases.id(*id))?,
            None => writeln!(out, "tree = conflicted")?,
        }
        for entry in index {
            writeln!(
                out,
                "{:06o} {} stage={} {:?}",
                entry.mode,
                aliases.id(entry.id),
                entry.stage,
                entry.path.as_bstr()
            )?;
        }

        writeln!(out, "\n[worktree]")?;
        for entry in worktree {
            let mode = entry.unix_mode.map_or_else(|| "-".into(), |mode| format!("{mode:06o}"));
            match &entry.kind {
                WorktreeEntryKind::Directory => writeln!(out, "{mode} dir  {:?}", portable_path(&entry.path))?,
                WorktreeEntryKind::File(data) => writeln!(
                    out,
                    "{mode} file {:?} = {:?}",
                    portable_path(&entry.path),
                    data.as_bstr()
                )?,
                WorktreeEntryKind::Symlink(target) => writeln!(
                    out,
                    "{mode} link {:?} -> {:?}",
                    portable_path(&entry.path),
                    portable_path(target)
                )?,
            }
        }

        if *show_object_ids {
            writeln!(out, "\n[objects]")?;
            for (id, alias) in &aliases.by_id {
                writeln!(out, "{alias} = {id}")?;
            }
        }
        Ok(())
    }
}

/// Return a commit's zero-based generation number: roots have generation zero and every other commit has one more
/// than its highest-generation parent. This is analogous to Git's v1 commit-graph generation numbers, except for the
/// zero-based root, and is used only to assign deterministic parent-before-child snapshot aliases. Parents absent from
/// the captured graph, such as those beyond a shallow boundary, act as roots; `cache` avoids traversing shared history
/// repeatedly.
fn commit_depth(
    id: ObjectId,
    parents: &BTreeMap<ObjectId, Vec<ObjectId>>,
    cache: &mut BTreeMap<ObjectId, usize>,
) -> usize {
    if let Some(depth) = cache.get(&id) {
        return *depth;
    }
    let depth = parents
        .get(&id)
        .into_iter()
        .flatten()
        .map(|parent| commit_depth(*parent, parents, cache) + 1)
        .max()
        .unwrap_or_default();
    cache.insert(id, depth);
    depth
}

fn portable_path(path: &Path) -> Cow<'_, BStr> {
    gix_path::to_unix_separators_on_windows(gix_path::into_bstr(path))
}

fn worktree(root: Option<&Path>) -> Result<Vec<WorktreeEntry>> {
    let Some(root) = root else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    visit_worktree(root, root, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn visit_worktree(root: &Path, directory: &Path, out: &mut Vec<WorktreeEntry>) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(directory)?.collect::<std::io::Result<_>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        if entry.file_name() == OsStr::new(".git") {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        let relative = path.strip_prefix(root)?.to_owned();
        let kind = if metadata.file_type().is_symlink() {
            WorktreeEntryKind::Symlink(fs::read_link(&path)?)
        } else if metadata.is_dir() {
            WorktreeEntryKind::Directory
        } else {
            WorktreeEntryKind::File(fs::read(&path)?)
        };
        out.push(WorktreeEntry {
            path: relative,
            kind,
            unix_mode: unix_mode(&metadata),
        });
        if metadata.is_dir() {
            visit_worktree(root, &path, out)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn unix_mode(metadata: &fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(metadata.permissions().mode())
}

#[cfg(not(unix))]
fn unix_mode(_metadata: &fs::Metadata) -> Option<u32> {
    None
}
