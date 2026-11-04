use gix::bstr::ByteSlice;
use gix_testtools::TestResult;

#[test]
fn staged_renames_use_working_contents_without_writing_objects_or_index() -> TestResult {
    let fixture = crate::named_repo("make_blame_worktree.sh")?;
    for (directory, file_path) in [
        ("staged", "new.txt"),
        ("rewritten", "new.txt"),
        ("merging", "old.txt"),
        ("renamed-merge", "renamed.txt"),
    ] {
        let path = fixture.workdir().expect("fixture has a worktree").join(directory);
        let repo = gix::open_opts(&path, gix::open::Options::isolated())?;
        let index_before = std::fs::read(repo.index_path())?;
        let objects_before = gix_testtools::git(&path, "count-objects -v")?;
        let outcome = repo.blame_worktree_file(
            file_path.into(),
            gix::repository::blame_file::Options {
                rewrites: Some(Default::default()),
                ..Default::default()
            },
        )?;
        let output = gix_testtools::git_command(&path)
            .args(["blame", "--line-porcelain", "--", file_path])
            .output()?;
        assert!(output.status.success(), "Git oracle succeeds");
        let mut oracle = output.stdout.lines();
        for (entry, lines) in outcome.entries_with_lines() {
            for (offset, line) in lines.iter().enumerate() {
                let header = oracle
                    .next()
                    .expect("one header for every line")
                    .to_str()?
                    .split_whitespace()
                    .collect::<Vec<_>>();
                assert_eq!(header[0], entry.commit_id.to_string(), "{directory}: source commit");
                assert_eq!(
                    header[1].parse::<u32>()?,
                    entry.start_in_source_file + offset as u32 + 1,
                    "source line"
                );
                assert_eq!(
                    header[2].parse::<u32>()?,
                    entry.start_in_blamed_file + offset as u32 + 1,
                    "final line"
                );
                let mut previous = None;
                let mut source_path = None;
                for metadata in oracle.by_ref() {
                    if let Some(source) = metadata.strip_prefix(b"\t") {
                        assert_eq!(source, line.trim_end_with(|c| c == '\n'), "source contents");
                        break;
                    }
                    if let Some(path) = metadata.strip_prefix(b"filename ") {
                        source_path = Some(path.to_str()?.to_owned());
                    }
                    if let Some(value) = metadata.strip_prefix(b"previous ") {
                        previous = Some(value.to_str()?.to_owned());
                    }
                }
                assert_eq!(
                    entry
                        .source_file_name
                        .as_ref()
                        .map_or(file_path.into(), |path| path.as_bstr())
                        .to_str()?,
                    source_path.expect("Git emits filename"),
                    "source path"
                );
                let native = outcome
                    .previous_origins
                    .get(&(entry.commit_id, entry.source_file_name.clone()))
                    .map(|origin| format!("{} {}", origin.commit_id, origin.source_file_name));
                assert_eq!(native, previous, "previous origin follows the rename");
            }
        }
        assert!(oracle.next().is_none(), "all oracle lines were compared");
        assert_eq!(
            std::fs::read(repo.index_path())?,
            index_before,
            "blame does not rewrite the index"
        );
        assert_eq!(
            gix_testtools::git(&path, "count-objects -v")?,
            objects_before,
            "synthetic objects stay in memory"
        );
    }
    let invalid = gix::open_opts(
        fixture.workdir().expect("fixture worktree").join("invalid-merge"),
        gix::open::Options::isolated(),
    )?;
    assert!(
        invalid
            .blame_worktree_file("old.txt".into(), Default::default())
            .is_err(),
        "invalid merge heads must fail"
    );
    Ok(())
}

#[test]
fn worktree_blame_rejects_untracked_and_unsupported_paths() -> TestResult {
    let fixture = crate::named_repo("make_blame_worktree.sh")?;
    let path = fixture.workdir().expect("fixture has a worktree").join("staged");
    let repo = gix::open_opts(&path, gix::open::Options::isolated())?;
    assert!(
        repo.blame_file_from(
            "new.txt".into(),
            gix::blame::Start::ContentsWithParents {
                parents: Vec::new(),
                contents: b"new\n".as_slice().into(),
            },
            Default::default()
        )
        .is_err(),
        "contents need at least one parent origin"
    );
    for file in ["untracked.txt", "missing.txt", "../outside.txt", ".git"] {
        assert!(
            repo.blame_worktree_file(file.into(), Default::default()).is_err(),
            "{file} must not be blamed"
        );
    }
    Ok(())
}
