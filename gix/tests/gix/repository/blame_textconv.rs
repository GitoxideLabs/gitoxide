use gix::bstr::ByteSlice;
use gix_testtools::TestResult;

#[test]
fn textconv_blame_matches_git_across_history_and_supplied_contents() -> TestResult {
    let repo = crate::named_repo("make_blame_textconv.sh")?;
    let head = repo.head_id()?.detach();
    for (path, committed) in ["renamed.dat", "history.dat", "override.dat"]
        .into_iter()
        .flat_map(|path| [true, false].map(|committed| (path, committed)))
    {
        let start = if committed {
            gix::blame::Start::Commit(head)
        } else {
            gix::blame::Start::Contents {
                first_suspect: head,
                contents: b"first\nSECOND\nTHIRD\nfourth\n".as_slice().into(),
            }
        };
        let outcome = repo.blame_file_from(
            path.into(),
            start,
            gix::repository::blame_file::Options {
                rewrites: Some(Default::default()),
                textconv: true,
                ..Default::default()
            },
        )?;
        if !committed {
            let worktree = repo.blame_worktree_file(
                path.into(),
                gix::repository::blame_file::Options {
                    rewrites: Some(Default::default()),
                    textconv: true,
                    ..Default::default()
                },
            )?;
            assert_eq!(worktree.blob, outcome.blob, "worktree conversion matches supplied text");
            assert_eq!(
                worktree.entries, outcome.entries,
                "worktree and supplied contents share attribution"
            );
        }
        let mut command = gix_testtools::git_command(repo.workdir().expect("fixture has a worktree"));
        command.args(["blame", "--line-porcelain"]);
        if committed {
            command.arg("HEAD");
        }
        let oracle = command.args(["--", path]).output()?;
        assert!(oracle.status.success());
        let mut oracle_lines = oracle.stdout.lines();
        for (entry, lines) in outcome.entries_with_lines() {
            for (offset, line) in lines.iter().enumerate() {
                let header = oracle_lines.next().expect("one entry per converted line").to_str()?;
                let fields = header.split_whitespace().collect::<Vec<_>>();
                assert_eq!(fields[0], entry.commit_id.to_string());
                assert_eq!(
                    fields[1].parse::<u32>()?,
                    entry.start_in_source_file + offset as u32 + 1
                );
                let text = oracle_lines
                    .by_ref()
                    .find_map(|line| line.strip_prefix(b"\t"))
                    .expect("porcelain text");
                assert_eq!(text, line.strip_suffix(b"\n").unwrap_or(line));
            }
        }
        assert!(oracle_lines.next().is_none(), "all converted lines are attributed");
    }
    Ok(())
}

#[test]
fn textconv_is_opt_in_skips_symlinks_and_propagates_failure() -> TestResult {
    let mut repo = crate::named_repo("make_blame_textconv.sh")?;
    let head = repo.head_id()?.detach();
    assert_eq!(
        repo.blame_file("history.dat".into(), head, Default::default())?.blob,
        b"first|SECOND|THIRD|"
    );
    let options = gix::repository::blame_file::Options {
        textconv: true,
        ..Default::default()
    };
    assert_eq!(
        repo.blame_file("link.dat".into(), head, options.clone())?.blob,
        b"first|SECOND|"
    );
    repo.config_snapshot_mut()
        .set_raw_value("diff.render.textconv", "false")?;
    repo.blame_file("history.dat".into(), head, options)
        .expect_err("failed converters abort blame");
    assert_eq!(repo.head_id()?.detach(), head, "blame never changes the repository tip");
    Ok(())
}
