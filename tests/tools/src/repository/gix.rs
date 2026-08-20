use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use bstr::{BString, ByteSlice};
use gix_hash::{Kind, ObjectId};
use gix_object::FindExt;

use super::{Commit, Head, IndexEntry, Reference, ReferenceTarget, State};
use crate::Result;

pub(super) fn snapshot(path: &Path) -> Result<State> {
    let (discovered, _trust) = gix_discover::upwards(path)?;
    let (git_dir, discovered_work_dir) = discovered.into_repository_and_work_tree_directories();
    let git_dir = gix_path::realpath(git_dir)?;
    let common_dir = gix_discover::path::from_plain_file_relative_to_file(&git_dir.join("commondir"))
        .transpose()?
        .map(gix_path::realpath)
        .transpose()?
        .unwrap_or_else(|| git_dir.clone());
    let config_path = common_dir.join("config");
    let config_data = std::fs::read(&config_path)?.into();
    let config = gix_config::File::from_path_no_includes(config_path, gix_config::Source::Local)?;
    let hash = object_hash(&config)?;
    let worktree_config = worktree_config(&git_dir, &config)?;
    let work_dir = worktree_from_config(&git_dir, discovered_work_dir, &config, worktree_config.as_ref())?;
    let refs = ref_store(&git_dir, &common_dir, hash);
    let (head, head_id) = head(&refs)?;
    let (references, mut roots) = references(&refs)?;
    roots.extend(head_id);
    let objects = gix_odb::at(common_dir.join("objects"), hash)?;
    let shallow = shallow_commits(&common_dir, hash)?;
    let commits = commits(&objects, roots, hash, &shallow)?;
    let index = index(&git_dir, hash, &objects)?;
    let index_tree = index_tree(&index, hash)?;
    let worktree = super::worktree(work_dir.as_deref())?;
    Ok(State {
        head,
        config: config_data,
        references,
        commits,
        index,
        index_tree,
        worktree,
        normalization_root: common_dir,
        show_object_ids: false,
    })
}

fn object_hash(config: &gix_config::File) -> Result<Kind> {
    let version = config
        .integer_by("core", None, "repositoryFormatVersion")?
        .unwrap_or_default();
    let format = config.string_by("extensions", None, "objectFormat");
    match (version, format) {
        (0 | 1, None) => supported_sha1(),
        (1, Some(format)) if format.trim().eq_ignore_ascii_case(b"sha1") => supported_sha1(),
        (1, Some(format)) if format.trim().eq_ignore_ascii_case(b"sha256") => supported_sha256(),
        (0, Some(_)) => Err("extensions.objectFormat requires repository format version 1".into()),
        (version, Some(format)) => {
            Err(format!("unsupported object format {format:?} for repository format version {version}").into())
        }
        (version, None) => Err(format!("unsupported repository format version {version}").into()),
    }
}

fn worktree_config(git_dir: &Path, config: &gix_config::File) -> Result<Option<gix_config::File>> {
    if !config
        .boolean_by("extensions", None, "worktreeConfig")?
        .unwrap_or(false)
    {
        return Ok(None);
    }
    let path = git_dir.join("config.worktree");
    Ok(path
        .is_file()
        .then(|| gix_config::File::from_path_no_includes(path, gix_config::Source::Worktree))
        .transpose()?)
}

fn worktree_from_config(
    git_dir: &Path,
    discovered: Option<PathBuf>,
    config: &gix_config::File,
    worktree_config: Option<&gix_config::File>,
) -> Result<Option<PathBuf>> {
    let bare = worktree_config
        .map(|config| config.boolean_by("core", None, "bare"))
        .transpose()?
        .flatten()
        .or(config.boolean_by("core", None, "bare")?)
        .unwrap_or(false);
    if bare {
        return Ok(None);
    }
    let Some(path) = worktree_config
        .and_then(|config| config.string_by("core", None, "worktree"))
        .or_else(|| config.string_by("core", None, "worktree"))
    else {
        return Ok(discovered);
    };
    let path = gix_path::from_bstr(path.trim().as_bstr()).into_owned();
    if path.as_os_str().is_empty() {
        return Err("core.worktree cannot be empty".into());
    }
    Ok(Some(if path.is_absolute() { path } else { git_dir.join(path) }))
}

fn supported_sha1() -> Result<Kind> {
    #[cfg(feature = "sha1")]
    {
        Ok(Kind::Sha1)
    }
    #[cfg(not(feature = "sha1"))]
    {
        Err("SHA-1 support is not compiled in".into())
    }
}

fn supported_sha256() -> Result<Kind> {
    #[cfg(feature = "sha256")]
    {
        Ok(Kind::Sha256)
    }
    #[cfg(not(feature = "sha256"))]
    {
        Err("SHA-256 support is not compiled in".into())
    }
}

fn ref_store(git_dir: &Path, common_dir: &Path, hash: Kind) -> gix_ref::file::Store {
    let options = gix_ref::store::init::Options {
        write_reflog: gix_ref::store::WriteReflog::Disable,
        precompose_unicode: false,
        prohibit_windows_device_names: cfg!(windows),
    };
    if git_dir == common_dir {
        gix_ref::file::Store::at_opts(git_dir.to_owned(), hash, options)
    } else {
        gix_ref::file::Store::for_linked_worktree_opts(git_dir.to_owned(), common_dir.to_owned(), hash, options)
    }
}

fn head(store: &gix_ref::file::Store) -> Result<(Head, Option<ObjectId>)> {
    let head = store.try_find_loose("HEAD")?.ok_or("HEAD does not exist")?;
    match head.target {
        gix_ref::Target::Object(id) => Ok((Head::Detached(id), Some(id))),
        gix_ref::Target::Symbolic(name) => {
            let resolved = resolve_symbolic(store, name)?;
            if resolved.is_broken {
                return Err(format!(
                    "HEAD resolves through broken reference {}",
                    resolved.name.as_ref().as_bstr()
                )
                .into());
            }
            let name = resolved.name.as_ref().as_bstr().to_owned();
            Ok(match resolved.id {
                Some(id) => (Head::Symbolic { name, id }, Some(id)),
                None => (Head::Unborn(name), None),
            })
        }
    }
}

struct SymbolicResolution {
    name: gix_ref::FullName,
    id: Option<ObjectId>,
    is_broken: bool,
}

fn resolve_symbolic(store: &gix_ref::file::Store, mut name: gix_ref::FullName) -> Result<SymbolicResolution> {
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(name.clone()) {
            return Ok(SymbolicResolution {
                name,
                id: None,
                is_broken: true,
            });
        }
        let reference = match store.try_find(name.as_ref()) {
            Ok(Some(reference)) => reference,
            Ok(None) => {
                return Ok(SymbolicResolution {
                    name,
                    id: None,
                    is_broken: false,
                });
            }
            Err(gix_ref::file::find::Error::ReferenceCreation { .. }) => {
                return Ok(SymbolicResolution {
                    name,
                    id: None,
                    is_broken: true,
                });
            }
            Err(err) => return Err(err.into()),
        };
        match reference.target {
            gix_ref::Target::Object(id) => {
                return Ok(SymbolicResolution {
                    name,
                    id: Some(id),
                    is_broken: false,
                });
            }
            gix_ref::Target::Symbolic(next) => name = next,
        }
    }
}

fn references(store: &gix_ref::file::Store) -> Result<(Vec<Reference>, Vec<ObjectId>)> {
    let platform = store.iter()?;
    let mut references = Vec::new();
    let mut roots = Vec::new();
    for reference in platform.all()? {
        let reference = match reference {
            Ok(reference) => reference,
            Err(gix_ref::file::iter::loose_then_packed::Error::ReferenceCreation { .. }) => continue,
            Err(err) => return Err(err.into()),
        };
        if !reference.name.as_ref().as_bstr().starts_with(b"refs/") {
            continue;
        }
        let target = match reference.target {
            gix_ref::Target::Object(id) => {
                roots.push(id);
                ReferenceTarget::Object(id)
            }
            gix_ref::Target::Symbolic(name) => {
                let resolved = resolve_symbolic(store, name.clone())?;
                if resolved.is_broken {
                    continue;
                }
                roots.extend(resolved.id);
                ReferenceTarget::Symbolic(name.as_ref().as_bstr().to_owned())
            }
        };
        references.push(Reference {
            name: reference.name.as_ref().as_bstr().to_owned(),
            target,
        });
    }
    Ok((references, roots))
}

fn shallow_commits(common_dir: &Path, hash: Kind) -> Result<BTreeSet<ObjectId>> {
    let mut out = BTreeSet::new();
    if let Some(boundaries) = gix_shallow::read(&common_dir.join("shallow"))? {
        for id in boundaries {
            if id.kind() != hash {
                return Err(format!("shallow boundary {id} uses the wrong hash kind").into());
            }
            out.insert(id);
        }
    }
    Ok(out)
}

fn commits(
    objects: &gix_odb::Handle,
    roots: Vec<ObjectId>,
    hash: Kind,
    shallow: &BTreeSet<ObjectId>,
) -> Result<Vec<Commit>> {
    let mut pending = Vec::new();
    for id in roots {
        if let Some(id) = peel_to_commit(objects, id, hash)? {
            pending.push(id);
        }
    }
    let mut seen = BTreeSet::new();
    let mut commits = Vec::new();
    let mut buf = Vec::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            continue;
        }
        let data = objects.find(id.as_ref(), &mut buf)?;
        if data.kind != gix_object::Kind::Commit {
            return Err(format!("expected commit {id}, got {}", data.kind).into());
        }
        if !shallow.contains(&id) {
            pending.extend(gix_object::CommitRefIter::from_bytes(data.data, hash).parent_ids());
        }
        commits.push(Commit {
            id,
            data: data.data.to_owned(),
        });
    }
    commits.sort_by_key(|commit| commit.id);
    Ok(commits)
}

fn peel_to_commit(objects: &gix_odb::Handle, mut id: ObjectId, hash: Kind) -> Result<Option<ObjectId>> {
    let mut seen = BTreeSet::new();
    let mut buf = Vec::new();
    loop {
        if !seen.insert(id) {
            return Err(format!("tag cycle at {id}").into());
        }
        let data = objects.find(id.as_ref(), &mut buf)?;
        match data.kind {
            gix_object::Kind::Commit => return Ok(Some(id)),
            gix_object::Kind::Tag => id = gix_object::TagRefIter::from_bytes(data.data, hash).target_id()?,
            _ => return Ok(None),
        }
    }
}

fn index(git_dir: &Path, hash: Kind, objects: &gix_odb::Handle) -> Result<Vec<IndexEntry>> {
    let index = gix_index::File::at_or_default(git_dir.join("index"), hash, false, Default::default())?;
    let mut out = Vec::new();
    for entry in index.entries() {
        let path = entry.path(&index);
        if entry.mode.is_sparse() {
            let prefix = path
                .strip_suffix(b"/")
                .ok_or("a sparse-index directory path must end in a slash")?;
            let expanded = gix_index::State::from_tree(
                entry.id.as_ref(),
                objects,
                gix_index::validate::path::component::Options::default(),
            )?;
            for entry in expanded.entries() {
                let mut path = BString::from(prefix);
                if !path.is_empty() {
                    path.push(b'/');
                }
                path.extend_from_slice(entry.path(&expanded));
                out.push(IndexEntry {
                    mode: entry.mode.bits(),
                    id: entry.id,
                    stage: entry.stage_raw().try_into()?,
                    path,
                });
            }
        } else {
            out.push(IndexEntry {
                mode: entry.mode.bits(),
                id: entry.id,
                stage: entry.stage_raw().try_into()?,
                path: path.to_owned(),
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path).then(a.stage.cmp(&b.stage)));
    Ok(out)
}

#[derive(Default)]
struct Tree {
    entries: BTreeMap<BString, TreeEntry>,
}

enum TreeEntry {
    Tree(Tree),
    Leaf { mode: u32, id: ObjectId },
}

fn index_tree(entries: &[IndexEntry], hash: Kind) -> Result<Option<ObjectId>> {
    if entries.iter().any(|entry| entry.stage != 0) {
        return Ok(None);
    }
    let mut root = Tree::default();
    for entry in entries {
        insert(&mut root, entry.path.as_ref(), entry.mode, entry.id)?;
    }
    Ok(Some(hash_tree(&root, hash)?))
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

fn hash_tree(tree: &Tree, hash: Kind) -> Result<ObjectId> {
    let mut entries: Vec<_> = tree.entries.iter().collect();
    entries.sort_by(|(a, a_entry), (b, b_entry)| {
        gix_object::tree::name_order(
            a,
            matches!(a_entry, TreeEntry::Tree(_)),
            b,
            matches!(b_entry, TreeEntry::Tree(_)),
        )
    });
    let mut data = Vec::new();
    for (name, entry) in entries {
        let (mode, id) = match entry {
            TreeEntry::Tree(tree) => (0o40000, hash_tree(tree, hash)?),
            TreeEntry::Leaf { mode, id } => (*mode, *id),
        };
        data.extend_from_slice(format!("{mode:o}").as_bytes());
        data.push(b' ');
        data.extend_from_slice(name);
        data.push(0);
        data.extend_from_slice(id.as_bytes());
    }
    gix_object::compute_hash(hash, gix_object::Kind::Tree, &data).map_err(Into::into)
}
