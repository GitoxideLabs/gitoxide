use std::{
    cmp::Ordering as CmpOrdering,
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BStr, BString},
    hash::ChangeId,
};

pub(crate) const HEADER: &str = "change-id";

pub(crate) fn effective<'a>(commit_id: ObjectId, mut values: impl Iterator<Item = &'a BStr>) -> ChangeId {
    values
        .find_map(|value| ChangeId::from_reverse_hex(value).ok())
        .unwrap_or_else(|| commit_id.into())
}

pub(crate) fn for_commit(repo: &gix::Repository, id: ObjectId) -> Result<ChangeId> {
    let object = repo
        .find_commit(id)
        .context("could not read commit for its change ID")?;
    let commit = object.decode().context("could not decode commit for its change ID")?;
    Ok(effective(id, commit.extra_headers().find_all(HEADER)))
}

pub(crate) fn abbreviations(
    repo: &gix::Repository,
    ids: impl IntoIterator<Item = ObjectId>,
    len: usize,
) -> Result<Abbreviations> {
    let values = ids
        .into_iter()
        .map(|id| Ok((id, for_commit(repo, id)?)))
        .collect::<Result<Vec<_>>>()?;
    Ok(collect_abbreviations(values, len))
}

pub(crate) struct Abbreviations {
    pub values: HashMap<ObjectId, ChangeId>,
    pub ambiguous: HashSet<ObjectId>,
}

pub(crate) fn resolve_prefix(
    repo: &gix::Repository,
    prefix: &str,
    ids: impl IntoIterator<Item = ObjectId>,
) -> Result<Option<ObjectId>> {
    let Ok(prefix) = gix::hash::Prefix::from_reverse_hex(prefix) else {
        return Ok(None);
    };
    let mut found = None;
    for id in ids {
        let change_id = for_commit(repo, id)?;
        if prefix.cmp_oid(&change_id) != CmpOrdering::Equal {
            continue;
        }
        if found.replace(id).is_some() {
            anyhow::bail!(
                "change ID prefix {} is ambiguous in the default Tix view",
                prefix.to_reverse_hex()
            );
        }
    }
    Ok(found)
}

fn collect_abbreviations(values: impl IntoIterator<Item = (ObjectId, ChangeId)>, len: usize) -> Abbreviations {
    let mut by_prefix = HashMap::new();
    let mut all = HashMap::new();
    let mut ambiguous = HashSet::new();
    for (id, change_id) in values {
        let prefix = change_id.to_reverse_hex_with_len(len).to_string();
        if let Some(first) = by_prefix.insert(prefix, id) {
            ambiguous.insert(first);
            ambiguous.insert(id);
        }
        all.insert(id, change_id);
    }
    Abbreviations { values: all, ambiguous }
}

pub(crate) fn inherit(repo: &gix::Repository, commit: &mut gix::objs::Commit, predecessor: ObjectId) -> Result<()> {
    let change_id = for_commit(repo, predecessor).context("could not preserve predecessor change ID")?;
    store(commit, change_id);
    Ok(())
}

fn store(commit: &mut gix::objs::Commit, change_id: ChangeId) {
    commit.extra_headers.retain(|(name, _)| name != HEADER);
    commit
        .extra_headers
        .push((HEADER.into(), BString::from(change_id.to_string())));
}

pub(crate) struct Scan {
    pub overrides: HashMap<ObjectId, ChangeId>,
    pub duplicates: HashSet<ObjectId>,
}

pub(crate) fn scan(repo: &gix::Repository, ids: &[ObjectId], cancelled: &AtomicBool) -> Result<Option<Scan>> {
    let mut overrides = HashMap::new();
    let mut first_by_change = HashMap::new();
    let mut duplicates = HashSet::new();
    for &id in ids {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let object = repo
            .find_commit(id)
            .context("could not read commit while scanning change IDs")?;
        let commit = object
            .decode()
            .context("could not decode commit while scanning change IDs")?;
        let change_id = effective(id, commit.extra_headers().find_all(HEADER));
        if change_id != ChangeId::from(id) {
            overrides.insert(id, change_id);
        }
        if let Some(first) = first_by_change.insert(change_id, id) {
            duplicates.insert(first);
            duplicates.insert(id);
        }
    }
    Ok(Some(Scan { overrides, duplicates }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> ObjectId {
        ObjectId::Sha1([byte; 20])
    }

    #[test]
    fn stores_one_canonical_header() {
        let predecessor = id(1);
        let inherited = ChangeId::from(id(2));
        let mut commit = gix::objs::Commit {
            tree: id(3),
            parents: Default::default(),
            author: Default::default(),
            committer: Default::default(),
            encoding: None,
            message: Default::default(),
            extra_headers: vec![
                (HEADER.into(), "invalid".into()),
                (HEADER.into(), inherited.to_string().into()),
            ],
        };

        store(&mut commit, inherited);
        assert_eq!(commit.extra_headers.len(), 1, "the rewrite stores one canonical header");
        assert_eq!(
            effective(predecessor, commit.extra_headers().find_all(HEADER)),
            inherited,
            "the first valid inherited identity wins"
        );
    }

    #[test]
    fn marks_ambiguous_abbreviations_without_omitting_them() -> gix_testtools::Result {
        let shared_a = ChangeId::from_reverse_hex(format!("zzzzzzz{}", "k".repeat(33)).as_bytes())?;
        let shared_b = ChangeId::from_reverse_hex(format!("zzzzzzz{}", "l".repeat(33)).as_bytes())?;
        let duplicate = ChangeId::from_reverse_hex(format!("yyyyyyy{}", "m".repeat(33)).as_bytes())?;
        let unique = ChangeId::from_reverse_hex(format!("xxxxxxx{}", "n".repeat(33)).as_bytes())?;
        let abbreviations = collect_abbreviations(
            [
                (id(1), shared_a),
                (id(2), shared_b),
                (id(3), duplicate),
                (id(4), duplicate),
                (id(5), unique),
            ],
            7,
        );
        assert_eq!(abbreviations.values.len(), 5, "every change ID remains visible");
        assert_eq!(
            abbreviations.ambiguous,
            HashSet::from([id(1), id(2), id(3), id(4)]),
            "prefix collisions and complete duplicates are marked"
        );
        Ok(())
    }

    #[test]
    fn scans_the_whole_scene_for_duplicate_identities() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let predecessor = repo.rev_parse_single("HEAD~1")?.detach();
        let mut duplicate = repo.find_commit(repo.head_id()?)?.decode()?.into_owned()?;
        duplicate
            .extra_headers
            .push((HEADER.into(), ChangeId::from(predecessor).to_string().into()));
        let duplicate = repo.write_object(&duplicate)?.detach();

        let prefix = ChangeId::from(predecessor).to_reverse_hex_with_len(7).to_string();
        assert_eq!(
            resolve_prefix(&repo, &prefix, [predecessor])?,
            Some(predecessor),
            "a unique default-view change ID resolves"
        );
        assert!(
            format!(
                "{:#}",
                resolve_prefix(&repo, &prefix, [predecessor, duplicate])
                    .expect_err("two commits sharing a change ID are ambiguous")
            )
            .contains("ambiguous"),
            "ambiguity is reported explicitly"
        );

        let mut successor = repo.find_commit(duplicate)?.decode()?.into_owned()?;
        successor.extra_headers.clear();
        inherit(&repo, &mut successor, duplicate)?;
        assert_eq!(
            effective(duplicate, successor.extra_headers().find_all(HEADER)),
            ChangeId::from(predecessor),
            "later rewrites inherit the stored identity from their predecessor"
        );

        let scan =
            scan(&repo, &[predecessor, duplicate], &AtomicBool::new(false))?.expect("the scan was not cancelled");
        assert_eq!(
            scan.duplicates,
            HashSet::from([predecessor, duplicate]),
            "both off-screen and visible instances are marked"
        );
        assert_eq!(
            scan.overrides.get(&duplicate),
            Some(&ChangeId::from(predecessor)),
            "the header overrides the duplicate commit's trivial identity"
        );
        Ok(())
    }
}
