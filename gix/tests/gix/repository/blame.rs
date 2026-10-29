use gix::bstr::BString;
use gix_testtools::TestResult;
use std::num::NonZero;

#[test]
fn simple() -> TestResult {
    let repo = crate::named_repo("make_blame_repo.sh")?;

    let suspect = repo.head_id()?;
    let outcome = repo.blame_file("simple.txt".into(), suspect, Default::default())?;

    assert_eq!(outcome.entries.len(), 4);

    Ok(())
}

#[test]
fn with_options() -> TestResult {
    let repo = crate::named_repo("make_blame_repo.sh")?;

    let options = gix::repository::blame_file::Options {
        ranges: gix::blame::BlameRanges::from_one_based_inclusive_range(1..=2)?,
        ..Default::default()
    };

    let suspect = repo.head_id()?;
    let outcome = repo.blame_file("simple.txt".into(), suspect, options)?;

    assert_eq!(outcome.entries.len(), 2);

    let entries_with_lines: Vec<_> = outcome.entries_with_lines().collect();

    assert!(matches!(
        entries_with_lines.as_slice(),
        &[
            (
                gix::blame::BlameEntry {
                    start_in_blamed_file: 0,
                    start_in_source_file: 0,
                    source_file_name: None,
                    ..
                },
                _,
            ),
            (
                gix::blame::BlameEntry {
                    start_in_blamed_file: 1,
                    start_in_source_file: 1,
                    source_file_name: None,
                    ..
                },
                _,
            )
        ]
    ));

    assert_eq!(entries_with_lines[0].0.len, NonZero::new(1).unwrap());
    assert_eq!(entries_with_lines[1].0.len, NonZero::new(1).unwrap());

    assert_eq!(entries_with_lines[0].1, vec![BString::new("line 1\n".into())]);
    assert_eq!(entries_with_lines[1].1, vec![BString::new("line 2\n".into())]);

    Ok(())
}

#[test]
fn contents_and_previous_origins_match_git() -> TestResult {
    use gix::bstr::ByteSlice;
    let repo = crate::named_repo("make_blame_origins.sh")?;
    let head = repo.head_id()?;
    let contents = std::fs::read(repo.workdir().expect("fixture has a worktree").join("renamed.txt"))?;
    for start in [
        gix::blame::Start::Commit(head.into()),
        gix::blame::Start::Contents {
            first_suspect: head.into(),
            contents: contents.into(),
        },
    ] {
        let committed = matches!(&start, gix::blame::Start::Commit(_));
        let outcome = repo.blame_file_from(
            "renamed.txt".into(),
            start,
            gix::repository::blame_file::Options {
                rewrites: Some(Default::default()),
                ..Default::default()
            },
        )?;
        let mut git = gix_testtools::git_command(repo.workdir().expect("fixture has a worktree"));
        git.args(["blame", "--line-porcelain"]);
        if committed {
            git.arg("HEAD");
        }
        let oracle = git.args(["--", "renamed.txt"]).output()?;
        assert!(oracle.status.success(), "Git oracle must succeed");
        let mut oracle_lines = oracle.stdout.lines();
        for (entry, lines) in outcome.entries_with_lines() {
            for (offset, line) in lines.iter().enumerate() {
                let header = oracle_lines
                    .next()
                    .expect("one oracle entry per source line")
                    .to_str()?;
                let fields = header.split_whitespace().collect::<Vec<_>>();
                assert_eq!(fields[0], entry.commit_id.to_string(), "source commit");
                assert_eq!(
                    fields[1].parse::<u32>()?,
                    entry.start_in_source_file + offset as u32 + 1,
                    "source line"
                );
                assert_eq!(
                    fields[2].parse::<u32>()?,
                    entry.start_in_blamed_file + offset as u32 + 1,
                    "final line"
                );
                let mut previous = None;
                for metadata in oracle_lines.by_ref() {
                    if let Some(source) = metadata.strip_prefix(b"\t") {
                        assert_eq!(source, line.trim_end_with(|b| b == '\n'), "line contents");
                        break;
                    }
                    if let Some(origin) = metadata.strip_prefix(b"previous ") {
                        previous = Some(origin.to_str()?.to_owned());
                    }
                }
                let native = outcome
                    .previous_origins
                    .get(&(entry.commit_id, entry.source_file_name.clone()))
                    .map(|origin| format!("{} {}", origin.commit_id, origin.source_file_name));
                assert_eq!(
                    native, previous,
                    "previous origin must match Git including renames and uncommitted lines"
                );
            }
        }
        assert!(oracle_lines.next().is_none(), "all oracle lines are covered");
    }
    Ok(())
}

#[test]
fn shallow_boundary_stops_blame_even_with_parent_objects() -> TestResult {
    let fixture = crate::named_repo("make_blame_origins.sh")?;
    let repo = gix::open_opts(
        fixture.workdir().expect("fixture worktree").join("shallow.git"),
        gix::open::Options::isolated(),
    )?;
    let head = repo.head_id()?;
    let outcome = repo.blame_file("renamed.txt".into(), head, Default::default())?;
    assert_eq!(outcome.entries.len(), 1, "a shallow tip owns every line");
    assert_eq!(outcome.entries[0].commit_id, head, "parents are not traversed");
    assert!(
        outcome.previous_origins.is_empty(),
        "a shallow root has no previous origin"
    );
    Ok(())
}

#[test]
fn type_changes_do_not_transfer_blame_even_for_identical_blob_contents() -> TestResult {
    let repo = crate::named_repo("make_blame_origins.sh")?;
    let head = repo.head_id()?;
    let committed = repo.blame_file("typechange.txt".into(), head, Default::default())?;
    assert_eq!(
        committed.entries[0].commit_id, head,
        "changing a regular file to a symlink introduces its contents"
    );
    assert!(
        committed.previous_origins.is_empty(),
        "the prior regular file is not the symlink's origin"
    );
    let worktree = repo.blame_file_from(
        "typechange.txt".into(),
        gix::blame::Start::Contents {
            first_suspect: head.into(),
            contents: b"target".as_slice().into(),
        },
        Default::default(),
    )?;
    assert!(
        worktree.entries[0].commit_id.is_null(),
        "replacing the symlink with a regular file introduces all lines"
    );
    assert!(
        worktree.previous_origins.is_empty(),
        "the prior symlink is not the regular file's origin"
    );
    Ok(())
}
