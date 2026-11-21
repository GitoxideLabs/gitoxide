use gix::bstr::ByteSlice;
use gix_testtools::TestResult;

#[test]
fn merged_histories_keep_distinct_source_paths_at_the_same_commit() -> TestResult {
    let repo = crate::named_repo("make_blame_multiple_paths.sh")?;
    let outcome = repo.blame_worktree_file(
        "merged.txt".into(),
        gix::repository::blame_file::Options {
            rewrites: Some(Default::default()),
            ..Default::default()
        },
    )?;
    let git = gix_testtools::git_command(repo.workdir().expect("fixture worktree"))
        .args(["blame", "--line-porcelain", "--", "merged.txt"])
        .output()?;
    assert!(git.status.success(), "Git oracle succeeds");
    let mut oracle = git.stdout.lines();
    for (entry, lines) in outcome.entries_with_lines() {
        for (offset, source) in lines.iter().enumerate() {
            let header = oracle
                .next()
                .expect("one header per line")
                .to_str()?
                .split_whitespace()
                .collect::<Vec<_>>();
            assert_eq!(entry.commit_id.to_string(), header[0], "originating commit");
            assert_eq!(
                entry.start_in_source_file + offset as u32 + 1,
                header[1].parse::<u32>()?,
                "original line"
            );
            let mut filename = None;
            for line in oracle.by_ref() {
                if let Some(text) = line.strip_prefix(b"\t") {
                    assert_eq!(text, source.trim_end_with(|c| c == '\n'), "source content");
                    break;
                }
                if let Some(path) = line.strip_prefix(b"filename ") {
                    filename = Some(path.to_str()?.to_owned());
                }
            }
            assert_eq!(
                entry
                    .source_file_name
                    .as_ref()
                    .map(|path| path.to_str().expect("ASCII fixture")),
                filename.as_deref(),
                "originating filename"
            );
        }
    }
    assert!(oracle.next().is_none(), "all lines attributed");
    assert_eq!(
        outcome.entries.len(),
        2,
        "adjacent ranges from different source paths must not coalesce"
    );
    Ok(())
}
