use gix_diff::blob::{Algorithm, Driver};
use gix_testtools::TestResult;

use crate::util::named_repo;

#[test]
fn resource_cache() -> TestResult {
    let repo = named_repo("make_diff_repo.sh")?;
    let index = repo.index()?;
    let cache = gix::diff::resource_cache(
        &repo,
        gix::diff::blob::pipeline::Mode::ToWorktreeAndBinaryToText,
        repo.attributes_only(&index, gix_worktree::stack::state::attributes::Source::IdMapping)?
            .detach(),
        Default::default(),
    )?;
    assert_eq!(
        cache.filter.drivers(),
        &[
            Driver {
                name: "all-but-binary".into(),
                command: Some("command".into()),
                algorithm: Some(Algorithm::Histogram),
                binary_to_text_command: Some("textconv".into()),
                is_binary: None
            },
            Driver {
                name: "binary-false".into(),
                is_binary: Some(false),
                ..Default::default()
            },
            Driver {
                name: "binary-true".into(),
                is_binary: Some(true),
                ..Default::default()
            }
        ]
    );
    assert_eq!(cache.options.algorithm, Some(Algorithm::Histogram));
    assert!(
        !cache.options.skip_internal_diff_if_external_is_configured,
        "pre-set to something that makes sense for most"
    );
    assert_eq!(
        cache.filter.options.large_file_threshold_bytes,
        512 * 1024 * 1024,
        "the default value unless it's configured"
    );
    Ok(())
}

#[cfg(all(feature = "command", feature = "revision"))]
mod tools {
    use std::process::Stdio;

    use gix::{bstr::ByteSlice, diff::blob::ResourceKind, objs::tree::EntryKind};
    use gix_testtools::TestResult;

    fn output(command: &mut std::process::Command, repo: &gix::Repository) -> TestResult<std::process::Output> {
        let environment: Vec<_> = command
            .get_envs()
            .filter_map(|(key, value)| value.map(|value| (key.to_owned(), value.to_owned())))
            .collect();
        gix_testtools::configure_git_environment(command, repo.current_dir()).envs(environment);
        Ok(command.stdout(Stdio::piped()).stderr(Stdio::piped()).output()?)
    }

    #[test]
    #[serial_test::serial]
    fn difftool_uses_the_terminal_tool_and_exact_resources() -> TestResult {
        let _environment = gix_testtools::isolate_git_environment()?;
        let mut repo = crate::util::named_repo("make_basic_repo.sh")?;
        repo.config_snapshot_mut().append_config(
            [
                "diff.tool=capture",
                "diff.guitool=gui",
                "difftool.guiDefault=true",
                "difftool.gui.cmd=exit 99",
                r#"difftool.capture.cmd=printf 'path:%s\nbase:%s\ncount:%s/%s\n' "$MERGED" "$BASE" "$GIT_DIFF_PATH_COUNTER" "$GIT_DIFF_PATH_TOTAL"; printf 'old:'; cat "$LOCAL"; printf 'new:'; cat "$REMOTE""#,
            ],
            gix_config::Source::User,
        )?;
        let mut untrusted = gix_config::File::new(
            gix_config::file::Metadata::from(gix_config::Source::Local).with(gix_sec::Trust::Reduced),
        );
        untrusted.set_raw_value("difftool.capture.cmd", "exit 98")?;
        repo.config_snapshot_mut().append(untrusted)?;
        let old_blob_id = repo.rev_parse_single("HEAD~1:this")?.detach();
        let new_blob_id = repo.rev_parse_single("HEAD:this")?.detach();
        let mut resources = repo.diff_resource_cache_for_tree_diff()?;
        let old_path = b"old file;$(touch unexpected).rs".as_bstr();
        for (blob_id, path, kind) in [
            (Some(old_blob_id), old_path, ResourceKind::OldOrSource),
            (
                Some(new_blob_id),
                b"new file.rs".as_bstr(),
                ResourceKind::NewOrDestination,
            ),
        ] {
            resources.set_resource(blob_id, EntryKind::Blob, path, kind, &repo)?;
        }
        let mut command = repo.difftool_command(&resources, 1, 4)?.expect("capture is configured");
        let temporary_paths: Vec<_> = command
            .get_args()
            .skip_while(|arg| *arg != "difftool--helper")
            .skip(2)
            .step_by(3)
            .take(2)
            .map(std::path::PathBuf::from)
            .collect();
        assert!(
            temporary_paths.len() == 2 && temporary_paths.iter().all(|path| path.is_file()),
            "both blob contents live in temporary files until the command is dropped"
        );
        assert_eq!(
            temporary_paths.iter().map(|path| path.file_name()).collect::<Vec<_>>(),
            [
                Some(std::ffi::OsStr::new("old file;$(touch unexpected).rs")),
                Some(std::ffi::OsStr::new("new file.rs")),
            ],
            "tools can detect each resource's language from its original filename"
        );
        let output = output(&mut command, &repo)?;
        assert!(output.status.success(), "the terminal difftool succeeds: {output:?}");
        assert_eq!(
            output.stdout,
            b"path:old file;$(touch unexpected).rs\nbase:old file;$(touch unexpected).rs\ncount:2/4\nold:new:hello\n",
            "tool variables retain the logical path while both contents come from the selected blobs"
        );
        assert!(
            !repo.workdir().expect("non-bare fixture").join("unexpected").exists(),
            "paths never become shell code"
        );
        drop(command);
        assert!(
            temporary_paths.iter().all(|path| !path.exists()),
            "dropping the command releases its input files"
        );
        assert!(
            temporary_paths
                .iter()
                .all(|path| !path.parent().expect("temporary inputs have a directory").exists()),
            "dropping the command removes its temporary directories too"
        );
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn difftool_configuration_falls_back_and_retains_failures() -> TestResult {
        let _environment = gix_testtools::isolate_git_environment()?;
        let mut repo = crate::util::named_repo("make_basic_repo.sh")?;
        let mut resources = repo.diff_resource_cache_for_tree_diff()?;
        assert!(
            repo.difftool_command(&resources, 0, 1)?.is_none(),
            "tool selection must be configured"
        );
        repo.config_snapshot_mut().append_config(
            ["diff.tool=", "merge.tool=capture", "mergetool.capture.cmd=exit 17"],
            gix_config::Source::User,
        )?;
        let blob_id = repo.rev_parse_single("HEAD:this")?.detach();
        for kind in [ResourceKind::OldOrSource, ResourceKind::NewOrDestination] {
            resources.set_resource(Some(blob_id), EntryKind::Blob, b"this".as_bstr(), kind, &repo)?;
        }
        let mut command = repo
            .difftool_command(&resources, 0, 1)?
            .expect("merge.tool is the terminal fallback");
        assert_eq!(
            output(&mut command, &repo)?.status.code(),
            Some(17),
            "tool failures remain available to the caller"
        );
        repo.config_snapshot_mut().set_raw_value("diff.tool", "vimdiff")?;
        repo.config_snapshot_mut()
            .set_raw_value("difftool.vimdiff.path", "true")?;
        let mut command = repo
            .difftool_command(&resources, 0, 1)?
            .expect("a built-in tool needs no custom command");
        let output = output(&mut command, &repo)?;
        assert!(
            output.status.success(),
            "Git's built-in tool uses its configured executable: {output:?}"
        );
        for kind in [ResourceKind::OldOrSource, ResourceKind::NewOrDestination] {
            resources.set_resource(None, EntryKind::Blob, b"missing".as_bstr(), kind, &repo)?;
        }
        assert!(
            repo.difftool_command(&resources, 0, 1).is_err(),
            "a tool cannot inspect two missing resources"
        );
        Ok(())
    }

    #[test]
    fn difftool_anchors_relative_and_bare_repositories_after_a_cwd_change() -> TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let fixture = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;
        let _cwd = gix_testtools::set_current_dir(&fixture)?;
        let main = gix::open_opts(".", gix::open::Options::isolated())?;
        let mut bare = gix::open_opts(
            "non-bare-without-worktree",
            gix::open::Options::isolated()
                .open_path_as_is(true)
                .config_overrides(["core.bare=true"]),
        )?;
        bare.set_workdir(None)?;
        let elsewhere = gix_testtools::tempfile::tempdir()?;
        for mut repo in [main, bare] {
            repo.config_snapshot_mut().append_config(
                [
                    "diff.tool=capture",
                    r#"difftool.capture.cmd=printf 'old:'; if test "$LOCAL" = /dev/null; then printf 'missing\n'; else cat "$LOCAL"; fi; printf 'new:'; if test "$REMOTE" = /dev/null; then printf 'missing\n'; else cat "$REMOTE"; fi; printf 'cwd:%s\n' "$PWD""#,
                ],
                gix_config::Source::User,
            )?;
            let blob_id = repo.rev_parse_single("HEAD:this")?.detach();
            let mut resources = repo.diff_resource_cache_for_tree_diff()?;
            for (old, new, expected) in [
                (None, Some(blob_id), "old:missing\nnew:hello\n"),
                (Some(blob_id), None, "old:hello\nnew:missing\n"),
            ] {
                for (blob_id, kind) in [(old, ResourceKind::OldOrSource), (new, ResourceKind::NewOrDestination)] {
                    resources.set_resource(blob_id, EntryKind::Blob, b"this".as_bstr(), kind, &repo)?;
                }
                let _moved_cwd = gix_testtools::set_current_dir(elsewhere.path())?;
                let mut command = repo.difftool_command(&resources, 0, 1)?.expect("capture is configured");
                let output = output(&mut command, &repo)?;
                assert!(
                    output.status.success(),
                    "difftools work outside the original CWD: {output:?}"
                );
                let expected_dir = repo
                    .current_dir()
                    .join(repo.workdir().unwrap_or(repo.git_dir()))
                    .canonicalize()?;
                assert_eq!(
                    String::from_utf8(output.stdout)?.replace("\r\n", "\n"),
                    format!("{expected}cwd:{}\n", expected_dir.display()),
                    "additions and deletions use the captured repository, including bare repositories"
                );
            }
        }
        Ok(())
    }
}
