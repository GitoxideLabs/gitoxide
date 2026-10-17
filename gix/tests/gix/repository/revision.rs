use gix_testtools::TestResult;

#[test]
fn corrupt_commit_graph_parents_retry_the_current_commit() -> TestResult {
    use gix::revision::walk::Sorting;
    use gix::traverse::commit::simple::CommitTimeOrder;

    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let head_commit_id = repo.head_id()?;
    gix_testtools::git(repo.path(), "commit-graph write --reachable")?;
    let graph = repo.commit_graph()?;
    let graph_path = repo.objects.store_ref().path().join("info/commit-graph");
    let mut data = std::fs::read(&graph_path)?;
    let table_end = 8 + usize::from(data[6]) * 12;
    let commit_data = data[8..table_end]
        .as_chunks::<12>()
        .0
        .iter()
        .find(|entry| &entry[..4] == b"CDAT")
        .expect("Git writes a commit-data chunk");
    let hash_len = repo.object_hash().len_in_bytes();
    let parent_offset = usize::try_from(u64::from_be_bytes(commit_data[4..].try_into()?))?
        + graph.lookup(head_commit_id).expect("the tip is in the graph").0 as usize * (hash_len + 16)
        + hash_len;
    data[parent_offset..parent_offset + 4].copy_from_slice(&graph.num_commits().to_be_bytes());
    drop(graph);
    // Replace Git's read-only file rather than changing a live mapping.
    let replacement = graph_path.with_file_name("corrupt-commit-graph");
    std::fs::write(&replacement, data)?;
    #[cfg(windows)]
    {
        let mut permissions = std::fs::metadata(&graph_path)?.permissions();
        #[expect(clippy::permissions_set_readonly_false, reason = "this block only runs on Windows")]
        permissions.set_readonly(false);
        std::fs::set_permissions(&graph_path, permissions)?;
    }
    std::fs::rename(replacement, graph_path)?;
    let git = gix_testtools::git_command(repo.path())
        .args(["rev-list", "HEAD"])
        .output()?;
    assert!(
        !git.status.success(),
        "Git rejects the out-of-range parent in the disposable graph"
    );

    for sorting in [
        Sorting::BreadthFirst,
        Sorting::ByCommitTime(CommitTimeOrder::NewestFirst),
        Sorting::ByCommitTime(CommitTimeOrder::OldestFirst),
        Sorting::ByCommitTimeCutoff {
            seconds: 0,
            order: CommitTimeOrder::NewestFirst,
        },
    ] {
        for first_parent_only in [false, true] {
            let expected = repo.rev_walk([head_commit_id]).use_commit_graph(false).sorting(sorting);
            let actual = repo.rev_walk([head_commit_id]).use_commit_graph(true).sorting(sorting);
            let (expected, actual) = if first_parent_only {
                (expected.first_parent_only(), actual.first_parent_only())
            } else {
                (expected, actual)
            };
            let expected = expected
                .all()?
                .map(|info| info.map(|info| info.id))
                .collect::<gix::Result<Vec<_>>>()?;
            let actual = actual
                .all()?
                .map(|info| info.map(|info| info.id))
                .collect::<gix::Result<Vec<_>>>()?;
            assert_eq!(expected.len(), 2, "the fixture contains the tip and its parent");
            assert_eq!(
                actual, expected,
                "cache fallback retains the popped commit and its ancestry"
            );
        }
    }
    Ok(())
}

#[test]
fn commit_graph_parent_buffers_respect_the_repository_allocation_limit() -> TestResult {
    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let head = repo.head_commit()?;
    let mut merge = head.decode()?.to_owned()?;
    merge.parents.clear();
    for index in 0..9 {
        let mut parent = head.decode()?.to_owned()?;
        parent.message = format!("octopus parent {index}\n").into();
        merge.parents.push(repo.write_object(parent)?.into());
    }
    merge.message = "octopus merge for allocation limit\n".into();
    let merge_commit_id = repo.write_object(merge)?;
    let mut child = gix_testtools::git_command(repo.path())
        .args(["commit-graph", "write", "--stdin-commits"])
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    use std::io::Write;
    writeln!(child.stdin.take().expect("stdin was piped"), "{merge_commit_id}")?;
    assert!(
        child.wait()?.success(),
        "Git writes an octopus commit graph in the disposable repository"
    );
    assert!(
        repo.commit_graph_if_enabled()?.is_some(),
        "the graph is readable without a limit"
    );

    let mut parents = smallvec::SmallVec::<[(gix::ObjectId, (u32, i64)); 1]>::new();
    let graph = repo.commit_graph()?;
    for parent in graph
        .commit_by_id(merge_commit_id)
        .expect("Git includes the merge")
        .iter_parents()
    {
        let parent = graph.commit_at(parent?);
        parents.push((
            parent.id().to_owned(),
            (parent.generation(), parent.committer_timestamp() as i64),
        ));
    }
    assert_eq!(
        parents.len(),
        9,
        "the graph exercises growth to a sixteen-element topological buffer"
    );
    let parent_buffer_bytes = parents.capacity() * std::mem::size_of::<(gix::ObjectId, (u32, i64))>();
    let limited = gix::open_opts(
        repo.path(),
        gix::open::Options::isolated()
            .config_overrides([format!("gitoxide.objects.allocLimit={}", parent_buffer_bytes - 1)]),
    )?;
    for err in [
        limited
            .commit_graph()
            .err()
            .expect("explicit graph loading applies the limit"),
        limited
            .commit_graph_if_enabled()
            .err()
            .expect("optional graph loading applies the limit"),
    ] {
        assert!(
            err.classify().any(|classification| classification.class()
                == gix::error::Class::ResourceExhaustion(gix::error::ResourceExhaustionKind::AllocationLimit)),
            "the repository preserves allocation-limit failures"
        );
    }
    Ok(())
}

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
