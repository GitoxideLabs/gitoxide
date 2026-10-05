use gix_testtools::bstr::ByteSlice;

use super::{baseline, git_layouts, writable_fixture};

#[test]
fn baselines_agree_across_storage_formats() -> crate::Result {
    for name in [
        "first.id",
        "second.id",
        "annotated.id",
        "head",
        "linked-head.id",
        "refs",
        "linked-refs",
        "topic.log",
    ] {
        let expected = baseline("files", name)?;
        for layout in ["packed", "reftable"] {
            assert_eq!(
                baseline(layout, name)?,
                expected,
                "{layout}: {name} describes the same records as loose files"
            );
        }
    }
    Ok(())
}

#[test]
fn git_reads_reference_records() -> crate::Result {
    let first_commit_id = baseline("files", "first.id")?;
    let second_commit_id = baseline("files", "second.id")?;
    let tag_id = baseline("files", "annotated.id")?;

    for layout in git_layouts() {
        let dir = writable_fixture(layout)?;
        let repo = dir.path().join("repo");
        assert_eq!(
            gix_testtools::git(&repo, "rev-parse --show-object-format")?.trim_end(),
            gix_testtools::object_hash().to_string(),
            "{layout}: the fixture uses the selected object-hash format"
        );
        assert_eq!(
            gix_testtools::git(&repo, "for-each-ref --format=%(refname)")?
                .as_bytes()
                .as_bstr(),
            baseline(layout, "refs")?.as_bstr(),
            "{layout}: Git enumerates the archived reference records"
        );
        for (name, expected) in [
            ("HEAD", &second_commit_id),
            ("refs/heads/main", &second_commit_id),
            ("refs/heads/shared", &first_commit_id),
            ("refs/tags/shared", &second_commit_id),
            ("refs/tags/annotated", &tag_id),
            ("refs/tags/annotated^{}", &first_commit_id),
            ("refs/heads/alias", &second_commit_id),
            ("refs/remotes/origin/HEAD", &second_commit_id),
            ("refs/namespaces/test/refs/heads/main", &first_commit_id),
        ] {
            assert_eq!(
                gix_testtools::git(&repo, &format!("rev-parse --verify {name}"))?
                    .as_bytes()
                    .as_bstr(),
                expected.as_bstr(),
                "{layout}: {name} retains its recorded target or peeled value"
            );
        }
        for (name, target) in [
            ("HEAD", "refs/heads/main\n"),
            ("refs/heads/alias", "refs/heads/main\n"),
            ("refs/remotes/origin/HEAD", "refs/remotes/origin/main\n"),
        ] {
            assert_eq!(
                gix_testtools::git(&repo, &format!("symbolic-ref {name}"))?,
                target,
                "{layout}: {name} remains a symbolic reference"
            );
        }
    }
    Ok(())
}

#[test]
fn git_reads_reflog_records() -> crate::Result {
    let first_commit_id = baseline("files", "first.id")?;
    let second_commit_id = baseline("files", "second.id")?;
    let expected = format!(
        "{}\tcommitter\tcommitter@example.com\tadvanced\n{}\tcommitter\tcommitter@example.com\tcreated\n",
        second_commit_id.trim_end().as_bstr(),
        first_commit_id.trim_end().as_bstr(),
    );

    for layout in git_layouts() {
        let dir = writable_fixture(layout)?;
        let repo = dir.path().join("repo");
        assert_eq!(
            gix_testtools::git(&repo, "reflog show --format='%H %gs' refs/heads/topic")?
                .as_bytes()
                .as_bstr(),
            baseline(layout, "topic.log")?.as_bstr(),
            "{layout}: the archived reflog preserves its entries and order"
        );
        assert_eq!(
            gix_testtools::git(&repo, "reflog show --format=%H%x09%gn%x09%ge%x09%gs refs/heads/topic")?,
            expected,
            "{layout}: the reflog retains object IDs, committer identities, and messages"
        );
    }
    Ok(())
}

#[test]
fn git_reads_relocated_worktree_records() -> crate::Result {
    let linked_commit_id = baseline("files", "linked-head.id")?;
    let main_commit_id = baseline("files", "second.id")?;
    let expected_refs = baseline("files", "linked-refs")?;

    for layout in git_layouts() {
        let dir = writable_fixture(layout)?;
        let linked = dir.path().join("linked");
        let git_dir = gix_testtools::git(&linked, "rev-parse --absolute-git-dir")?;
        assert_eq!(
            std::path::Path::new(git_dir.trim_end()).canonicalize()?,
            dir.path().join("repo/.git/worktrees/linked").canonicalize()?,
            "{layout}: the relocated worktree does not point back into the cached fixture"
        );
        assert_eq!(
            gix_testtools::git(&linked, "for-each-ref --format=%(refname)")?
                .as_bytes()
                .as_bstr(),
            expected_refs.as_bstr(),
            "{layout}: Git sees the archived shared and private references after relocation"
        );
        for (name, expected) in [
            ("HEAD", &linked_commit_id),
            ("refs/worktree/private", &linked_commit_id),
            ("refs/heads/main", &main_commit_id),
            ("main-worktree/refs/worktree/private", &main_commit_id),
        ] {
            assert_eq!(
                gix_testtools::git(&linked, &format!("rev-parse --verify {name}"))?
                    .as_bytes()
                    .as_bstr(),
                expected.as_bstr(),
                "{layout}: {name} selects the recorded shared or private value"
            );
        }
    }
    Ok(())
}
