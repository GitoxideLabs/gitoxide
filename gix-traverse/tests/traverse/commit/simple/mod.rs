use crate::hex_to_id;
use crate::util::{commit_graph, git_graph, git_graph_with_time, named_fixture, parse_commit_names};
use gix_hash::ObjectId;
use gix_traverse::commit::{simple::Sorting, Parents, Simple};

mod adjusted_dates;
mod different_date;
mod different_date_intermixed;
mod hide;
mod same_date;

/// Run a simple traversal and collect the resulting commit IDs.
fn traverse(
    tips: impl IntoIterator<Item = ObjectId>,
    odb: &gix_odb::Handle,
    sorting: Sorting,
    parents: Parents,
    hidden: impl IntoIterator<Item = ObjectId>,
) -> crate::Result<Vec<ObjectId>> {
    let graph = commit_graph(odb.store_ref());
    Simple::new(tips, odb)
        .sorting(sorting)?
        .parents(parents)
        .hide(hidden)?
        .commit_graph(graph)
        .map(|res| res.map(|info| info.id))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Run a traversal with both commit-graph enabled and disabled to ensure consistency.
fn traverse_both(
    tips: impl IntoIterator<Item = ObjectId> + Clone,
    odb: &gix_odb::Handle,
    sorting: Sorting,
    parents: Parents,
    hidden: impl IntoIterator<Item = ObjectId> + Clone,
) -> crate::Result<Vec<ObjectId>> {
    // Without commit graph
    let without_graph: Vec<_> = Simple::new(tips.clone(), odb)
        .sorting(sorting)?
        .parents(parents)
        .hide(hidden.clone())?
        .commit_graph(None)
        .map(|res| res.map(|info| info.id))
        .collect::<Result<Vec<_>, _>>()?;

    // With commit graph
    let graph = commit_graph(odb.store_ref());
    let with_graph: Vec<_> = Simple::new(tips, odb)
        .sorting(sorting)?
        .parents(parents)
        .hide(hidden)?
        .commit_graph(graph)
        .map(|res| res.map(|info| info.id))
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(
        without_graph, with_graph,
        "results must be consistent with and without commit-graph"
    );
    Ok(with_graph)
}

fn all_sortings() -> impl Iterator<Item = Sorting> {
    use gix_traverse::commit::simple::CommitTimeOrder;
    [
        Sorting::BreadthFirst,
        Sorting::ByCommitTime(CommitTimeOrder::NewestFirst),
        Sorting::ByCommitTime(CommitTimeOrder::OldestFirst),
    ]
    .into_iter()
}
