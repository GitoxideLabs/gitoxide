use std::path::Path;

#[test]
fn pipeline_in_nonbare_repo_without_index() -> crate::Result {
    let repo = named_subrepo_opts("make_basic_repo.sh", "all-untracked", Default::default())?;
    let _ = repo.filter_pipeline(None).expect("does not fail due to missing index");
    Ok(())
}

use gix::bstr::ByteSlice;
use gix_filter::driver::apply::Delay;

use crate::util::{hex_to_id, named_repo, named_subrepo_opts};

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
        let out = pipe.convert_to_worktree(input.as_bytes(), "file".into(), Delay::Forbid)?;
        assert!(!out.is_changed(), "no filtering is configured, nothing changes");
    }

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

    let submodule_id = hex_to_id("a047f8183ba2bb7eb00ef89e60050c5fde740483");
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
        Some((
            hex_to_id("d95f3ad14dee633a758d2e331151e950dd13e4ed"),
            gix::object::tree::EntryKind::Blob
        ))
    );
    assert_eq!(
        take_two(pipe.worktree_file_to_object("link".into(), &index)?),
        Some((
            hex_to_id("1a010b1c0f081b2e8901d55307a15c29ff30af0e"),
            gix::object::tree::EntryKind::Link
        ))
    );
    assert_eq!(
        take_two(pipe.worktree_file_to_object("exe".into(), &index)?),
        Some((
            hex_to_id("a9128c283485202893f5af379dd9beccb6e79486"),
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
        let out = pipe.convert_to_worktree("hi\n".as_bytes(), "file".into(), Delay::Forbid)?;
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
