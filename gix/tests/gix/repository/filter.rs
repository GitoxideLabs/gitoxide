use gix_testtools::{Result, TestResult};
use std::{path::Path, sync::atomic::AtomicBool};

#[test]
fn pipeline_in_nonbare_repo_without_index() -> TestResult {
    let repo = named_subrepo_opts("make_basic_repo.sh", "all-untracked", Default::default())?;
    let _ = repo.filter_pipeline(None)?;
    Ok(())
}

use gix::bstr::ByteSlice;
use gix_filter::pipeline::convert::to_worktree;

use super::blob_id;
use crate::util::{named_repo, named_subrepo_opts};

#[test]
fn pipeline_in_repo_without_special_options() -> TestResult {
    let repo = named_repo("make_basic_repo.sh")?;
    let (mut pipe, index) = repo.filter_pipeline(None)?;

    let input = "hi\n";
    {
        let out = pipe.convert_to_git(input.as_bytes(), Path::new("file"), &index)?;
        assert!(!out.is_changed(), "no filtering is configured, nothing changes");
    }

    {
        let out = pipe.convert_to_worktree(input.as_bytes(), "file".into(), to_worktree::Options::default())?;
        assert!(!out.is_changed(), "no filtering is configured, nothing changes");
    }

    Ok(())
}

#[test]
fn repo_local_filter_driver_configuration_overrides_global_configuration() -> TestResult {
    let mut repo = named_repo("make_basic_repo.sh")?;
    repo.config_snapshot_mut()
        .append_config(
            ["filter.lfs.clean=global-clean", "filter.lfs.smudge=global-smudge"],
            gix_config::Source::User,
        )?
        .append_config(["filter.lfs.clean=local-clean"], gix_config::Source::Local)?;

    let drivers = gix::filter::Pipeline::options(&repo)?.drivers;
    assert_eq!(drivers.len(), 1, "configuration for the same driver is merged");
    assert_eq!(
        drivers[0].clean.as_deref().map(Vec::as_slice),
        Some(b"local-clean".as_slice()),
        "repository-local properties take precedence"
    );
    assert_eq!(
        drivers[0].smudge.as_deref().map(Vec::as_slice),
        Some(b"global-smudge".as_slice()),
        "properties not overridden locally are inherited"
    );
    Ok(())
}

#[test]
#[cfg(all(feature = "blob-diff", feature = "merge"))]
fn legacy_driver_names_are_canonical() -> TestResult {
    let repo = named_repo("make_legacy_config_consumers.sh")?;
    let filters = gix::filter::Pipeline::options(&repo)?.drivers;
    assert_eq!(
        filters.len(),
        2,
        r#"[filter.Legacy] folds to legacy and merges with quoted "legacy"; quoted "Legacy" stays distinct"#
    );
    let filter = filters
        .iter()
        .find(|driver| driver.name == "legacy")
        .expect(r#"[filter.Legacy] and [filter "legacy"] both identify the lowercase legacy filter"#);
    assert_eq!(
        filter.clean.as_deref().map(Vec::as_slice),
        Some(b"legacy-clean".as_slice()),
        r#"[filter.Legacy] supplies clean for legacy; quoted "Legacy" is distinct and cannot override it"#
    );
    assert_eq!(
        filter.smudge.as_deref().map(Vec::as_slice),
        Some(b"quoted-smudge".as_slice()),
        r#"[filter "legacy"] supplies smudge for the same driver as the lowercased [filter.Legacy]"#
    );

    let diff = repo.diff_resource_cache(Default::default(), Default::default())?;
    let diffs = diff.filter.drivers();
    assert_eq!(
        diffs.len(),
        2,
        r#"[diff.Legacy] folds to legacy and merges with quoted "legacy"; quoted "Legacy" stays distinct"#
    );
    let diff = diffs
        .iter()
        .find(|driver| driver.name == "legacy")
        .expect(r#"[diff.Legacy] and [diff "legacy"] both identify the lowercase legacy diff driver"#);
    assert_eq!(
        diff.command.as_ref().map(|value| value.as_bstr()),
        Some("legacy-diff".into()),
        r#"[diff.Legacy] supplies command for legacy; quoted "Legacy" configures a separate driver"#
    );
    assert_eq!(
        diff.binary_to_text_command.as_ref().map(|value| value.as_bstr()),
        Some("quoted-textconv".into()),
        r#"[diff "legacy"] supplies textconv for the same driver as the lowercased [diff.Legacy]"#
    );

    let merge = repo.merge_resource_cache(Default::default())?;
    let merges = merge.drivers();
    assert_eq!(
        merges.len(),
        2,
        r#"[merge.Legacy] folds to legacy and merges with quoted "legacy"; quoted "Legacy" stays distinct"#
    );
    let merge = merges
        .iter()
        .find(|driver| driver.name == "legacy")
        .expect(r#"[merge.Legacy] and [merge "legacy"] both identify the lowercase legacy merge driver"#);
    assert_eq!(
        merge.command, "legacy-merge",
        r#"[merge.Legacy] supplies driver for legacy; quoted "Legacy" is distinct and cannot override it"#
    );
    assert_eq!(
        merge.recursive.as_ref().map(|value| value.as_bstr()),
        Some("binary".into()),
        r#"[merge "legacy"] supplies recursive for the same driver as the lowercased [merge.Legacy]"#
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn pipeline_worktree_file_to_object() -> TestResult {
    let repo = named_repo("repo_with_untracked_files.sh")?;
    let work_dir = repo.workdir().expect("non-bare");
    let (mut pipe, index) = repo.filter_pipeline(None)?;
    fn take_two<A, B, C>(t: Option<(A, B, C)>) -> Option<(A, B)> {
        t.map(|t| (t.0, t.1))
    }

    let submodule_id = gix::open_opts(work_dir.join("embedded-repository"), gix::open::Options::isolated())?
        .head_id()?
        .detach();
    assert_eq!(
        take_two(pipe.worktree_file_to_object("embedded-repository".into(), &index)?),
        Some((submodule_id, gix::object::tree::EntryKind::Commit))
    );
    assert_eq!(
        take_two(pipe.worktree_file_to_object("submodule".into(), &index)?),
        Some((submodule_id, gix::object::tree::EntryKind::Commit))
    );
    assert_eq!(
        take_two(pipe.worktree_file_to_object("uninitialized-embedded-repository".into(), &index)?),
        None,
        "repositories that don't have HEAD pointing to an ID yet are ignored"
    );
    assert!(work_dir.join("empty-dir").is_dir());
    assert_eq!(
        take_two(pipe.worktree_file_to_object("empty-dir".into(), &index)?),
        None,
        "directories that aren't even repos are also ignored"
    );
    assert_eq!(
        take_two(pipe.worktree_file_to_object("file".into(), &index)?),
        Some((blob_id(&repo, b"content\n"), gix::object::tree::EntryKind::Blob))
    );
    assert_eq!(
        take_two(pipe.worktree_file_to_object("link".into(), &index)?),
        Some((blob_id(&repo, b"file"), gix::object::tree::EntryKind::Link))
    );
    assert_eq!(
        take_two(pipe.worktree_file_to_object("exe".into(), &index)?),
        Some((
            blob_id(&repo, b"binary\n"),
            gix::object::tree::EntryKind::BlobExecutable
        ))
    );
    assert_eq!(
        take_two(pipe.worktree_file_to_object("missing".into(), &index)?),
        None,
        "Missing files are specifically typed and no error"
    );
    assert!(work_dir.join("fifo").exists(), "there is a fifo");
    assert_eq!(
        take_two(pipe.worktree_file_to_object("fifo".into(), &index)?),
        None,
        "untrackable entries are just ignored as if they didn't exist"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn worktree_file_to_object_rejects_parent_symlinks() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable_with_args(
        "make_worktree_file_ids.sh",
        ["sha1"],
        gix_testtools::Creation::Execute,
    )?;
    let repo = gix::open_opts(fixture.path().join("repo"), crate::restricted())?;
    let (mut pipe, index) = repo.filter_pipeline(None)?;
    assert!(
        pipe.worktree_file_to_object("parent-link/file".into(), &index).is_err(),
        "reading a worktree entry must not follow a parent symlink outside the worktree"
    );
    Ok(())
}

#[test]
fn worktree_file_to_id_matches_git_without_writing_objects() -> TestResult {
    for hash in gix::hash::Kind::all() {
        let fixture =
            gix_testtools::scripted_fixture_read_only_with_args("make_worktree_file_ids.sh", [hash.to_string()])?;
        let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
        let (mut pipe, index) = repo.filter_pipeline(None)?;
        for record in std::fs::read(fixture.join("expected-entries"))?.split(|byte| *byte == 0) {
            if record.is_empty() {
                continue;
            }
            let (header, path) = record.split_once_str(b"\t").expect("Git separates metadata from paths");
            let mut header = header.split(|byte| *byte == b' ');
            let expected_kind = gix::objs::tree::EntryMode::from_bytes(header.next().expect("mode"))
                .expect("Git emits a valid mode")
                .kind();
            let expected_id = gix::ObjectId::from_hex(header.next().expect("object ID"))?;
            assert!(
                !repo.has_object(expected_id),
                "the fixture keeps oracle objects separate"
            );
            let (actual_id, actual_kind, metadata) = pipe
                .worktree_file_to_id(path.as_bstr(), &index, &AtomicBool::new(false))?
                .expect("the fixture entry is trackable");
            assert_eq!(
                (actual_id, actual_kind),
                (expected_id, expected_kind),
                "Git agrees for {path:?}"
            );
            assert_eq!(
                metadata.len(),
                repo.workdir()
                    .expect("non-bare")
                    .join(gix::path::from_bstr(path.as_bstr())?)
                    .symlink_metadata()?
                    .len(),
                "returned metadata describes the worktree representation"
            );
            assert!(!repo.has_object(actual_id), "hashing does not persist objects");
        }
    }
    Ok(())
}

#[test]
fn worktree_file_to_id_preserves_missing_paths_filters_and_cancellation() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only_with_args("make_worktree_file_ids.sh", ["sha1"])?;
    let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
    let (mut pipe, index) = repo.filter_pipeline(None)?;
    let interrupt = AtomicBool::new(false);
    for path in ["missing", "absent/parent/file", "plain/child/grandchild"] {
        assert!(
            pipe.worktree_file_to_id(path.into(), &index, &interrupt)?.is_none(),
            "missing entries and blocked paths retain the object-writing API's absent outcome"
        );
    }
    assert!(
        pipe.worktree_file_to_id("failure".into(), &index, &interrupt).is_err(),
        "required filter failures remain errors"
    );
    assert!(
        pipe.worktree_file_to_id("../outside/file".into(), &index, &interrupt)
            .is_err(),
        "paths cannot escape the worktree root"
    );
    #[cfg(unix)]
    assert!(
        pipe.worktree_file_to_id("parent-link/file".into(), &index, &interrupt)
            .is_err(),
        "hashing shares the writer's parent-symlink check"
    );
    assert!(
        pipe.worktree_file_to_id("missing".into(), &index, &AtomicBool::new(true))
            .expect_err("interruption precedes path reads")
            .is_cancelled(),
        "the caller can distinguish interruption from a missing path"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn worktree_file_to_object_opens_submodules_after_path_options_were_consumed() -> TestResult {
    let repo = named_repo("repo_with_untracked_files.sh")?;
    let submodule = gix::open_opts(
        repo.workdir().expect("non-bare").join("submodule"),
        gix::open::Options::isolated(),
    )?;
    let checked_out_head = submodule.head_id()?;
    let mut repo = gix::open_opts(repo.git_dir(), gix::open::Options::isolated().open_path_as_is(true))?;

    fn submodule_entry(repo: &gix::Repository) -> Result<Option<(gix::ObjectId, gix::object::tree::EntryKind)>> {
        let (mut pipe, index) = repo.filter_pipeline(None)?;
        Ok(pipe
            .worktree_file_to_object("submodule".into(), &index)?
            .map(|(id, kind, _)| (id, kind)))
    }

    let (id, kind) = submodule_entry(&repo)?.expect("the submodule can be opened");
    assert_eq!(
        id, checked_out_head,
        "the ID comes from the opened submodule's HEAD, not the superproject's index"
    );
    assert_eq!(
        kind,
        gix::object::tree::EntryKind::Commit,
        "an option used to open the parent repository must not affect opening its submodule"
    );

    repo.reload()?;
    let (id, kind) = submodule_entry(&repo)?.expect("the submodule can still be opened after reload");
    assert_eq!(
        id, checked_out_head,
        "reload-only options must not prevent reading the opened submodule's HEAD"
    );
    assert_eq!(
        kind,
        gix::object::tree::EntryKind::Commit,
        "reload-only options must not affect opening a submodule afterward"
    );
    Ok(())
}

#[test]
fn pipeline_with_autocrlf() -> TestResult {
    let repo = named_repo("make_config_repo.sh")?;
    let (mut pipe, index) = repo.filter_pipeline(None)?;

    let input = "hi\r\n";
    {
        let out = pipe.convert_to_git(input.as_bytes(), Path::new("file"), &index)?;
        assert!(out.is_changed(), "filtering is configured so a change should happen");
        assert_eq!(
            out.as_bytes()
                .expect("a buffer is needed for eol conversions")
                .as_bstr(),
            "hi\n"
        );
    }

    {
        let out = pipe.convert_to_worktree("hi\n".as_bytes(), "file".into(), to_worktree::Options::default())?;
        assert_eq!(
            out.as_bytes()
                .expect("a buffer is needed for eol conversions")
                .as_bstr(),
            input,
            "autocrlf converts text LF to CRLF in the worktree"
        );
    }
    Ok(())
}
