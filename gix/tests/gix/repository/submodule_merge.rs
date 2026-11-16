use gix::{ObjectId, bstr::BString, merge::tree::TreatAsUnresolved};
use gix_testtools::TestResult;

fn fixture() -> gix_testtools::Result<Option<std::path::PathBuf>> {
    gix_testtools::scripted_fixture_read_only_with_git_version(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../gix-merge/tests/fixtures/tree-submodule-history.sh"),
        |version| version >= (2, 38, 0),
    )
}

#[test]
fn submodule_history_matches_git_trees_and_index_stages() -> TestResult {
    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    let cases = std::fs::read_to_string(fixture.join("cases"))?;
    for layout in cases
        .lines()
        .filter(|layout| !matches!(*layout, "corrupt-parent" | "malformed-unavailable"))
    {
        let root = fixture.join(layout);
        let repo = gix::open_opts(&root, crate::util::restricted())?.with_object_memory();
        let tree_ids = ids(&root.join("trees"))?;
        let commit_ids = ids(&root.join("commits"))?;
        for (current, other, direction) in [(1, 2, "A-B"), (2, 1, "B-A")] {
            if !layout.starts_with("recursive-") {
                let outcome = repo.merge_trees(
                    tree_ids[0],
                    tree_ids[current],
                    tree_ids[other],
                    Default::default(),
                    repo.tree_merge_options()?,
                )?;
                assert_git_result(&repo, outcome, &root, direction, &format!("{layout} {direction} tree"))?;
            }
            let outcome = repo.merge_commits(
                commit_ids[current],
                commit_ids[other],
                Default::default(),
                repo.tree_merge_options()?.into(),
            )?;
            assert_git_result(
                &repo,
                outcome.tree_merge,
                &root,
                direction,
                &format!("{layout} {direction} commit"),
            )?;
        }
    }
    Ok(())
}

#[test]
fn required_submodule_discovery_and_ancestry_errors_match_git() -> TestResult {
    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    for layout in ["corrupt-parent", "malformed-unavailable"] {
        let root = fixture.join(layout);
        let repo = gix::open_opts(&root, crate::util::restricted())?.with_object_memory();
        let tree_ids = ids(&root.join("trees"))?;
        let commit_ids = ids(&root.join("commits"))?;
        for (current, other, direction) in [(1, 2, "A-B"), (2, 1, "B-A")] {
            assert_eq!(
                std::fs::read_to_string(root.join(format!("{direction}.status")))?.trim(),
                "128",
                "{layout} {direction}: Git also rejects corrupt required metadata or history"
            );
            assert!(
                repo.merge_trees(
                    tree_ids[0],
                    tree_ids[current],
                    tree_ids[other],
                    Default::default(),
                    repo.tree_merge_options()?
                )
                .is_err(),
                "{layout} {direction}: tree merging propagates the required submodule error"
            );
            assert!(
                repo.merge_commits(
                    commit_ids[current],
                    commit_ids[other],
                    Default::default(),
                    repo.tree_merge_options()?.into()
                )
                .is_err(),
                "{layout} {direction}: commit merging propagates the required submodule error"
            );
        }
    }
    Ok(())
}

fn ids(path: &std::path::Path) -> gix_testtools::Result<Vec<ObjectId>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(|line| ObjectId::from_hex(line.as_bytes()))
        .collect::<Result<_, _>>()?)
}

type IndexEntry = (BString, ObjectId, u32, u8);

fn entries(index: &gix::index::State) -> Vec<IndexEntry> {
    index
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.path(index).to_owned(),
                entry.id,
                entry.mode.bits(),
                entry.stage() as u8,
            )
        })
        .collect()
}

fn assert_git_result(
    repo: &gix::Repository,
    mut outcome: gix::merge::tree::Outcome<'_>,
    root: &std::path::Path,
    direction: &str,
    context: &str,
) -> TestResult {
    let merge_info = std::fs::read_to_string(root.join(format!("{direction}.merge-info")))?;
    let mut fields = merge_info.split('\0');
    let expected_tree_id = ObjectId::from_hex(fields.next().expect("Git records the result tree first").as_bytes())?;
    let mut stages = Vec::new();
    for field in fields {
        let Some((metadata, path)) = field.split_once('\t') else {
            break;
        };
        let mut metadata = metadata.split(' ');
        let mode = u32::from_str_radix(metadata.next().expect("stage entry mode"), 8)?;
        let id = ObjectId::from_hex(metadata.next().expect("stage entry object ID").as_bytes())?;
        let stage = metadata.next().expect("stage entry number").parse()?;
        stages.push((BString::from(path), id, mode, stage));
    }
    let expected_index = gix::index::State::from_tree(&expected_tree_id, repo, Default::default())?;
    let mut expected_entries = entries(&expected_index);
    expected_entries.retain(|(path, ..)| !stages.iter().any(|(stage_path, ..)| stage_path == path));
    expected_entries.extend(stages);
    expected_entries.sort_by(|a, b| (&a.0, a.3).cmp(&(&b.0, b.3)));
    let actual_index = outcome.index(TreatAsUnresolved::git())?;
    actual_index.verify_entries()?;
    assert_eq!(
        outcome.tree.write()?,
        expected_tree_id,
        "{context}: selected tree matches Git"
    );
    assert_eq!(
        entries(&actual_index),
        expected_entries,
        "{context}: every index path, mode, object and stage matches Git"
    );
    let status = std::fs::read_to_string(root.join(format!("{direction}.status")))?;
    assert_eq!(
        outcome.has_unresolved_conflicts(TreatAsUnresolved::git()),
        status.trim() == "1",
        "{context}: conflict verdict matches Git"
    );
    Ok(())
}
