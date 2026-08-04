use anyhow::bail;

use crate::OutputFormat;

pub fn merge_base(
    mut repo: gix::Repository,
    first: String,
    others: Vec<String>,
    mut out: impl std::io::Write,
    format: OutputFormat,
) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        bail!("Only 'human' format is currently supported");
    }
    repo.object_cache_size_if_unset(50 * 1024 * 1024);
    let first_id = commit_id(&repo, first.as_str())?;
    let other_ids: Vec<_> = others
        .iter()
        .map(|other| commit_id(&repo, other.as_str()))
        .collect::<Result<_, _>>()?;

    let cache = repo.commit_graph_if_enabled()?;
    let mut graph = repo.revision_graph(cache.as_ref());
    let bases = repo.merge_bases_many_with_graph(first_id, &other_ids, &mut graph)?;
    if bases.is_empty() {
        bail!("No base found for {first} and {others}", others = others.join(", "))
    }
    for id in bases {
        writeln!(&mut out, "{id}")?;
    }
    Ok(())
}

fn commit_id(repo: &gix::Repository, revspec: &str) -> anyhow::Result<gix::ObjectId> {
    Ok(repo.rev_parse_single(revspec)?.object()?.peel_to_commit()?.id)
}
