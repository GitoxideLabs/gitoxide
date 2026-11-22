use gix::{
    Result,
    error::{ResultExt, message},
    fsck::Connectivity,
};

pub fn function(mut repo: gix::Repository, spec: Option<String>, mut out: impl std::io::Write) -> Result<()> {
    let spec = spec.unwrap_or("HEAD".into());

    repo.object_cache_size_if_unset(4 * 1024 * 1024);
    // We expect to be finding a bunch of non-existent objects here - never refresh the ODB
    repo.objects.refresh_never();

    let id = repo
        .rev_parse_single(spec.as_str())
        .or_raise(|| message("Only single revisions are supported"))?;
    let commits: gix::revision::Walk<'_> = id
        .object()?
        .peel_to_kind(gix::object::Kind::Commit)
        .or_raise(|| message("Need committish as starting point"))?
        .id()
        .ancestors()
        .all()?;

    let mut check = Connectivity::new(&repo.objects, |oid, kind| {
        writeln!(out, "{oid}: {kind}").expect("failed to write output");
    });
    // Leave ancestry to the revision walk so shallow boundaries are respected.
    for commit in commits {
        check.check_commit(&commit?.id)?;
    }
    Ok(())
}
