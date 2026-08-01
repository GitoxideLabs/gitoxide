use std::path::Path;

#[test]
fn pipeline_in_nonbare_repo_without_index() -> crate::Result {
    let repo = named_subrepo_opts("make_basic_repo.sh", "all-untracked", Default::default())?;
    let _ = repo.filter_pipeline(None).expect("does not fail due to missing index");
    Ok(())
}

use gix::bstr::ByteSlice;
use gix_filter::pipeline::convert::to_worktree;

use super::blob_id;
use crate::util::{named_repo, named_subrepo_opts};

#[test]
fn pipeline_in_repo_without_special_options() -> crate::Result {
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
fn repo_local_filter_driver_configuration_overrides_global_configuration() -> crate::Result {
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
#[cfg(unix)]
fn pipeline_worktree_file_to_object() -> crate::Result {
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
fn worktree_file_to_object_opens_submodules_after_path_options_were_consumed() -> crate::Result {
    let repo = named_repo("repo_with_untracked_files.sh")?;
    let submodule = gix::open_opts(
        repo.workdir().expect("non-bare").join("submodule"),
        gix::open::Options::isolated(),
    )?;
    let checked_out_head = submodule.head_id()?;
    let mut repo = gix::open_opts(repo.git_dir(), gix::open::Options::isolated().open_path_as_is(true))?;

    fn submodule_entry(repo: &gix::Repository) -> crate::Result<Option<(gix::ObjectId, gix::object::tree::EntryKind)>> {
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
fn pipeline_with_autocrlf() -> crate::Result {
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
