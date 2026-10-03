pub(crate) mod function {
    use crate::OutputFormat;
    use crate::repository::HexId;
    use gix::odb::store::RefreshMode;
    use gix::revision::plumbing::Spec;
    use gix::{
        Result,
        error::{ResultExt, bail, message},
        prelude::ObjectIdExt,
        revision::walk::Sorting,
    };
    use std::{borrow::Cow, ffi::OsString};

    pub fn list(
        mut repo: gix::Repository,
        spec: OsString,
        mut out: impl std::io::Write,
        long_hashes: bool,
        format: OutputFormat,
    ) -> Result<()> {
        if format != OutputFormat::Human {
            bail!(gix::error::unsupported("Only human output is currently supported"));
        }
        let graph = repo
            .commit_graph_if_enabled()
            .or_raise(|| message("a commitgraph is required, but none was found"))?;
        repo.object_cache_size_if_unset(4 * 1024 * 1024);
        repo.objects.refresh = RefreshMode::Never;

        let spec = gix::path::os_str_into_bstr(&spec)?;
        let spec = repo.rev_parse(spec)?.detach();
        let commits = match spec {
            Spec::Include(id) => connected_commit_id(&repo, id)?
                .ancestors()
                .sorting(Sorting::ByCommitTime(Default::default()))
                .all()?,
            Spec::Range { from, to } => connected_commit_id(&repo, to)?
                .ancestors()
                .sorting(Sorting::ByCommitTime(Default::default()))
                .with_hidden(Some(connected_commit_id(&repo, from)?))
                .all()?,
            Spec::Exclude(_) | Spec::Merge { .. } | Spec::IncludeOnlyParents(_) | Spec::ExcludeParents(_) => {
                bail!("The spec isn't currently supported: {spec:?}".unsupported())
            }
        };
        for commit in commits {
            let commit = commit?;
            writeln!(
                out,
                "{} {} {} {}",
                HexId::new(commit.id(), long_hashes),
                commit.commit_time.expect("traversal with date"),
                commit.parent_ids.len(),
                graph
                    .as_ref()
                    .map_or(Cow::Borrowed(""), |graph| graph.commit_by_id(commit.id).map_or_else(
                        || Cow::Borrowed("<NOT IN GRAPH-CACHE>"),
                        |c| Cow::Owned(format!(
                            "{} {}",
                            HexId::new(c.root_tree_id().to_owned().attach(&repo), long_hashes),
                            c.generation()
                        ))
                    ))
            )
            .or_error()?;
        }
        Ok(())
    }

    fn connected_commit_id(repo: &gix::Repository, id: gix::ObjectId) -> Result<gix::Id<'_>> {
        Ok(id
            .attach(repo)
            .object()?
            .peel_to_kind(gix::object::Kind::Commit)
            .or_raise(|| message("Need committish as starting point"))?
            .id())
    }
}
