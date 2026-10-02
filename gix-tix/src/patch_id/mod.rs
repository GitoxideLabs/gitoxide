use std::str::FromStr;

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BStr, BString, ByteSlice},
    diff::{blob, tree::recorder::Change},
};

pub(crate) const HEADER: &str = "patch-id";
pub(crate) const UNAVAILABLE: &[u8] = b"v1 unavailable";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) struct PatchId(ObjectId);

impl std::fmt::Display for PatchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = [0; gix::hash::Kind::longest().len_in_bytes() * 4];
        for (&byte, encoded) in self.0.as_slice().iter().zip(out.as_chunks_mut::<4>().0) {
            for (shift, digit) in [6, 4, 2, 0].into_iter().zip(encoded) {
                *digit = b'g' + ((byte >> shift) & 3);
            }
        }
        f.write_str(std::str::from_utf8(&out[..self.0.as_slice().len() * 4]).map_err(|_| std::fmt::Error)?)
    }
}

impl FromStr for PatchId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        anyhow::ensure!(
            gix::hash::Kind::all()
                .iter()
                .any(|kind| kind.len_in_bytes() * 4 == value.len()),
            "patch ID has an invalid length"
        );
        anyhow::ensure!(
            value.bytes().all(|byte| (b'g'..=b'j').contains(&byte)),
            "patch ID must use only g, h, i, and j"
        );
        let mut decoded = gix::hash::Kind::buf();
        for (encoded, byte) in value.as_bytes().as_chunks::<4>().0.iter().zip(&mut decoded) {
            *byte = encoded.iter().fold(0, |byte, digit| (byte << 2) | (digit - b'g'));
        }
        ObjectId::try_from(&decoded[..value.len() / 4])
            .map(PatchId)
            .context("patch ID has an unsupported hash kind")
    }
}

#[derive(Clone, Copy)]
struct Header {
    id: PatchId,
    base_tree_id: ObjectId,
    tree_id: ObjectId,
}

pub(crate) struct RefreshOutcome {
    #[cfg(test)]
    pub id: PatchId,
    #[cfg(test)]
    pub tree_walks: usize,
    #[cfg(test)]
    pub blob_reads: usize,
    #[cfg(test)]
    pub text_diffs: usize,
}

#[derive(Default)]
struct Work {
    tree_walks: usize,
    blob_reads: usize,
    text_diffs: usize,
}

impl RefreshOutcome {
    fn new(id: PatchId, work: Work) -> Self {
        #[cfg(not(test))]
        let _ = (id, work);
        Self {
            #[cfg(test)]
            id,
            #[cfg(test)]
            tree_walks: work.tree_walks,
            #[cfg(test)]
            blob_reads: work.blob_reads,
            #[cfg(test)]
            text_diffs: work.text_diffs,
        }
    }
}

/// Read only commit metadata. Missing, stale, and pending identities are not displayable.
pub(crate) fn for_commit(repo: &gix::Repository, commit_id: ObjectId) -> Result<Option<PatchId>> {
    let commit = repo
        .find_commit(commit_id)
        .context("could not read the patch identity commit")?;
    current(
        repo,
        &commit.decode().context("could not decode the patch identity commit")?,
    )
}

pub(crate) fn current(repo: &gix::Repository, commit: &gix::objs::CommitRef<'_>) -> Result<Option<PatchId>> {
    if commit.extra_headers.iter().any(|(name, value)| {
        (*name == "tix-rebase-parent" || *name == "tix-rebase-merge")
            || ((*name == "gpgsig" || *name == "gpgsig-sha256") && value.is_empty())
    }) {
        return Ok(None);
    }
    let Some(header) = stored(commit.extra_headers().find_all(HEADER))? else {
        return Ok(None);
    };
    if header.tree_id != commit.tree() || header.id.0.kind() != repo.object_hash() {
        return Ok(None);
    }
    Ok((header.base_tree_id == parent_tree(repo, commit.parents().next())?).then_some(header.id))
}

/// Refresh a final commit's cache before signing. Lazy rewrites keep their existing header instead.
pub(crate) fn refresh(repo: &gix::Repository, commit: &mut gix::objs::Commit) -> Result<RefreshOutcome> {
    anyhow::ensure!(
        commit.extra_headers().find("tix-rebase-parent").is_none()
            && commit.extra_headers().find("tix-rebase-merge").is_none(),
        "a pending rebase cannot refresh its patch identity"
    );
    anyhow::ensure!(
        !is_unavailable(commit),
        "an unresolved conflict cannot refresh its patch identity"
    );
    let base_tree_id = parent_tree(repo, commit.parents.first().copied())?;
    let mut work = Work::default();
    let previous = match stored(commit.extra_headers().find_all(HEADER)) {
        Ok(header) => header.filter(|header| header.id.0.kind() == repo.object_hash()),
        Err(err) => {
            tracing::warn!(error = %err, "replacing malformed patch identity while rewriting a final commit");
            None
        }
    };
    if let Some(header) = previous
        && header.base_tree_id == base_tree_id
        && header.tree_id == commit.tree
    {
        return Ok(RefreshOutcome::new(header.id, work));
    }

    let current_changes = changes(repo, base_tree_id, commit.tree, &mut work)?;
    let reused = if let Some(header) = previous {
        // The previous trees may have been pruned: the cache must never make a final rewrite fail.
        match changes(repo, header.base_tree_id, header.tree_id, &mut work) {
            Ok(previous) if previous == current_changes => Some(header.id),
            Ok(_) => None,
            Err(err) => {
                tracing::debug!(error = %err, "could not reuse previous patch identity trees");
                None
            }
        }
    } else {
        None
    };
    let id = match reused {
        Some(id) => id,
        None => fingerprint(repo, &current_changes, &mut work)?,
    };
    store(commit, format!("v1 {id} {base_tree_id} {}", commit.tree).into());
    Ok(RefreshOutcome::new(id, work))
}

pub(crate) fn mark_unavailable(commit: &mut gix::objs::Commit) {
    store(commit, UNAVAILABLE.into());
}

pub(crate) fn is_unavailable(commit: &gix::objs::Commit) -> bool {
    commit
        .extra_headers()
        .find_all(HEADER)
        .any(|value| value == UNAVAILABLE)
}

pub(crate) fn clear_unavailable(commit: &mut gix::objs::Commit) {
    commit
        .extra_headers
        .retain(|(name, value)| name != HEADER || value.as_slice() != UNAVAILABLE);
}

fn store(commit: &mut gix::objs::Commit, value: BString) {
    commit.extra_headers.retain(|(name, _)| name != HEADER);
    commit.extra_headers.push((HEADER.into(), value));
}

fn stored<'a>(mut values: impl Iterator<Item = &'a BStr>) -> Result<Option<Header>> {
    let Some(value) = values.next() else {
        return Ok(None);
    };
    anyhow::ensure!(values.next().is_none(), "a commit has multiple patch-id headers");
    if value == UNAVAILABLE {
        return Ok(None);
    }
    let mut fields = value.split(|byte| *byte == b' ');
    let (Some(b"v1"), Some(encoded), Some(base), Some(tree), None) = (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) else {
        anyhow::bail!("patch-id must contain v1, a patch ID, and its base and result tree IDs");
    };
    let id: PatchId = std::str::from_utf8(encoded).context("patch ID is not ASCII")?.parse()?;
    let base_tree_id = ObjectId::from_hex(base).context("patch ID has an invalid base tree")?;
    let tree_id = ObjectId::from_hex(tree).context("patch ID has an invalid result tree")?;
    anyhow::ensure!(
        id.0.kind() == base_tree_id.kind() && id.0.kind() == tree_id.kind(),
        "patch ID and its tree IDs must use the same hash kind"
    );
    Ok(Some(Header {
        id,
        base_tree_id,
        tree_id,
    }))
}

fn parent_tree(repo: &gix::Repository, parent_commit_id: Option<ObjectId>) -> Result<ObjectId> {
    match parent_commit_id {
        Some(parent_commit_id) => repo
            .find_commit(parent_commit_id)
            .context("could not read the patch's first parent")?
            .tree_id()
            .map(gix::Id::detach)
            .context("could not read the patch's base tree ID"),
        None => Ok(repo.object_hash().empty_tree()),
    }
}

fn changes(repo: &gix::Repository, base_tree_id: ObjectId, tree_id: ObjectId, work: &mut Work) -> Result<Vec<Change>> {
    if base_tree_id == tree_id {
        return Ok(Vec::new());
    }
    work.tree_walks += 1;
    let base = repo
        .find_tree(base_tree_id)
        .context("could not read the patch's base tree")?;
    let tree = repo
        .find_tree(tree_id)
        .context("could not read the patch's result tree")?;
    let mut recorder = gix::diff::tree::Recorder::default();
    // ponytail: renames remain deletion/addition pairs; use exact rename matching if inherited rename content must retain reviews.
    gix::diff::tree(
        gix::objs::TreeRefIter::from_bytes(&base.data, base.id.kind()),
        gix::objs::TreeRefIter::from_bytes(&tree.data, tree.id.kind()),
        &mut gix::diff::tree::State::default(),
        repo,
        &mut recorder,
    )
    .context("could not read the patch's changed entries")?;
    recorder.records.retain_mut(|change| match change {
        Change::Addition {
            entry_mode, relation, ..
        }
        | Change::Deletion {
            entry_mode, relation, ..
        } => {
            *relation = None;
            !entry_mode.is_tree()
        }
        Change::Modification { entry_mode, .. } => !entry_mode.is_tree(),
    });
    recorder.records.sort_by(|a, b| path(a).cmp(path(b)));
    Ok(recorder.records)
}

fn path(change: &Change) -> &BStr {
    match change {
        Change::Addition { path, .. } | Change::Deletion { path, .. } | Change::Modification { path, .. } => {
            path.as_bstr()
        }
    }
}

fn fingerprint(repo: &gix::Repository, changes: &[Change], work: &mut Work) -> Result<PatchId> {
    let mut hash = gix::hash::hasher(repo.object_hash());
    hash.update(b"tix-patch-id\0v1\0");
    for change in changes {
        hash_bytes(&mut hash, path(change));
        match change {
            Change::Addition { entry_mode, oid, .. } | Change::Deletion { entry_mode, oid, .. } => {
                hash.update(if matches!(change, Change::Addition { .. }) {
                    b"+"
                } else {
                    b"-"
                });
                hash.update(&entry_mode.value().to_be_bytes());
                // All bytes change for an addition/deletion; their existing object hash is sufficient.
                hash.update(oid.as_slice());
            }
            Change::Modification {
                previous_entry_mode,
                previous_oid,
                entry_mode,
                oid,
                ..
            } => {
                hash.update(b"m");
                hash.update(&previous_entry_mode.value().to_be_bytes());
                hash.update(&entry_mode.value().to_be_bytes());
                if previous_oid == oid {
                    hash.update(b"=");
                } else if !previous_entry_mode.is_blob_or_symlink() || !entry_mode.is_blob_or_symlink() {
                    hash.update(b"o");
                    hash.update(previous_oid.as_slice());
                    hash.update(oid.as_slice());
                } else {
                    work.blob_reads += 2;
                    let before = repo
                        .find_blob(*previous_oid)
                        .context("could not read the patch's old blob")?;
                    let after = repo.find_blob(*oid).context("could not read the patch's new blob")?;
                    if before.data.contains(&0) || after.data.contains(&0) {
                        hash.update(b"o");
                        hash.update(previous_oid.as_slice());
                        hash.update(oid.as_slice());
                    } else {
                        hash.update(b"t");
                        work.text_diffs += 1;
                        hash_text(&mut hash, &before.data, &after.data);
                    }
                }
            }
        }
    }
    hash.try_finalize()
        .map(PatchId)
        .context("could not hash the patch identity")
}

fn hash_bytes(hash: &mut gix::hash::Hasher, bytes: &[u8]) {
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

fn hash_text(hash: &mut gix::hash::Hasher, before: &[u8], after: &[u8]) {
    let input = blob::InternedInput::new(before, after);
    let diff = blob::Diff::compute(blob::Algorithm::Histogram, &input);
    // Location and hunk grouping are deliberately absent: unchanged context may split a hunk after replay.
    for removed in [true, false] {
        let tokens = if removed { &input.before } else { &input.after };
        let selected = || {
            tokens.iter().enumerate().filter_map(|(index, token)| {
                let changed = if removed {
                    diff.is_removed(index as u32)
                } else {
                    diff.is_added(index as u32)
                };
                changed.then_some(input.interner[*token])
            })
        };
        let len: u64 = selected().map(|bytes| bytes.len() as u64).sum();
        hash.update(&len.to_be_bytes());
        for bytes in selected() {
            hash.update(bytes);
        }
    }
}

#[cfg(test)]
mod tests;
