use std::{
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
    fn scans_the_whole_scene_for_duplicate_identities() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let predecessor = repo.rev_parse_single("HEAD~1")?.detach();
        let mut duplicate = repo.find_commit(repo.head_id()?)?.decode()?.into_owned()?;
        duplicate
            .extra_headers
            .push((HEADER.into(), ChangeId::from(predecessor).to_string().into()));
        let duplicate = repo.write_object(&duplicate)?.detach();

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
