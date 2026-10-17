use gix_testtools::TestResult;

#[test]
fn missing_objects_info_does_not_prevent_merge_base() -> TestResult {
    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let info_dir = repo.objects.store_ref().path().join("info");
    std::fs::create_dir_all(&info_dir)?;
    assert!(
        repo.commit_graph_if_enabled()?.is_none(),
        "an empty objects/info directory has no optional commit-graph"
    );
    std::fs::remove_dir(&info_dir)?;
    assert!(
        repo.commit_graph_if_enabled()?.is_none(),
        "an absent objects/info directory also has no optional commit-graph"
    );

    let head_commit_id = repo.head_id()?;
    assert_eq!(
        repo.merge_base(head_commit_id, head_commit_id)?,
        Some(head_commit_id),
        "a commit is its own merge-base without a commit-graph"
    );
    let parent_commit_id = repo.rev_parse_single("HEAD^")?;
    assert_eq!(
        repo.merge_base(head_commit_id, parent_commit_id)?,
        Some(parent_commit_id),
        "merge-base can traverse history without a commit-graph"
    );
    Ok(())
}

#[test]
fn merge_base_variants_find_common_ancestors() -> TestResult {
    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let head_commit_id = repo.head_id()?;
    let parent_commit_id = repo.rev_parse_single("HEAD^")?;
    let mut graph = repo.revision_graph(None);
    assert_eq!(
        repo.merge_base_with_graph(head_commit_id, parent_commit_id, &mut graph)?,
        Some(parent_commit_id),
        "a reusable graph finds the same ancestor as an ordinary pairwise traversal"
    );
    for bases in [
        repo.merge_bases_many(head_commit_id, &[parent_commit_id.into()])?,
        repo.merge_bases_many_with_graph(head_commit_id, &[parent_commit_id.into()], &mut graph)?,
    ] {
        assert_eq!(
            bases,
            [parent_commit_id],
            "many-base queries still return a list of bases"
        );
    }
    for (commit_ids, expected, description) in [
        (
            vec![head_commit_id, parent_commit_id, head_commit_id],
            Some(parent_commit_id),
            "related octopus inputs share their ancestor",
        ),
        (
            vec![head_commit_id],
            Some(head_commit_id),
            "a singleton octopus input is its own base",
        ),
        (Vec::new(), None, "empty octopus input has no base"),
    ] {
        assert_eq!(
            repo.merge_base_octopus(commit_ids.iter().copied())?,
            expected,
            "{description}"
        );
        assert_eq!(
            repo.merge_base_octopus_with_graph(commit_ids, &mut graph)?,
            expected,
            "{description}, including when reusing a graph"
        );
    }
    Ok(())
}

#[test]
fn unrelated_histories_have_no_merge_base() -> TestResult {
    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let head = repo.head_commit()?;
    let head_commit_id = head.id();
    let mut unrelated_commit = head.decode()?.to_owned()?;
    unrelated_commit.parents.clear();
    unrelated_commit.message = "unrelated history\n".into();
    let unrelated_commit_id = repo.write_object(unrelated_commit)?;
    let mut graph = repo.revision_graph(None);

    for base in [
        repo.merge_base(head_commit_id, unrelated_commit_id)?,
        repo.merge_base_with_graph(head_commit_id, unrelated_commit_id, &mut graph)?,
        repo.merge_base_octopus([head_commit_id, unrelated_commit_id])?,
        repo.merge_base_octopus_with_graph([head_commit_id, unrelated_commit_id], &mut graph)?,
    ] {
        assert_eq!(base, None, "unrelated histories are a successful query without a base");
    }
    for bases in [
        repo.merge_bases_many(head_commit_id, &[unrelated_commit_id.into()])?,
        repo.merge_bases_many_with_graph(head_commit_id, &[unrelated_commit_id.into()], &mut graph)?,
    ] {
        assert!(
            bases.is_empty(),
            "many-base queries succeed with an empty list for unrelated histories"
        );
    }
    Ok(())
}

#[test]
fn is_ancestor_follows_parents_like_git() -> TestResult {
    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let head = repo.head_commit()?;
    let head_commit_id = head.id();
    let parent_commit_id = repo.rev_parse_single("HEAD^")?;
    let mut unrelated_commit = head.decode()?.to_owned()?;
    unrelated_commit.parents.clear();
    unrelated_commit.message = "unrelated history\n".into();
    let unrelated_commit_id = repo.write_object(unrelated_commit)?;
    let mut graph = repo.revision_graph(None);

    for (ancestor_commit_id, descendant_commit_id, expected, description) in [
        (
            parent_commit_id,
            head_commit_id,
            true,
            "a parent is an ancestor of its child",
        ),
        (
            head_commit_id,
            parent_commit_id,
            false,
            "a child isn't an ancestor of its parent",
        ),
        (
            head_commit_id,
            head_commit_id,
            true,
            "a commit is its own ancestor, as in Git",
        ),
        (
            unrelated_commit_id,
            head_commit_id,
            false,
            "commits of unrelated histories aren't ancestors of each other",
        ),
        (
            head_commit_id,
            unrelated_commit_id,
            false,
            "commits of unrelated histories aren't ancestors of each other, in either direction",
        ),
    ] {
        assert_eq!(
            repo.is_ancestor(ancestor_commit_id, descendant_commit_id)?,
            expected,
            "{description}"
        );
        assert_eq!(
            repo.is_ancestor_with_graph(ancestor_commit_id, descendant_commit_id, &mut graph)?,
            expected,
            "{description}, including when reusing a graph"
        );
    }
    Ok(())
}

#[test]
fn ahead_behind_counts_match_git() -> TestResult {
    use gix::revision::plumbing::AheadBehind;

    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let head = repo.head_commit()?;
    let head_commit_id = head.id();
    let parent_commit_id = repo.rev_parse_single("HEAD^")?;
    let mut commit = head.decode()?.to_owned()?;
    commit.message = "divergent history\n".into();
    let divergent_commit_id = repo.write_object(&commit)?;
    commit.parents.clear();
    commit.message = "unrelated history\n".into();
    let unrelated_commit_id = repo.write_object(&commit)?;

    for use_commitgraph in [false, true] {
        if use_commitgraph {
            gix_testtools::git(repo.git_dir(), "commit-graph write --reachable")?;
        }
        let cache = repo.commit_graph_if_enabled()?;
        assert_eq!(
            cache.is_some(),
            use_commitgraph,
            "the optional commit-graph is exercised"
        );
        let mut graph = repo.revision_graph(cache.as_ref());
        for (tip_commit_id, base_commit_id) in [
            (head_commit_id, parent_commit_id),
            (parent_commit_id, head_commit_id),
            (head_commit_id, head_commit_id),
            (head_commit_id, divergent_commit_id),
            (head_commit_id, unrelated_commit_id),
            (unrelated_commit_id, head_commit_id),
        ] {
            let counts = gix_testtools::git(
                repo.git_dir(),
                &format!("rev-list --left-right --count {tip_commit_id}...{base_commit_id}"),
            )?;
            let mut counts = counts.split_whitespace();
            let expected = AheadBehind {
                ahead: counts.next().expect("Git returns the left count").parse()?,
                behind: counts.next().expect("Git returns the right count").parse()?,
            };
            assert_eq!(
                repo.ahead_behind(tip_commit_id, base_commit_id)?,
                expected,
                "counts match Git, commit-graph: {use_commitgraph}"
            );
            assert_eq!(
                repo.ahead_behind_with_graph(tip_commit_id, base_commit_id, &mut graph)?,
                expected,
                "counts match Git with a reused graph, commit-graph: {use_commitgraph}"
            );
        }
    }
    Ok(())
}

#[test]
fn shallow_boundaries_stop_ancestry_and_counting_like_git() -> TestResult {
    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    gix_testtools::git(
        repo.workdir().expect("the fixture has a worktree"),
        "commit --allow-empty -m third",
    )?;
    let tip_commit_id = repo.head_id()?;
    let boundary_commit_id = repo.rev_parse_single("HEAD^")?;
    let root_commit_id = repo.rev_parse_single("HEAD~2")?;
    gix_testtools::git(repo.git_dir(), "commit-graph write --reachable")?;
    let cache = repo.commit_graph()?;
    let mut graph = repo.revision_graph(Some(&cache));
    // Populate the reusable graph before recording a boundary, while retaining all older objects.
    repo.ahead_behind_with_graph(tip_commit_id, root_commit_id, &mut graph)?;
    std::fs::write(repo.shallow_file()?, format!("{boundary_commit_id}\n"))?;
    assert_eq!(
        repo.rev_walk([tip_commit_id]).all()?.count(),
        2,
        "rev-walk stops at the boundary"
    );
    assert_eq!(
        repo.ahead_behind_with_graph(tip_commit_id, root_commit_id, &mut graph)?,
        gix::revision::plumbing::AheadBehind { ahead: 2, behind: 1 },
        "counting independently applies the boundary to an already populated graph"
    );
    assert_history_queries_match_git(&repo, tip_commit_id, root_commit_id, &mut graph)?;
    assert_history_queries_match_git(&repo, tip_commit_id, boundary_commit_id, &mut graph)?;
    let mut uncached_graph = repo.revision_graph(None);
    assert_history_queries_match_git(&repo, tip_commit_id, root_commit_id, &mut uncached_graph)?;
    std::fs::remove_file(repo.shallow_file()?)?;
    assert_history_queries_match_git(&repo, tip_commit_id, root_commit_id, &mut uncached_graph)?;
    assert_history_queries_match_git(&repo, tip_commit_id, root_commit_id, &mut graph)?;
    Ok(())
}

#[test]
fn replacements_disable_stale_commit_graphs_like_git() -> TestResult {
    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    gix_testtools::git(
        repo.workdir().expect("the fixture has a worktree"),
        "commit --allow-empty -m third",
    )?;
    let tip_commit_id = repo.head_id()?.into();
    let root_commit_id: gix::ObjectId = repo.rev_parse_single("HEAD~2")?.into();
    let mut replacement = repo.head_commit()?.decode()?.to_owned()?;
    replacement.parents.clear();
    let replacement_commit_id = repo.write_object(replacement)?;
    gix_testtools::git(repo.git_dir(), "commit-graph write --reachable")?;
    let cache = repo.commit_graph()?;
    gix_testtools::git(
        repo.git_dir(),
        &format!("replace {tip_commit_id} {replacement_commit_id}"),
    )?;
    let repo = gix::open_opts(
        repo.git_dir(),
        crate::util::restricted().config_overrides(["core.useReplaceRefs=true"]),
    )?;
    assert!(
        repo.objects.store_ref().replacements().next().is_some(),
        "the repository object store has active replacements"
    );
    assert!(
        repo.commit_graph_if_enabled()?.is_none(),
        "replacement topology is incompatible with the cache"
    );
    for count_first in [false, true] {
        let mut graph = repo.revision_graph(Some(&cache));
        graph.get_or_insert_full_commit(tip_commit_id, |_| {})?;
        if count_first {
            assert_eq!(
                repo.ahead_behind_with_graph(tip_commit_id, root_commit_id, &mut graph)?,
                gix::revision::plumbing::AheadBehind { ahead: 1, behind: 1 },
                "counting independently discards stale cached topology"
            );
        }
        assert_history_queries_match_git(&repo, tip_commit_id, root_commit_id, &mut graph)?;
    }
    let mut repo = repo;
    repo.objects.ignore_replacements = true;
    assert!(
        repo.commit_graph_if_enabled()?.is_some(),
        "ignored replacements allow the original cache"
    );
    assert!(
        repo.is_ancestor(root_commit_id, tip_commit_id)?,
        "ignored replacements retain original ancestry"
    );
    assert_eq!(
        repo.ahead_behind(tip_commit_id, root_commit_id)?,
        gix::revision::plumbing::AheadBehind { ahead: 2, behind: 0 },
        "ignored replacements retain original counts"
    );
    Ok(())
}

fn assert_history_queries_match_git(
    repo: &gix::Repository,
    tip_commit_id: impl Into<gix::ObjectId>,
    base_commit_id: impl Into<gix::ObjectId>,
    graph: &mut gix::revision::plumbing::Graph<
        '_,
        '_,
        gix::revwalk::graph::Commit<gix::revision::plumbing::merge_base::Flags>,
    >,
) -> TestResult {
    use gix::revision::plumbing::AheadBehind;

    let tip_commit_id = tip_commit_id.into();
    let base_commit_id = base_commit_id.into();
    for (tip_commit_id, base_commit_id) in [(tip_commit_id, base_commit_id), (base_commit_id, tip_commit_id)] {
        let output = gix_testtools::git_command(repo.git_dir())
            .args([
                "merge-base",
                "--is-ancestor",
                &base_commit_id.to_string(),
                &tip_commit_id.to_string(),
            ])
            .output()?;
        assert!(
            matches!(output.status.code(), Some(0 | 1)),
            "Git must answer the ancestry query"
        );
        let expected = output.status.success();
        assert_eq!(
            repo.is_ancestor(base_commit_id, tip_commit_id)?,
            expected,
            "ancestry matches Git"
        );
        assert_eq!(
            repo.is_ancestor_with_graph(base_commit_id, tip_commit_id, graph)?,
            expected,
            "reused ancestry matches Git"
        );
        let counts = gix_testtools::git(
            repo.git_dir(),
            &format!("rev-list --left-right --count {tip_commit_id}...{base_commit_id}"),
        )?;
        let mut counts = counts.split_whitespace();
        let expected = AheadBehind {
            ahead: counts.next().expect("Git returns the left count").parse()?,
            behind: counts.next().expect("Git returns the right count").parse()?,
        };
        assert_eq!(
            repo.ahead_behind(tip_commit_id, base_commit_id)?,
            expected,
            "counts match Git"
        );
        assert_eq!(
            repo.ahead_behind_with_graph(tip_commit_id, base_commit_id, graph)?,
            expected,
            "reused counts match Git"
        );
    }
    Ok(())
}

#[test]
fn date() -> TestResult {
    let repo = crate::named_repo("make_rev_parse_repo.sh")?;
    let actual = repo
        .rev_parse_single("old@{20 years ago}")
        .expect("it returns the oldest possible rev when overshooting");
    assert_eq!(actual, "be2f093f0588eaeb71e1eff7451b18c2a9b1d765");

    let actual = repo
        .rev_parse_single("old@{1732184844}")
        .expect("it finds something in the middle");
    assert_eq!(
        actual, "b29405fe9147a3a366c4048fbe295ea04de40fa6",
        "It also figures out that we don't mean an index, but a date"
    );
    Ok(())
}
