use anyhow::Context;
use gix::{ObjectId, objs::Kind};

pub fn function(mut repo: gix::Repository, spec: Option<String>, mut out: impl std::io::Write) -> anyhow::Result<()> {
    let spec = spec.unwrap_or("HEAD".into());

    repo.verify_references()
        .context("Reference storage verification failed")?;

    repo.object_cache_size_if_unset(4 * 1024 * 1024);
    // We expect to be finding a bunch of non-existent objects here - never refresh the ODB
    repo.objects.refresh_never();

    let id = repo
        .rev_parse_single(spec.as_str())
        .context("Only single revisions are supported")?;
    let commits: gix::revision::Walk<'_> = id
        .object()?
        .peel_to_kind(gix::object::Kind::Commit)
        .context("Need committish as starting point")?
        .id()
        .ancestors()
        .all()?;

    let on_missing = |oid: &ObjectId, kind: Kind| {
        writeln!(out, "{oid}: {kind}").expect("failed to write output");
    };

    let mut check = gix_fsck::Connectivity::new(&repo.objects, on_missing);
    // Walk all commits, checking each one for connectivity
    for commit in commits {
        let commit = commit?;
        check.check_commit(&commit.id)?;
        // Note that we leave parent-iteration to the commits iterator, as it will
        // correctly handle shallow repositories which are expected to have the commits
        // along the shallow boundary missing.
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_corrupt_reference_storage_before_walking_objects() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = gix::ThreadSafeRepository::init(
            temp.path(),
            gix::create::Kind::Bare,
            gix::create::Options {
                reference_storage: gix::create::ReferenceStorage::Reftable,
                ..Default::default()
            },
        )?
        .to_thread_local();
        let reftable = repo.git_dir().join("reftable");
        let table_name = std::fs::read_to_string(reftable.join("tables.list"))?;
        std::fs::write(reftable.join(table_name.trim()), b"corrupt")?;

        let err = super::function(repo, None, Vec::new()).expect_err("corrupt reference storage must fail fsck");
        assert!(
            format!("{err:#}").contains("Reference storage verification failed"),
            "fsck should retain reference verification context: {err:#}"
        );
        Ok(())
    }
}
