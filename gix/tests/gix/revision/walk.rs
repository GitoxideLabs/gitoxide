use gix::revision::walk::{Platform, Sorting};
use gix_testtools::TestResult;

const FIXTURE: &str = "make_revision_walk.sh";

fn ids(walk: gix::revision::Walk<'_>) -> gix::Result<Vec<String>> {
    walk.map(|commit| commit.map(|commit| commit.id.to_string())).collect()
}

fn baseline(repo: &gix::Repository, name: &str) -> std::io::Result<Vec<String>> {
    Ok(
        std::fs::read_to_string(repo.workdir().expect("the fixture is non-bare").join(name))?
            .lines()
            .map(str::to_owned)
            .collect(),
    )
}

fn named_ids(repo: &gix::Repository, names: &[&str]) -> gix::Result<Vec<String>> {
    names
        .iter()
        .map(|name| repo.rev_parse_single(*name).map(|commit_id| commit_id.to_string()))
        .collect()
}

fn assert_graph_use(walk: Platform<'_>, expected: bool) -> gix::Result<()> {
    let commits = walk.all()?.collect::<gix::Result<Vec<_>>>()?;
    assert!(!commits.is_empty(), "the cache check traverses existing commits");
    assert!(
        commits.iter().all(|commit| commit.generation.is_some() == expected),
        "generation numbers reveal whether the optional graph was used"
    );
    Ok(())
}

#[test]
fn topological_sorting_matches_git() -> TestResult {
    let repo = crate::named_repo(FIXTURE)?;
    let tip_commit_id = repo.head_id()?;
    let main_commit_id = repo.rev_parse_single("main")?;
    let side_commit_id = repo.rev_parse_single("side")?;
    let root_commit_id = repo.rev_parse_single("root")?;
    for (sorting, order) in [(Sorting::TopoOrder, "topo-order"), (Sorting::DateOrder, "date-order")] {
        for (first_parent, parents) in [(false, "all"), (true, "first")] {
            for use_graph in [false, true] {
                for (tips, hidden, name) in [
                    (vec![tip_commit_id], vec![], "all"),
                    (vec![tip_commit_id], vec![main_commit_id], "hidden"),
                    (
                        vec![main_commit_id, side_commit_id, main_commit_id],
                        vec![root_commit_id],
                        "multiple",
                    ),
                ] {
                    let walk = repo
                        .rev_walk(tips)
                        .sorting(sorting)
                        .with_hidden(hidden)
                        .use_commit_graph(use_graph);
                    let walk = if first_parent { walk.first_parent_only() } else { walk };
                    assert_eq!(
                        ids(walk.all()?)?,
                        baseline(&repo, &format!("{order}-{parents}-{name}.baseline"))?,
                        "{order}, {parents}, {name}, commit-graph={use_graph} agrees with Git"
                    );
                }
            }
        }
    }

    Ok(())
}

#[test]
fn topological_info_is_attached_and_respects_first_parent() -> TestResult {
    let repo = crate::named_repo(FIXTURE)?;
    for sorting in [Sorting::TopoOrder, Sorting::DateOrder] {
        for use_graph in [false, true] {
            for first_parent in [false, true] {
                let walk = repo
                    .rev_walk([repo.head_id()?])
                    .sorting(sorting)
                    .use_commit_graph(use_graph);
                let walk = if first_parent { walk.first_parent_only() } else { walk };
                for info in walk.all()? {
                    let info = info?;
                    let commit = info.object()?;
                    assert_eq!(info.id(), commit.id(), "attached IDs can resolve their commit");
                    assert_eq!(info.commit_time(), commit.time()?.seconds, "commit times match objects");
                    let expected: Vec<_> = commit
                        .parent_ids()
                        .take(if first_parent { 1 } else { usize::MAX })
                        .collect();
                    assert_eq!(
                        info.parent_ids().collect::<Vec<_>>(),
                        expected,
                        "returned parents follow the configured traversal policy"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn predicates_and_boundaries_prune_paths() -> TestResult {
    let repo = crate::named_repo(FIXTURE)?;
    let tip_commit_id = repo.head_id()?;
    let main_commit_id = repo.rev_parse_single("main")?;
    let expected = named_ids(&repo, &["tip", "merge", "side", "side-base", "root"])?;
    for sorting in [Sorting::TopoOrder, Sorting::DateOrder] {
        for use_graph in [false, true] {
            let mut visited = std::collections::HashSet::new();
            let walk = repo
                .rev_walk([tip_commit_id])
                .sorting(sorting)
                .use_commit_graph(use_graph)
                .selected(|commit_id| {
                    assert!(
                        visited.insert(commit_id.to_owned()),
                        "the predicate runs once per commit"
                    );
                    commit_id != main_commit_id.as_ref()
                })?;
            assert_eq!(
                ids(walk)?,
                expected,
                "pruning main still reaches the shared root along the side branch"
            );
            assert_eq!(
                ids(repo
                    .rev_walk([tip_commit_id])
                    .with_boundary([main_commit_id])
                    .sorting(sorting)
                    .use_commit_graph(use_graph)
                    .all()?)?,
                expected,
                "boundaries use the same path pruning when their time cutoff is overridden"
            );
        }
    }
    Ok(())
}

#[test]
fn empty_rejected_and_missing_tips() -> TestResult {
    let repo = crate::named_repo(FIXTURE)?;
    for sorting in [Sorting::TopoOrder, Sorting::DateOrder] {
        assert!(
            ids(repo.rev_walk([] as [gix::ObjectId; 0]).sorting(sorting).all()?)?.is_empty(),
            "no starting points produce no commits"
        );
        assert!(
            ids(repo.rev_walk([repo.head_id()?]).sorting(sorting).selected(|_| false)?)?.is_empty(),
            "rejecting a starting point also prunes its ancestors"
        );
        assert!(
            repo.rev_walk([gix::ObjectId::null(repo.object_hash())])
                .sorting(sorting)
                .all()
                .and_then(ids)
                .is_err(),
            "an accepted missing commit reports an error"
        );
    }
    Ok(())
}

#[test]
fn graph_controls_are_shared_with_simple_walks() -> TestResult {
    let mut repo = crate::named_repo(FIXTURE)?;
    for configured in [false, true] {
        repo.config_snapshot_mut()
            .set_raw_value("core.commitGraph", configured.to_string())?;
        for sorting in [Sorting::BreadthFirst, Sorting::TopoOrder, Sorting::DateOrder] {
            for override_graph in [None, Some(false), Some(true)] {
                assert_graph_use(
                    repo.rev_walk([repo.head_id()?])
                        .sorting(sorting)
                        .use_commit_graph(override_graph),
                    override_graph.unwrap_or(configured),
                )?;
                assert_graph_use(
                    repo.rev_walk([repo.head_id()?])
                        .sorting(sorting)
                        .with_commit_graph(Some(repo.commit_graph()?))
                        .use_commit_graph(override_graph),
                    override_graph.unwrap_or(configured),
                )?;
            }
        }
    }
    let mut repo = gix::open_opts(repo.git_dir(), crate::util::restricted().strict_config(true))?;
    repo.config_snapshot_mut()
        .set_raw_value("core.commitGraph", "not-a-boolean")?;
    for sorting in [Sorting::BreadthFirst, Sorting::TopoOrder, Sorting::DateOrder] {
        assert!(
            repo.rev_walk([repo.head_id()?]).sorting(sorting).all().is_err(),
            "strict configuration errors are reported when there is no explicit override"
        );
        for use_graph in [false, true] {
            assert_graph_use(
                repo.rev_walk([repo.head_id()?])
                    .sorting(sorting)
                    .use_commit_graph(use_graph),
                use_graph,
            )?;
        }
    }
    Ok(())
}

#[test]
fn shallow_commits_are_roots_even_with_hidden_history_and_a_stale_graph() -> TestResult {
    let full = crate::named_repo(FIXTURE)?;
    for sorting in [Sorting::BreadthFirst, Sorting::TopoOrder, Sorting::DateOrder] {
        let repo = crate::util::named_subrepo_opts(FIXTURE, "shallow", crate::util::restricted())?;
        let infos = repo
            .rev_walk([repo.head_id()?])
            .sorting(sorting)
            .with_commit_graph(Some(full.commit_graph()?))
            .all()?
            .collect::<gix::Result<Vec<_>>>()?;
        assert_eq!(infos.len(), 1, "the depth-one walk does not read its missing parent");
        assert!(
            infos[0].parent_ids.is_empty(),
            "shallow commits expose no traversal parents"
        );
        assert!(
            infos[0].generation.is_none(),
            "a supplied graph cannot bypass shallow boundaries"
        );
        assert_eq!(
            infos[0].object()?.parent_ids().count(),
            1,
            "attached object access preserves the stored commit's parents"
        );

        let repo = crate::util::named_subrepo_opts(FIXTURE, "shallow-with-objects", crate::util::restricted())?;
        let mut actual = ids(repo
            .rev_walk([repo.head_id()?])
            .sorting(sorting)
            .with_hidden([repo.rev_parse_single("main")?])
            .with_commit_graph(Some(repo.commit_graph()?))
            .all()?)?;
        let mut expected = baseline(&full, "shallow-hidden.baseline")?;
        actual.sort();
        expected.sort();
        assert_eq!(
            actual, expected,
            "hidden painting stops at shallow main, preserving root reached through side"
        );
    }
    Ok(())
}
