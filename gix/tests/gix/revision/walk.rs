use gix::revision::walk::{Platform, SharedCommitGraph, Sorting};
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

fn assert_graph_use(walk: Platform<'_, '_>, expected: bool) -> gix::Result<()> {
    let commits = walk.all()?.collect::<gix::Result<Vec<_>>>()?;
    assert!(!commits.is_empty(), "the cache check traverses existing commits");
    assert!(
        commits.iter().all(|commit| commit.generation.is_some() == expected),
        "generation numbers reveal whether the optional graph was used"
    );
    Ok(())
}

#[test]
fn hidden_walk_cache_matches_stock_across_tips_and_bases() -> TestResult {
    for use_graph in [false, true] {
        let repo = crate::util::named_subrepo_opts(
            FIXTURE,
            ".",
            crate::util::restricted().config_overrides([format!("core.commitGraph={use_graph}")]),
        )?;
        let tip = repo.head_id()?;
        let mut cache = repo.rev_walk_cache();
        for (tip, hidden) in [
            ("tip", "main"),
            ("side", "tip"),
            ("merge", "side"),
            ("main", "side"),
            ("tip", "root"),
            ("tip", "main"),
        ] {
            let tip = repo.rev_parse_single(tip)?;
            let hidden = repo.rev_parse_single(hidden)?;
            for first_parent in [false, true] {
                let walk = || {
                    let walk = repo.rev_walk([tip]).with_hidden([hidden]);
                    if first_parent { walk.first_parent_only() } else { walk }
                };
                assert_eq!(
                    detached(walk().with_cache(&mut cache).all()?)?,
                    detached(walk().all()?)?,
                    "tip {tip}, hidden {hidden}, commit-graph {use_graph}, first parent {first_parent}"
                );
            }
        }
        let base = repo.rev_parse_single("main")?;
        let with_override = || repo.rev_walk([tip]).with_hidden([base]).use_commit_graph(false);
        assert_eq!(
            detached(with_override().with_cache(&mut cache).all()?)?,
            detached(with_override().all()?)?,
            "explicit graph policy applies even with a populated cache"
        );
    }
    Ok(())
}

#[test]
fn hidden_walk_cache_respects_commit_graph_overrides() -> TestResult {
    let repo = crate::util::named_subrepo_opts(
        FIXTURE,
        ".",
        crate::util::restricted().config_overrides(["core.commitGraph=false"]),
    )?;
    let partial_repo = crate::util::named_subrepo_opts(FIXTURE, "partial-graph", crate::util::restricted())?;
    let tip_commit_id = repo.head_id()?;
    let main_commit_id = repo.rev_parse_single("main")?;
    let root_commit_id = repo.rev_parse_single("root")?;
    let mut cache = repo.rev_walk_cache();
    for (graph_policy, supplied_repo, expected_generations) in [
        (None, None, (false, false)),
        (Some(true), None, (true, true)),
        (Some(true), Some(&partial_repo), (false, true)),
        (Some(true), Some(&repo), (true, true)),
        (Some(false), Some(&repo), (false, false)),
        (Some(true), None, (true, true)),
        (None, Some(&repo), (false, false)),
    ] {
        let walk = || -> gix::Result<_> {
            Ok(repo
                .rev_walk([tip_commit_id])
                .with_hidden([root_commit_id])
                .use_commit_graph(graph_policy)
                .with_commit_graph(supplied_repo.map(gix::Repository::commit_graph).transpose()?))
        };
        let actual = detached(walk()?.with_cache(&mut cache).all()?)?;
        assert_eq!(
            actual,
            detached(walk()?.all()?)?,
            "warming with another graph does not change results"
        );
        let has_generation = |commit_id: gix::Id<'_>| {
            actual
                .iter()
                .find(|info| info.id == commit_id)
                .expect("the walk yields both tip and main")
                .generation
                .is_some()
        };
        assert_eq!(
            (has_generation(tip_commit_id), has_generation(main_commit_id)),
            expected_generations,
            "the walk only uses generations from the graph selected for it"
        );
    }
    Ok(())
}

#[test]
fn shared_commit_graph_keeps_the_snapshot_alive_for_a_walk() -> TestResult {
    let repo = crate::named_repo(FIXTURE)?;
    let tip_commit_id = repo.head_id()?;
    let graph = SharedCommitGraph::new(repo.commit_graph()?);
    let pos = graph.lookup(tip_commit_id).expect("the fixture graph contains the tip");
    let shared = graph.clone();
    assert!(
        std::ptr::eq(graph.id_at(pos), shared.id_at(pos)),
        "cloning the handle shares the mapped commit data"
    );
    drop(graph);

    let tip = gix::traverse::commit::Simple::new([tip_commit_id], &repo.objects)
        .with_commit_graph(Some(shared))
        .next()
        .transpose()?
        .expect("the walk yields the tip");
    assert_eq!(tip.id, tip_commit_id, "the shared snapshot yields the original tip");
    assert!(
        tip.generation.is_some(),
        "the walk still reads the graph after its original handle drops"
    );
    Ok(())
}

#[test]
fn hidden_walk_cache_is_bound_to_its_creating_handle() -> TestResult {
    let repo = crate::named_repo(FIXTURE)?;
    let other_repo = repo.clone();
    let tip_commit = repo.head_commit()?;
    let root_commit_id = repo.rev_parse_single("root")?;
    let walk = || tip_commit.ancestors().with_hidden([root_commit_id]);
    let other_walk = || other_repo.rev_walk([tip_commit.id()]).with_hidden([root_commit_id]);
    let mut cache = repo.rev_walk_cache();

    assert!(
        other_walk().with_cache(&mut cache).all().is_err(),
        "even an unused cache rejects a cloned repository handle"
    );
    let expected = detached(walk().all()?)?;
    assert_eq!(
        detached(walk().with_cache(&mut cache).all()?)?,
        expected,
        "Commit::ancestors can use a cache created on its repository handle"
    );
    assert!(
        other_walk().with_cache(&mut cache).selected(|_| true).is_err(),
        "warming the cache does not permit switching handles"
    );
    assert_eq!(
        detached(walk().with_cache(&mut cache).all()?)?,
        expected,
        "rejecting another handle does not affect reuse on the creating handle"
    );
    Ok(())
}

#[test]
fn hidden_walk_cache_preserves_date_cutoff_with_preloaded_ancestry() -> TestResult {
    let repo = crate::util::named_subrepo_opts(FIXTURE, "skewed", crate::util::restricted())?;
    let base = repo.rev_parse_single("base")?;
    let root = repo.rev_parse_single("root")?;
    let tip = repo.head_id()?;
    let mut cache = repo.rev_walk_cache();
    let preload = repo
        .rev_walk([base])
        .with_hidden([root])
        .with_cache(&mut cache)
        .all()?
        .collect::<gix::Result<Vec<_>>>()?;
    assert_eq!(preload.len(), 7, "preload the whole hidden chain");
    drop(preload);

    let walk = || repo.rev_walk([tip]).with_hidden([base]);
    let stock = detached(walk().all()?)?;
    assert!(
        stock.iter().any(|info| info.id == root.detach()),
        "the stock date cutoff leaves the skewed root visible"
    );
    assert_eq!(
        detached(walk().with_cache(&mut cache).all()?)?,
        stock,
        "the cutoff only marks metadata loaded during the current paint"
    );
    Ok(())
}

#[test]
fn hidden_walk_cache_preserves_selection_and_can_be_cleared() -> TestResult {
    let repo = crate::named_repo(FIXTURE)?;
    let tip_commit_id = repo.head_id()?;
    let root_commit_id = repo.rev_parse_single("root")?;
    let walk = || repo.rev_walk([tip_commit_id]).with_hidden([root_commit_id]);
    let mut cache = repo.rev_walk_cache();
    detached(walk().with_cache(&mut cache).all()?)?;

    for rejected_commit_id in [repo.rev_parse_single("main")?, repo.rev_parse_single("side")?] {
        let mut uncached_calls = Vec::new();
        let uncached = walk().with_cache(&mut cache).with_cache(None).selected(|commit_id| {
            uncached_calls.push(commit_id.to_owned());
            commit_id != rejected_commit_id.as_ref()
        })?;
        let mut cached_calls = Vec::new();
        let cached = walk().with_cache(Some(&mut cache)).selected(|commit_id| {
            cached_calls.push(commit_id.to_owned());
            commit_id != rejected_commit_id.as_ref()
        })?;
        let (cached, uncached) = (detached(cached)?, detached(uncached)?);
        assert_eq!(
            cached, uncached,
            "a warm cache preserves the selected walk's ordered results"
        );
        assert_eq!(
            cached_calls, uncached_calls,
            "metadata hits do not skip or reorder predicate calls"
        );
        assert!(
            cached_calls.contains(&rejected_commit_id.detach()),
            "the predicate must see the rejected commit"
        );
        assert!(
            cached.iter().all(|info| info.id != rejected_commit_id),
            "the predicate prunes the rejected commit"
        );
    }
    Ok(())
}

fn detached(walk: gix::revision::Walk<'_>) -> gix::Result<Vec<gix::traverse::commit::Info>> {
    walk.map(|info| info.map(gix::revision::walk::Info::detach)).collect()
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
        let tip = repo.head_id()?;
        let hidden = repo.rev_parse_single("main")?;
        let walk = || repo.rev_walk([tip]).sorting(sorting).with_hidden([hidden]);
        let mut actual = ids(walk().with_commit_graph(Some(repo.commit_graph()?)).all()?)?;
        let mut expected = baseline(&full, "shallow-hidden.baseline")?;
        actual.sort();
        expected.sort();
        assert_eq!(
            actual, expected,
            "hidden painting stops at shallow main, preserving root reached through side"
        );
        let mut cache = repo.rev_walk_cache();
        assert_eq!(
            detached(walk().with_cache(&mut cache).all()?)?,
            detached(walk().all()?)?,
            "the cache path respects shallow boundaries and preserves order"
        );
    }
    Ok(())
}
