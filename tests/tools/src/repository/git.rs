use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use bstr::{BString, ByteSlice};
use gix_hash::{Kind, ObjectId};

use super::{Commit, Head, IndexEntry, Reference, ReferenceTarget, State};
use crate::Result;

pub(super) fn snapshot(path: &Path) -> Result<State> {
    let hash = object_hash(path)?;
    let common_dir = common_dir(path)?;
    let config = std::fs::read(common_dir.join("config"))?.into();
    let symbolic_head = git_optional(path, ["symbolic-ref", "-q", "HEAD"])?.map(|out| out.trim().to_owned());
    let head_id = git_optional(path, ["rev-parse", "--verify", "HEAD"])?
        .map(|out| parse_id(out.trim(), hash))
        .transpose()?;
    let head = match (symbolic_head, head_id) {
        (Some(name), Some(id)) => Head::Symbolic { name: name.into(), id },
        (Some(name), None) => Head::Unborn(name.into()),
        (None, Some(id)) => Head::Detached(id),
        (None, None) => return Err("HEAD is neither symbolic, detached, nor unborn".into()),
    };
    let (references, mut roots) = references(path, hash)?;
    roots.extend(head_id);
    let shallow = shallow_commits(path, hash)?;
    let commits = commits(path, roots, hash, &shallow)?;
    let index = index(path, hash)?;
    let index_tree = index_tree(path, &index, hash)?;
    let worktree_root = worktree_root(path)?;
    let worktree = super::worktree(worktree_root.as_deref())?;
    Ok(State {
        head,
        config,
        references,
        commits,
        index,
        index_tree,
        worktree,
        normalization_root: common_dir,
        show_object_ids: false,
    })
}

fn object_hash(path: &Path) -> Result<Kind> {
    match git(path, ["rev-parse", "--show-object-format"])?.trim() {
        #[cfg(feature = "sha1")]
        b"sha1" => Ok(Kind::Sha1),
        #[cfg(feature = "sha256")]
        b"sha256" => Ok(Kind::Sha256),
        value => Err(format!("unsupported object format: {}", value.as_bstr()).into()),
    }
}

fn references(path: &Path, hash: Kind) -> Result<(Vec<Reference>, Vec<ObjectId>)> {
    let output = git(
        path,
        [
            "for-each-ref",
            "--format=%(refname)%00%(symref)%00%(objectname)",
            "refs/",
        ],
    )?;
    let mut out = Vec::new();
    let mut roots = Vec::new();
    for record in output.lines().filter(|line| !line.is_empty()) {
        let mut fields = record.split(|byte| *byte == 0);
        let name = fields.next().ok_or("reference output lacks a name")?;
        let symbolic = fields.next().ok_or("reference output lacks a symbolic target")?;
        let id = fields.next().ok_or("reference output lacks an object target")?;
        let resolved = (!id.is_empty()).then(|| parse_id(id, hash)).transpose()?;
        roots.extend(resolved);
        let target = if symbolic.is_empty() {
            ReferenceTarget::Object(resolved.ok_or("a direct reference lacks an object target")?)
        } else {
            ReferenceTarget::Symbolic(symbolic.into())
        };
        out.push(Reference {
            name: name.into(),
            target,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((out, roots))
}

fn common_dir(path: &Path) -> Result<PathBuf> {
    let common_dir = git(path, ["rev-parse", "--git-common-dir"])?;
    let common_dir = PathBuf::from(String::from_utf8(common_dir.trim().to_owned())?);
    Ok(if common_dir.is_absolute() {
        common_dir
    } else {
        path.join(common_dir)
    })
}

fn shallow_commits(path: &Path, hash: Kind) -> Result<BTreeSet<ObjectId>> {
    let data = match std::fs::read(common_dir(path)?.join("shallow")) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(err) => return Err(err.into()),
    };
    data.lines().map(|line| parse_id(line, hash)).collect()
}

fn commits(path: &Path, roots: Vec<ObjectId>, hash: Kind, shallow: &BTreeSet<ObjectId>) -> Result<Vec<Commit>> {
    let mut pending = Vec::new();
    for id in roots {
        if let Some(id) = peel_to_commit(path, id, hash)? {
            pending.push(id);
        }
    }
    let mut seen = BTreeSet::new();
    let mut commits = Vec::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            continue;
        }
        let hex = id.to_string();
        let data = git(path, ["cat-file", "commit", &hex])?;
        if !shallow.contains(&id) {
            for parent in data
                .lines()
                .take_while(|line| !line.is_empty())
                .filter_map(|line| line.strip_prefix(b"parent "))
            {
                pending.push(parse_id(parent, hash)?);
            }
        }
        commits.push(Commit { id, data });
    }
    commits.sort_by_key(|commit| commit.id);
    Ok(commits)
}

fn peel_to_commit(path: &Path, mut id: ObjectId, hash: Kind) -> Result<Option<ObjectId>> {
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(id) {
            return Err(format!("tag cycle at {id}").into());
        }
        let hex = id.to_string();
        match git(path, ["cat-file", "-t", &hex])?.trim() {
            b"commit" => return Ok(Some(id)),
            b"tag" => {
                let data = git(path, ["cat-file", "tag", &hex])?;
                let target = data
                    .lines()
                    .next()
                    .and_then(|line| line.strip_prefix(b"object "))
                    .ok_or("tag object lacks its target")?;
                id = parse_id(target, hash)?;
            }
            _ => return Ok(None),
        }
    }
}

fn index(path: &Path, hash: Kind) -> Result<Vec<IndexEntry>> {
    let output = git(path, ["ls-files", "--stage", "-z"])?;
    output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| {
            let (metadata, path) = record.split_once_str(b"\t").ok_or("index entry lacks a path")?;
            let mut fields = metadata.split(|byte| *byte == b' ');
            let mode = u32::from_str_radix(
                std::str::from_utf8(fields.next().ok_or("index entry lacks a mode")?)?,
                8,
            )?;
            let id = parse_id(fields.next().ok_or("index entry lacks an object ID")?, hash)?;
            let stage = std::str::from_utf8(fields.next().ok_or("index entry lacks a stage")?)?.parse::<u8>()?;
            Ok(IndexEntry {
                mode,
                id,
                stage,
                path: path.into(),
            })
        })
        .collect()
}

#[derive(Default)]
struct Tree {
    entries: BTreeMap<BString, TreeEntry>,
}

enum TreeEntry {
    Tree(Tree),
    Leaf { mode: u32, id: ObjectId },
}

fn index_tree(path: &Path, entries: &[IndexEntry], hash: Kind) -> Result<Option<ObjectId>> {
    if entries.iter().any(|entry| entry.stage != 0) {
        return Ok(None);
    }
    let mut root = Tree::default();
    for entry in entries {
        insert(&mut root, entry.path.as_ref(), entry.mode, entry.id)?;
    }
    Ok(Some(hash_tree(path, &root, hash)?))
}

fn insert(tree: &mut Tree, path: &[u8], mode: u32, id: ObjectId) -> Result<()> {
    let mut components = path.split(|byte| *byte == b'/').peekable();
    let mut tree = tree;
    while let Some(component) = components.next() {
        if components.peek().is_none() {
            tree.entries.insert(component.into(), TreeEntry::Leaf { mode, id });
            return Ok(());
        }
        tree = match tree
            .entries
            .entry(component.into())
            .or_insert_with(|| TreeEntry::Tree(Tree::default()))
        {
            TreeEntry::Tree(tree) => tree,
            TreeEntry::Leaf { .. } => return Err("an index path traverses a non-tree entry".into()),
        };
    }
    Err("an index entry has an empty path".into())
}

fn hash_tree(path: &Path, tree: &Tree, hash: Kind) -> Result<ObjectId> {
    let mut entries: Vec<_> = tree.entries.iter().collect();
    entries.sort_by(|(a, a_entry), (b, b_entry)| tree_name_cmp(a, a_entry, b, b_entry));
    let mut data = Vec::new();
    for (name, entry) in entries {
        let (mode, id) = match entry {
            TreeEntry::Tree(tree) => (0o40000, hash_tree(path, tree, hash)?),
            TreeEntry::Leaf { mode, id } => (*mode, *id),
        };
        data.extend_from_slice(format!("{mode:o}").as_bytes());
        data.push(b' ');
        data.extend_from_slice(name);
        data.push(0);
        data.extend_from_slice(id.as_bytes());
    }
    let id = git_with_input(path, ["hash-object", "-t", "tree", "--stdin"], &data)?;
    parse_id(id.trim(), hash)
}

fn tree_name_cmp(a: &[u8], a_entry: &TreeEntry, b: &[u8], b_entry: &TreeEntry) -> Ordering {
    let mut a = a
        .iter()
        .copied()
        .chain(std::iter::once(if matches!(a_entry, TreeEntry::Tree(_)) {
            b'/'
        } else {
            0
        }));
    let mut b = b
        .iter()
        .copied()
        .chain(std::iter::once(if matches!(b_entry, TreeEntry::Tree(_)) {
            b'/'
        } else {
            0
        }));
    loop {
        match (a.next(), b.next()) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => return a.cmp(&b),
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
        }
    }
}

fn worktree_root(path: &Path) -> Result<Option<PathBuf>> {
    git_optional(path, ["rev-parse", "--show-toplevel"])?
        .map(|root| String::from_utf8(root.trim().to_owned()).map(PathBuf::from))
        .transpose()
        .map_err(Into::into)
}

fn parse_id(input: &[u8], hash: Kind) -> Result<ObjectId> {
    let id = ObjectId::from_hex(input)?;
    if id.kind() != hash {
        return Err("object ID uses the wrong hash kind".into());
    }
    Ok(id)
}

fn git<const N: usize>(path: &Path, args: [&str; N]) -> Result<Vec<u8>> {
    git_os(path, args.iter().map(OsStr::new))
}

fn git_os<'a>(path: &Path, args: impl IntoIterator<Item = &'a OsStr>) -> Result<Vec<u8>> {
    let output = Command::new(gix_path::env::exe_invocation())
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .output()?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(format!(
        "git command failed with {}: {}",
        output.status,
        output.stderr.as_bstr().trim().as_bstr()
    )
    .into())
}

fn git_with_input<const N: usize>(path: &Path, args: [&str; N], input: &[u8]) -> Result<Vec<u8>> {
    let mut child = Command::new(gix_path::env::exe_invocation())
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().expect("configured as piped").write_all(input)?;
    let output = child.wait_with_output()?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(format!(
        "git command failed with {}: {}",
        output.status,
        output.stderr.as_bstr().trim().as_bstr()
    )
    .into())
}

fn git_optional<const N: usize>(path: &Path, args: [&str; N]) -> Result<Option<Vec<u8>>> {
    let output = Command::new(gix_path::env::exe_invocation())
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .output()?;
    Ok(output.status.success().then_some(output.stdout))
}
