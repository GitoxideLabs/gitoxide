mod modules_file {
    use crate::submodule::repo;
    use gix_testtools::TestResult;

    #[test]
    fn overrides_respect_section_trust_for_every_modules_source() -> TestResult {
        use gix::submodule::config::Update;
        use gix_sec::Trust;

        let dir = gix_testtools::scripted_fixture_read_only("make_submodule_config_overrides.sh")?;
        for source in ["worktree", "index", "tree"] {
            let repo_dir = dir.join(source);
            for (options, use_overrides) in [
                (crate::restricted().with_trust(Trust::Reduced), false),
                (crate::restricted().with_trust(Trust::Full), true),
                (
                    crate::restricted()
                        .with_trust(Trust::Full)
                        .filter_config_section(|_| false),
                    false,
                ),
                (
                    crate::restricted()
                        .with_trust(Trust::Reduced)
                        .filter_config_section(|_| true),
                    true,
                ),
            ] {
                let repo = gix::open_opts(&repo_dir, options)?;
                let modules = repo.modules()?.expect("the module is available from each source");
                assert_eq!(
                    modules.update("s".into())?,
                    Some(if use_overrides {
                        Update::Command("override".into())
                    } else {
                        Update::Checkout
                    }),
                    "{source}: update commands must respect the repository's section filter"
                );
                for (field, original, overridden) in [
                    ("url", "https://safe.example/s", "https://override.example/s"),
                    ("ignore", "none", "all"),
                    ("branch", "main", "override"),
                    ("fetchRecurseSubmodules", "true", "false"),
                ] {
                    assert_eq!(
                        modules.config().string(&format!("submodule.s.{field}")),
                        Some(if use_overrides { overridden } else { original }.into()),
                        "{source}: {field} uses the same trust policy as update"
                    );
                }
            }
            let repo = gix::open_opts(
                &repo_dir,
                crate::restricted()
                    .with_trust(Trust::Reduced)
                    .config_overrides(["submodule.s.update=!trusted"]),
            )?;
            assert_eq!(
                repo.modules()?.expect("module exists").update("s".into())?,
                Some(Update::Command("trusted".into())),
                "{source}: trusted API overrides remain usable in reduced-trust repositories"
            );
        }
        Ok(())
    }

    #[test]
    fn none_if_not_present() -> TestResult {
        let repo = repo("module1")?;
        assert!(repo.open_modules_file()?.is_none(), "it's OK to not have such a file");
        assert!(
            repo.modules()?.is_none(),
            "this is reflected in the persisted version as well"
        );
        Ok(())
    }

    #[test]
    fn is_read_from_worktree() -> TestResult {
        let repo = repo("with-submodules")?;
        let modules = repo.modules()?.expect("present");
        assert_eq!(
            modules.names().collect::<Vec<_>>(),
            &["m1", "dir/m1"],
            "dir/m1 is listed only in the worktree version"
        );
        Ok(())
    }

    #[test]
    fn is_read_from_index_if_not_in_worktree() -> TestResult {
        let repo = repo("with-submodules-in-index")?;
        assert!(
            repo.open_modules_file()?.is_none(),
            ".gitmodules not available in worktree"
        );
        let modules = repo.modules()?.expect("present as loaded from index");
        assert_eq!(
            modules.names().collect::<Vec<_>>(),
            &["m1", "dir/m1"],
            "dir/m1 is listed only in the index version"
        );
        Ok(())
    }

    #[test]
    fn is_read_from_tree_if_not_in_index() -> TestResult {
        let repo = repo("with-submodules-in-tree")?;
        assert!(
            repo.open_modules_file()?.is_none(),
            ".gitmodules not available in worktree"
        );
        let modules = repo.modules()?.expect("present as loaded from tree");
        assert_eq!(
            modules.names().collect::<Vec<_>>(),
            &["m1"],
            "only m1 has been committed and thus is available in the tree at HEAD"
        );
        Ok(())
    }
}

mod submodules {
    use gix::bstr::BString;
    use gix_testtools::TestResult;

    use crate::{submodule::repo, util::hex_to_id};

    #[test]
    fn all_modules_are_active_by_default() -> TestResult {
        let repo = repo("with-submodules")?;
        let id = hex_to_id("e046f3e51d955840619fc7d01fbd9a469663de22");
        assert_eq!(
            repo.submodules()?
                .expect("submodules")
                .map(|sm| (
                    sm.name().to_owned(),
                    sm.path().expect("valid path"),
                    sm.head_id().expect("valid"),
                    sm.index_id().expect("valid"),
                    sm.is_active().expect("no config error")
                ))
                .collect::<Vec<_>>(),
            [
                ("m1", "m1", Some(id), Some(id), true),
                ("dir/m1", "dir/m1", None, Some(id), true)
            ]
            .into_iter()
            .map(|(name, path, head_id, index_id, is_active)| (
                BString::from(name),
                BString::from(path),
                head_id,
                index_id,
                is_active
            ))
            .collect::<Vec<_>>()
        );

        Ok(())
    }
}

#[cfg(unix)]
mod advisory {
    use gix_testtools::TestResult;
    use gix_testtools::tempfile;

    /// Regression test for GHSA-pg4w-g64p-qwhj: a symlinked worktree `.gitmodules` must not allow
    /// attacker-controlled bytes outside the repository to define submodule configuration.
    #[test]
    fn symlinked_gitmodules_are_rejected() -> TestResult {
        use std::os::unix::fs as unix_fs;

        let temp = tempfile::tempdir()?;
        let repo_dir = temp.path().join("repo");
        let outside_modules = temp.path().join("outside.gitmodules");

        crate::init_repo_isolated(&repo_dir, gix::create::Kind::WithWorktree)?;
        std::fs::write(
            &outside_modules,
            "[submodule \"escaped\"]\n\tpath = escaped\n\turl = https://example.invalid/escaped\n",
        )?;
        unix_fs::symlink(&outside_modules, repo_dir.join(".gitmodules"))?;
        std::fs::create_dir(repo_dir.join("escaped"))?;

        for target_exists in [true, false] {
            if !target_exists {
                std::fs::remove_file(&outside_modules)?;
            }
            let repo = gix::open_opts(&repo_dir, crate::restricted())?;
            assert!(
                repo.open_modules_file()?.is_none(),
                "live and dangling worktree `.gitmodules` symlinks must both be ignored"
            );
            assert!(
                repo.submodules()?.is_none(),
                "attacker-controlled `.gitmodules` content outside the repository should not become active submodule configuration"
            );
        }
        Ok(())
    }

    #[test]
    fn symlinked_gitmodules_fall_back_to_index_and_tree() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo_dir = temp.path().join("repo");
        crate::init_repo_isolated(&repo_dir, gix::create::Kind::WithWorktree)?;
        let modules_path = repo_dir.join(".gitmodules");
        std::fs::write(
            &modules_path,
            "[submodule \"safe\"]\npath = safe\nurl = https://example.invalid/safe\n",
        )?;
        gix_testtools::git(&repo_dir, "add .gitmodules")?;
        gix_testtools::git(&repo_dir, "commit -m modules")?;
        std::fs::remove_file(&modules_path)?;
        let outside_modules = temp.path().join("outside.gitmodules");
        std::fs::write(
            &outside_modules,
            "[submodule \"escaped\"]\npath = escaped\nurl = https://example.invalid/escaped\n",
        )?;
        std::os::unix::fs::symlink(&outside_modules, &modules_path)?;

        for source in ["index", "tree"] {
            if source == "tree" {
                std::fs::remove_file(repo_dir.join(".git/index"))?;
            }
            let repo = gix::open_opts(&repo_dir, crate::restricted())?;
            assert!(repo.open_modules_file()?.is_none(), "the worktree symlink is ignored");
            let modules = repo.modules()?.expect("tracked `.gitmodules` remains available");
            assert_eq!(
                modules.names().collect::<Vec<_>>(),
                &["safe"],
                "{source}: only tracked configuration may define submodules"
            );
        }
        Ok(())
    }
}
