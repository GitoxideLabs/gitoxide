use gix_hash::ObjectId;
use gix_traverse::object::BreadthFirst;

fn ids(path: &std::path::Path) -> gix_error::TestResult<Vec<ObjectId>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(|line| ObjectId::from_hex(line.as_bytes()))
        .collect::<Result<_, _>>()?)
}

#[test]
fn reachable_objects_match_git_including_tags_and_excluding_gitlinks() -> gix_error::TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_object_graph.sh")?;
    let db = gix_odb::at(root.join(".git/objects"), gix_testtools::object_hash())?;
    let tips = ids(&root.join("tips"))?;
    let actual = BreadthFirst::new(tips.iter().copied().chain(tips.iter().copied()), &db)
        .map(|entry| entry.map(|entry| entry.id))
        .collect::<gix_error::Result<Vec<_>>>()?;
    let unique: std::collections::BTreeSet<_> = actual.iter().copied().collect();
    assert_eq!(
        actual.len(),
        unique.len(),
        "shared objects and repeated tips are returned once"
    );
    assert_eq!(
        unique,
        ids(&root.join("reachable"))?.into_iter().collect(),
        "all four object types follow the same edges as Git"
    );
    Ok(())
}

#[test]
fn shallow_commits_keep_their_trees_but_not_their_parents() -> gix_error::TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_object_graph.sh")?;
    let db = gix_odb::at(root.join(".git/objects"), gix_testtools::object_hash())?;
    let tips = ids(&root.join("tip"))?;
    let actual = BreadthFirst::new(tips.iter().copied(), &db)
        .with_shallow_commits(tips)
        .map(|entry| entry.map(|entry| entry.id))
        .collect::<gix_error::Result<std::collections::BTreeSet<_>>>()?;
    assert_eq!(
        actual,
        ids(&root.join("shallow-reachable"))?.into_iter().collect(),
        "shallow traversal agrees with Git's shallow file"
    );
    Ok(())
}

#[test]
fn missing_objects_stop_the_iterator() -> gix_error::TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_object_graph.sh")?;
    let db = gix_odb::at(root.join(".git/objects"), gix_testtools::object_hash())?;
    let mut tips = vec![gix_testtools::object_hash().null()];
    tips.extend(ids(&root.join("tips"))?);
    let mut traversal = BreadthFirst::new(tips, &db);
    assert!(
        traversal.next().expect("one error").is_err(),
        "missing objects fail closed"
    );
    assert!(
        traversal.next().is_none(),
        "errors stop the entire walk rather than hiding missing reachability"
    );
    Ok(())
}

#[test]
fn corrupt_edges_and_malformed_objects_are_errors_even_if_seen_before() -> gix_error::TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_object_graph.sh")?;
    let db = gix_odb::at(root.join(".git/objects"), gix_testtools::object_hash())?;
    for (name, previsit_blob) in [("wrong-kind", false), ("wrong-kind", true), ("malformed", false)] {
        let mut tips = Vec::new();
        if previsit_blob {
            tips.extend(ids(&root.join("blob"))?);
        }
        tips.extend(ids(&root.join(name))?);
        let mut traversal = BreadthFirst::new(tips, &db);
        assert!(
            traversal.by_ref().collect::<gix_error::Result<Vec<_>>>().is_err(),
            "{name} must fail even when its target was visited before"
        );
        assert!(traversal.next().is_none(), "corruption fuses the iterator");
    }
    Ok(())
}
