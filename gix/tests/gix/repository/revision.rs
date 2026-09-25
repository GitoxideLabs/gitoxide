use crate::Result;

#[test]
fn missing_objects_info_does_not_prevent_merge_base() -> Result {
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
fn merge_base_variants_find_common_ancestors() -> Result {
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
fn unrelated_histories_have_no_merge_base() -> Result {
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
fn date() -> Result {
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
