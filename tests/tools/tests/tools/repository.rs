mod snapshot {
    use std::io::Write;

    use bstr::ByteSlice;
    use gix_testtools::repository::{Head, WorktreeEntryKind};

    #[test]
    fn captures_refs_commits_index_tree_and_exact_worktree() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("make_repository_state.sh")?;
        let state = gix_testtools::repository::snapshot(fixture.path())?;

        assert!(
            matches!(state.head, Head::Symbolic { .. }),
            "HEAD attachment is retained"
        );
        assert_eq!(state.commits.len(), 2, "all commits reachable from refs are captured");
        assert!(
            state
                .references
                .iter()
                .any(|reference| reference.name == "refs/heads/side"),
            "non-HEAD refs are captured"
        );
        assert!(state.index_tree.is_none(), "a conflicted index has no computed tree");
        assert!(
            state
                .index
                .iter()
                .any(|entry| entry.path == "staged" && entry.stage == 0),
            "ordinary staged entries are represented"
        );
        assert_eq!(
            state
                .index
                .iter()
                .filter(|entry| entry.path == "conflicted")
                .map(|entry| entry.stage)
                .collect::<Vec<_>>(),
            [1, 2, 3],
            "all conflict stages are represented"
        );
        assert!(
            state.worktree.iter().any(|entry| {
                entry.path == std::path::Path::new("untracked")
                    && matches!(&entry.kind, WorktreeEntryKind::File(data) if data == b"untracked\n")
            }),
            "untracked worktree contents are captured"
        );
        assert!(
            state.worktree.iter().any(|entry| {
                entry.path == std::path::Path::new("nested/deep/tracked")
                    && matches!(&entry.kind, WorktreeEntryKind::File(data) if data == b"nested\n")
            }),
            "nested worktree contents are captured"
        );

        let again = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(state, again, "taking a snapshot has no observable side effects");

        gix_testtools::git(fixture.path(), "checkout --theirs -- conflicted")?;
        gix_testtools::git(fixture.path(), "add conflicted")?;
        assert!(
            gix_testtools::repository::snapshot(fixture.path())?
                .index_tree
                .is_some(),
            "a resolved index has a computed tree"
        );
        Ok(())
    }

    #[test]
    fn local_config_paths_are_normalized_and_portable_values_are_stabilized() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("make_repository_state.sh")?;
        let config_path = fixture.path().join(".git/config");
        let included_config = fixture.path().join(".git/included-config");
        let sibling_source = fixture
            .path()
            .parent()
            .expect("fixture repositories have a parent directory")
            .join("source");
        let outside_repository = std::env::current_dir()?.join("absolute-generated-path");
        std::fs::write(&included_config, b"[included]\n\tvalue = true\n")?;
        let mut config = std::fs::OpenOptions::new().append(true).open(&config_path)?;
        let outside_repository_for_config =
            gix_path::to_unix_separators_on_windows(gix_path::into_bstr(&outside_repository));
        let sibling_source_for_config = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(&sibling_source));
        let included_config_for_config = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(&included_config));
        write!(
            config,
            "\n# retained comment\n[snapshot]\n\tstable = keep\n\
             [core]\n\tignoreCase = true\n\tprecomposeUnicode = true\n\
             [remote \"unstable\"]\n\turl = {outside_repository_for_config}\n\
             [submodule \"sibling\"]\n\turl = {sibling_source_for_config}\n\
             [include]\n\tpath = {included_config_for_config}\n"
        )?;
        drop(config);
        let config_on_disk = std::fs::read(&config_path)?;

        let state = gix_testtools::repository::snapshot(fixture.path())?;
        let config = state.config.as_bstr();
        assert!(
            config.contains_str("path = <normalized>/included-config"),
            "ordinary snapshots retain repository-relative path components"
        );
        assert!(
            config.contains_str("url = <normalized>/../source"),
            "sibling locations retain their relationship to the repository"
        );
        assert!(
            config.contains_str("url = <normalized>\n"),
            "locations outside the repository are fully normalized"
        );
        assert_eq!(
            std::fs::read(&config_path)?,
            config_on_disk,
            "normalizing the snapshot doesn't alter the repository config"
        );
        for generated_key in ["ignoreCase", "precomposeUnicode"] {
            assert!(
                !config.contains_str(generated_key),
                "ordinary snapshots omit platform-generated key {generated_key:?}"
            );
        }

        let portable = gix_testtools::repository::snapshot_portable(fixture.path())?;
        let config = portable.config.as_bstr();
        assert!(config.contains_str("# retained comment"), "comments are retained");
        assert!(
            config.contains_str("stable = keep"),
            "repository-specific values are retained"
        );
        assert!(
            config.contains_str("path = <normalized>/included-config"),
            "portable snapshots retain repository-relative path components"
        );
        assert!(
            config.contains_str("url = <normalized>/../source"),
            "portable snapshots retain sibling repository relationships"
        );
        assert!(
            config.contains_str("url = <normalized>\n"),
            "generated paths are normalized"
        );
        assert!(
            !config.contains_str("absolute-generated-path"),
            "generated paths don't leak"
        );
        for generated_key in [
            "repositoryformatversion",
            "filemode",
            "logallrefupdates",
            "ignoreCase",
            "precomposeUnicode",
        ] {
            assert!(
                !config.contains_str(generated_key),
                "Git- or filesystem-generated key {generated_key:?} is omitted"
            );
        }
        Ok(())
    }

    #[test]
    fn portable_snapshots_alias_annotated_tags() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("make_repository_state.sh")?;
        gix_testtools::git(fixture.path(), "tag -a annotated -m annotated")?;
        let tag_id = gix_testtools::git(fixture.path(), "rev-parse refs/tags/annotated")?
            .trim()
            .to_owned();

        let snapshot = gix_testtools::repository::snapshot(fixture.path())?.to_string();
        assert!(
            snapshot.contains("refs/tags/annotated = O0"),
            "otherwise uncategorized ref targets receive a stable alias"
        );
        assert!(
            snapshot.contains(&format!("O0 = {tag_id}")),
            "ordinary snapshots retain the aliased object's ID"
        );

        let portable = gix_testtools::repository::snapshot_portable(fixture.path())?.to_string();
        assert!(
            portable.contains("refs/tags/annotated = O0"),
            "portable snapshots retain the annotated tag through its alias"
        );
        assert!(
            !portable.contains(&tag_id),
            "portable snapshots don't expose the annotated tag's object ID"
        );
        Ok(())
    }

    #[test]
    fn broken_references_are_ignored() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("make_repository_state.sh")?;
        let refs_dir = fixture.path().join(".git/refs");
        std::fs::write(refs_dir.join("broken"), b"notahexsha\n")?;
        std::fs::write(refs_dir.join("broken-symbolic"), b"ref: refs/broken\n")?;
        std::fs::write(refs_dir.join("broken-cycle-a"), b"ref: refs/broken-cycle-b\n")?;
        std::fs::write(refs_dir.join("broken-cycle-b"), b"ref: refs/broken-cycle-a\n")?;

        let state = gix_testtools::repository::snapshot(fixture.path())?;
        assert!(
            state
                .references
                .iter()
                .all(|reference| !reference.name.starts_with(b"refs/broken")),
            "malformed references, symbolic references resolving through them, and symbolic cycles are omitted"
        );
        Ok(())
    }

    #[test]
    fn repository_variants_are_captured_portably() -> gix_testtools::Result {
        // Sparse indexes were introduced in Git 2.34 and are one of the repository forms created by this fixture.
        let Some(fixture) = gix_testtools::scripted_fixture_writable_with_args_with_git_version(
            "make_repository_variants.sh",
            None::<String>,
            gix_testtools::Creation::Execute,
            |version| version >= (2, 34, 0),
        )?
        else {
            return Ok(());
        };
        let main = gix_testtools::repository::snapshot_portable(fixture.path().join("main"))?;
        let linked = gix_testtools::repository::snapshot_portable(fixture.path().join("linked"))?;
        let bare = gix_testtools::repository::snapshot_portable(fixture.path().join("bare.git"))?;
        let unborn = gix_testtools::repository::snapshot_portable(fixture.path().join("unborn"))?;

        let shallow = gix_testtools::repository::snapshot_portable(fixture.path().join("shallow"))?;
        assert_eq!(shallow.commits.len(), 1, "history stops at the shallow boundary");

        let split = gix_testtools::repository::snapshot_portable(fixture.path().join("split.git"))?;
        assert!(
            split
                .worktree
                .iter()
                .any(|entry| entry.path == std::path::Path::new("tracked")),
            "core.worktree is resolved relative to the Git directory"
        );

        let configured = gix_testtools::repository::snapshot_portable(fixture.path().join("configured-original"))?;
        assert!(
            configured
                .worktree
                .iter()
                .any(|entry| entry.path == std::path::Path::new("configured-only")),
            "core.worktree overrides a worktree inferred during discovery"
        );
        let configured_bare = gix_testtools::repository::snapshot_portable(fixture.path().join("configured-bare"))?;
        assert!(
            configured_bare.worktree.is_empty(),
            "core.bare overrides a worktree inferred during discovery"
        );

        let sparse = gix_testtools::repository::snapshot_portable(fixture.path().join("sparse"))?;
        assert!(
            sparse.index.iter().any(|entry| entry.path == "hidden/deep/tracked"),
            "sparse directory entries are expanded to the paths represented by Git"
        );
        assert!(
            sparse.index.iter().all(|entry| entry.mode != 0o040000),
            "the rendered index contains no sparse-directory placeholders"
        );

        let replaced = gix_testtools::repository::snapshot_portable(fixture.path().join("replaced"))?;
        let head_id = match replaced.head {
            Head::Symbolic { id, .. } | Head::Detached(id) => id,
            Head::Unborn(_) => return Err("the replacement-ref fixture unexpectedly has an unborn HEAD".into()),
        };
        assert_eq!(
            replaced.commits.len(),
            2,
            "legacy grafts do not truncate the raw commit graph"
        );
        assert!(
            replaced
                .commits
                .iter()
                .find(|commit| commit.id == head_id)
                .is_some_and(|commit| commit.data.ends_with(b"\ngitlinks\n")),
            "snapshots contain raw commits without applying replacement refs"
        );

        insta::assert_snapshot!(format!("[main]\n{main}\n[linked]\n{linked}\n[bare]\n{bare}\n[unborn]\n{unborn}"), @r#"
        [main]
        HEAD refs/heads/main -> C1

        [config]
        [core]
            bare = false
        [submodule "submodule"]
            url = <normalized>/../source
            active = true

        [refs]
        refs/heads/linked = C1
        refs/heads/main = C1

        [commits]
        C0
          tree T0
          author author <author@example.com> 946684800 +0000
          committer committer <committer@example.com> 946771200 +0000

          base

        C1
          tree T1
          parent C0
          author author <author@example.com> 946684800 +0000
          committer committer <committer@example.com> 946771200 +0000

          gitlinks

        [index]
        tree = T1
        100644 B0 stage=0 ".gitmodules"
        160000 S0 stage=0 "embedded"
        160000 S1 stage=0 "submodule"
        100644 B1 stage=0 "tracked"

        [worktree]
        - file ".gitmodules" = "[submodule \"submodule\"]\n\tpath = submodule\n\turl = ../source\n"
        - dir  "embedded"
        - file "embedded/tracked" = "embedded\n"
        - dir  "submodule"
        - file "submodule/tracked" = "submodule-source\n"
        - file "tracked" = "superproject\n"
        - dir  "untracked-repository"
        - file "untracked-repository/tracked" = "untracked-repository\n"

        [linked]
        HEAD refs/heads/linked -> C1

        [config]
        [core]
            bare = false
        [submodule "submodule"]
            url = <normalized>/../source
            active = true

        [refs]
        refs/heads/linked = C1
        refs/heads/main = C1

        [commits]
        C0
          tree T0
          author author <author@example.com> 946684800 +0000
          committer committer <committer@example.com> 946771200 +0000

          base

        C1
          tree T1
          parent C0
          author author <author@example.com> 946684800 +0000
          committer committer <committer@example.com> 946771200 +0000

          gitlinks

        [index]
        tree = T1
        100644 B0 stage=0 ".gitmodules"
        160000 S0 stage=0 "embedded"
        160000 S1 stage=0 "submodule"
        100644 B1 stage=0 "tracked"

        [worktree]
        - file ".gitmodules" = "[submodule \"submodule\"]\n\tpath = submodule\n\turl = ../source\n"
        - dir  "embedded"
        - dir  "submodule"
        - file "tracked" = "superproject\n"

        [bare]
        HEAD refs/heads/main -> C1

        [config]
        [core]
            bare = true
        [remote "origin"]
            url = <normalized>/../main

        [refs]
        refs/heads/linked = C1
        refs/heads/main = C1

        [commits]
        C0
          tree T0
          author author <author@example.com> 946684800 +0000
          committer committer <committer@example.com> 946771200 +0000

          base

        C1
          tree T1
          parent C0
          author author <author@example.com> 946684800 +0000
          committer committer <committer@example.com> 946771200 +0000

          gitlinks

        [index]
        tree = T2

        [worktree]

        [unborn]
        HEAD unborn refs/heads/main

        [config]
        [core]
            bare = false

        [refs]

        [commits]
        [index]
        tree = T0

        [worktree]
        "#);
        Ok(())
    }

    #[test]
    fn state_is_a_stable_visual_snapshot() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("make_repository_state.sh")?;
        #[cfg(unix)]
        {
            let snapshot_name = format!("state_is_a_stable_visual_snapshot_{}", gix_testtools::object_hash());
            insta::assert_snapshot!(snapshot_name, gix_testtools::repository::snapshot(fixture.path())?);
        }
        insta::assert_snapshot!(gix_testtools::repository::snapshot_portable(fixture.path())?, @r#"
        HEAD refs/heads/main -> C0

        [config]
        [core]
            bare = false

        [refs]
        refs/heads/main = C0
        refs/heads/side = C1

        [commits]
        C0
          tree T0
          author author <author@example.com> 946684800 +0000
          committer committer <committer@example.com> 946684800 +0000

          base

        C1
          tree T0
          parent C0
          author author <author@example.com> 946684800 +0000
          committer committer <committer@example.com> 946771200 +0000

          detached

        [index]
        tree = conflicted
        100644 B0 stage=1 "conflicted"
        100644 B1 stage=2 "conflicted"
        100644 B2 stage=3 "conflicted"
        100644 B3 stage=0 "nested/deep/tracked"
        100644 B4 stage=0 "staged"
        100644 B5 stage=0 "tracked"

        [worktree]
        - dir  "nested"
        - dir  "nested/deep"
        - file "nested/deep/tracked" = "nested\n"
        - file "staged" = "staged\n"
        - file "tracked" = "base\n"
        - file "untracked" = "untracked\n"
        "#);
        Ok(())
    }
}
