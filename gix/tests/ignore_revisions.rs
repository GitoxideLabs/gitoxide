#![cfg(all(feature = "blame", feature = "revision"))]

use gix::{bstr::ByteSlice, repository::blame_file::Options};
use gix_testtools::TestResult;

fn repo_with(name: &str, config: &[&str]) -> TestResult<gix::Repository> {
    let path = gix_testtools::scripted_fixture_read_only("make_blame_ignore_revisions.sh")?;
    Ok(gix::open_opts(
        path.join(name),
        gix::open::Options::isolated().config_overrides(config.iter().copied()),
    )?)
}

fn compare_with_git(repo: &gix::Repository, options: Options, args: &[&str]) -> TestResult<gix::blame::Outcome> {
    let result = repo.blame_file("file".into(), repo.head_id()?, options)?;
    let oracle = gix_testtools::git_command(repo.workdir().expect("fixture worktree"))
        .args([
            "-c",
            "blame.markIgnoredLines=true",
            "-c",
            "blame.markUnblamableLines=true",
            "blame",
            "--line-porcelain",
        ])
        .args(args)
        .args(["HEAD", "--", "file"])
        .output()?;
    assert!(oracle.status.success(), "Git blame oracle: {}", oracle.stderr.as_bstr());
    let mut expected = oracle.stdout.lines();
    for (entry, lines) in result.entries_with_lines() {
        for (offset, line) in lines.iter().enumerate() {
            let header = expected.next().expect("one header per blamed line").to_str()?;
            let fields: Vec<_> = header.split_whitespace().collect();
            assert_eq!(
                fields[0],
                entry.commit_id.to_string(),
                "same originating commit for line {}",
                entry.start_in_blamed_file + offset as u32 + 1
            );
            assert_eq!(
                fields[1].parse::<u32>()?,
                entry.start_in_source_file + offset as u32 + 1,
                "same source line"
            );
            let final_line = entry.start_in_blamed_file + offset as u32;
            let (mut ignored, mut unblamable) = (false, false);
            let mut text = None;
            for metadata in expected.by_ref() {
                if let Some(contents) = metadata.strip_prefix(b"\t") {
                    text = Some(contents);
                    break;
                }
                ignored |= metadata == b"ignored";
                unblamable |= metadata == b"unblamable";
                if let Some(path) = metadata.strip_prefix(b"filename ") {
                    assert_eq!(
                        path,
                        entry
                            .source_file_name
                            .as_ref()
                            .map_or(b"file".as_slice(), |path| path.as_slice()),
                        "same source path"
                    );
                }
            }
            assert_eq!(
                ignored,
                result.ignored_lines.contains(&final_line),
                "ignored marker at line {final_line}"
            );
            // Git retains a failed earlier parent's marker even if a later parent succeeds.
            // The merge test checks that we clear this stale flag instead.
            if !(ignored && unblamable) {
                assert_eq!(
                    unblamable,
                    result.unblamable_lines.contains(&final_line),
                    "unblamable marker at line {final_line}"
                );
            }
            let text = text.expect("porcelain text");
            assert_eq!(text, line.strip_suffix(b"\n").unwrap_or(line), "same final contents");
        }
    }
    assert!(expected.next().is_none(), "all oracle lines accounted for");
    Ok(result)
}

#[test]
fn configured_ignored_revisions_match_git() -> TestResult {
    let result = compare_with_git(&repo_with("", &[])?, Options::default(), &[])?;
    assert_eq!(
        result.ignored_lines,
        [0, 1].into(),
        "rewritten lines trace through both ignored commits"
    );
    assert_eq!(
        result.unblamable_lines,
        [2, 3].into(),
        "unrelated additions stay with the ignored commit"
    );
    Ok(())
}

#[test]
fn fuzzy_line_matching_merges_and_renames_match_git() -> TestResult {
    for case in [
        "merge-exact",
        "merge-fuzzy",
        "renamed",
        "fuzzy-join",
        "fuzzy-split",
        "fuzzy-repeated",
        "fuzzy-order",
        "fuzzy-offset",
        "split-addition",
    ] {
        eprintln!("case: {case}");
        let result = compare_with_git(
            &repo_with(case, &[])?,
            Options {
                rewrites: Some(Default::default()),
                ..Default::default()
            },
            &[],
        )?;
        if case == "merge-fuzzy" {
            assert_eq!(
                result.ignored_lines,
                [0].into(),
                "the second parent explains the changed line"
            );
            assert!(
                result.unblamable_lines.is_empty(),
                "failed attribution to the first parent must not leave a stale marker"
            );
        }
    }
    Ok(())
}

#[test]
fn explicit_revisions_override_config_and_peel_tags() -> TestResult {
    let repo = repo_with("", &[])?;
    let tag = repo.rev_parse_single("XT")?.detach();
    let newer = repo.rev_parse_single("Y")?.detach();
    compare_with_git(
        &repo,
        Options {
            ignored_revisions: Some(vec![tag, newer]),
            ..Default::default()
        },
        &["--ignore-revs-file", "", "--ignore-rev", "XT", "--ignore-rev", "Y"],
    )?;
    compare_with_git(
        &repo,
        Options {
            ignored_revisions: Some(Vec::new()),
            ..Default::default()
        },
        &["--ignore-revs-file", ""],
    )?;
    for object_id in [repo.head_commit()?.tree_id()?.detach(), repo.object_hash().null()] {
        assert!(
            repo.blame_file(
                "file".into(),
                repo.head_id()?,
                Options {
                    ignored_revisions: Some(vec![object_id]),
                    ..Default::default()
                }
            )
            .is_err(),
            "explicit exclusions must identify commits"
        );
    }
    Ok(())
}

#[test]
fn files_handle_comments_missing_objects_and_config_resets() -> TestResult {
    let cases = [
        (
            ["blame.ignoreRevsFile=", "blame.ignoreRevsFile=noncommit"],
            vec!["--ignore-revs-file", "", "--ignore-revs-file", "noncommit"],
        ),
        (
            ["blame.ignoreRevsFile=", "blame.ignoreRevsFile=annotated"],
            vec!["--ignore-revs-file", "", "--ignore-revs-file", "annotated"],
        ),
        (
            ["blame.ignoreRevsFile=", "blame.ignoreRevsFile=only-y"],
            vec!["--ignore-revs-file", "", "--ignore-revs-file", "only-y"],
        ),
        (
            ["blame.ignoreRevsFile=only-x", "blame.ignoreRevsFile="],
            vec!["--ignore-revs-file", ""],
        ),
        (
            ["blame.ignoreRevsFile=", "blame.ignoreRevsFile=:(optional)absent"],
            vec!["--ignore-revs-file", ""],
        ),
        (
            ["blame.ignoreRevsFile=", "blame.ignoreRevsFile=:(optional)"],
            vec!["--ignore-revs-file", ""],
        ),
    ];
    for (config, git_args) in cases {
        compare_with_git(&repo_with("", &config)?, Options::default(), &git_args)?;
    }
    Ok(())
}

#[test]
fn malformed_or_unreadable_ignore_files_fail() -> TestResult {
    for value in ["invalid", "missing", ".git"] {
        let repo = repo_with("", &[&format!("blame.ignoreRevsFile={value}")])?;
        assert!(
            repo.blame_file("file".into(), repo.head_id()?, Options::default())
                .is_err(),
            "invalid or unreadable {value:?} must be reported"
        );
    }
    Ok(())
}

#[test]
fn line_ranges_keep_source_positions_and_markers() -> TestResult {
    for case in ["", "fuzzy-split", "split-addition"] {
        compare_with_git(
            &repo_with(case, &[])?,
            Options {
                ranges: gix::blame::BlameRanges::from_one_based_inclusive_range(2..=3)?,
                ..Default::default()
            },
            &["-L", "2,3"],
        )?;
    }
    Ok(())
}

#[test]
fn configured_blame_works_after_cwd_changes_without_git_on_path() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_blame_ignore_revisions.sh")?.canonicalize()?;
    let original_cwd = gix_testtools::set_current_dir(&fixture)?;
    let repo = gix::open_opts(&fixture, gix::open::Options::isolated())?;
    drop(original_cwd);
    let _path = gix_testtools::Env::new().set("PATH", "");
    let result = repo.blame_file("file".into(), repo.head_id()?, Options::default())?;
    assert_eq!(result.entries[0].commit_id, repo.rev_parse_single("A")?.detach());
    assert_eq!(result.entries[1].commit_id, repo.rev_parse_single("B")?.detach());
    assert_eq!(result.ignored_lines, [0, 1].into());
    Ok(())
}
