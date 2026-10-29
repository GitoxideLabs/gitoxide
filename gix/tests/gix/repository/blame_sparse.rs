use gix::bstr::ByteSlice;
use gix_testtools::TestResult;

#[test]
fn staged_rename_expands_sparse_index_only_in_memory() -> TestResult {
    let repo = crate::named_repo("make_blame_sparse.sh")?;
    assert!(
        repo.index()?.is_sparse(),
        "fixture must retain sparse directory entries"
    );
    let index_before = std::fs::read(repo.index_path())?;
    let workdir = repo.workdir().expect("fixture worktree");
    let objects_before = gix_testtools::git(workdir, "count-objects -v")?;
    let outcome = repo.blame_worktree_file(
        "included/new.txt".into(),
        gix::repository::blame_file::Options {
            rewrites: Some(Default::default()),
            ..Default::default()
        },
    )?;
    let git = gix_testtools::git_command(workdir)
        .args(["blame", "--line-porcelain", "--", "included/new.txt"])
        .output()?;
    assert!(git.status.success(), "Git oracle succeeds");
    let expected = git
        .stdout
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(|byte| *byte == b' ');
            let commit = gix::ObjectId::from_hex(fields.next()?).ok()?;
            let original = fields.next()?.to_str().ok()?.parse::<u32>().ok()?;
            let final_line = fields.next()?.to_str().ok()?.parse::<u32>().ok()?;
            Some((commit, original, final_line))
        })
        .collect::<Vec<_>>();
    let actual = outcome
        .entries
        .iter()
        .flat_map(|entry| {
            (0..entry.len.get()).map(|offset| {
                (
                    entry.commit_id,
                    entry.start_in_source_file + offset + 1,
                    entry.start_in_blamed_file + offset + 1,
                )
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "sparse rename attribution matches Git");
    assert_eq!(
        std::fs::read(repo.index_path())?,
        index_before,
        "the persisted index stays sparse and unchanged"
    );
    assert_eq!(
        gix_testtools::git(workdir, "count-objects -v")?,
        objects_before,
        "expansion writes no objects to disk"
    );
    Ok(())
}
