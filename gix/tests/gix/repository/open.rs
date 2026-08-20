use std::{borrow::Cow, error::Error};

use gix::bstr::BString;
use gix::config::tree::Key;

use crate::util::named_subrepo_opts;

#[test]
fn open_permissions_is_isolated() {
    assert!(gix::open::Permissions::isolated().is_isolated());
    assert!(!gix::open::Permissions::all().is_isolated());
}

#[test]
#[serial_test::serial]
fn discover_with_git_dir_environment_override_uses_it_and_sets_trust() -> crate::Result {
    let fallback = gix_testtools::tempfile::TempDir::new()?;
    gix::init(fallback.path())?;
    let overridden = gix_testtools::tempfile::TempDir::new()?;
    let overridden = gix::init(overridden.path())?;
    let _env = gix_testtools::Env::new()
        .unset("GIT_WORK_TREE")
        .set("GIT_DIR", overridden.git_dir().to_string_lossy().into_owned());

    let repo = gix::ThreadSafeRepository::discover_with_environment_overrides_opts(
        fallback.path(),
        Default::default(),
        gix_sec::trust::Mapping {
            full: crate::restricted(),
            reduced: crate::restricted(),
        },
    )?;

    assert_eq!(
        repo.git_dir(),
        overridden.git_dir(),
        "the git-dir from the environment replaces the fallback and opens without panicking on missing trust"
    );
    Ok(())
}

#[test]
fn core_worktree_cli_override_does_not_override_bare() -> crate::Result {
    let fixture = gix_testtools::scripted_fixture_read_only("make_config_repos.sh")?;
    let worktree = gix_testtools::tempfile::TempDir::new()?;
    let repo = gix::open_opts(
        fixture.join("bare-repo"),
        gix::open::Options::isolated().cli_overrides([format!("core.worktree={}", worktree.path().display())]),
    )?;

    assert_eq!(
        repo.workdir(),
        None,
        "core.worktree from -c does not override core.bare in Git"
    );
    assert!(repo.is_bare(), "the CLI override leaves the bare repository bare");
    Ok(())
}

#[test]
fn on_root_with_decomposed_unicode() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;

    let decomposed = "a\u{308}";

    let root = tmp.path().join(decomposed);
    std::fs::create_dir(&root)?;

    let repo = gix::init(root)?;
    let precompose_unicode = repo
        .config_snapshot()
        .boolean("core.precomposeUnicode")
        .expect("created by init based on fs-capabilities");

    assert!(repo.git_dir().is_dir());
    let work_dir = repo.workdir().expect("non-bare");
    assert!(work_dir.is_dir());

    if precompose_unicode {
        assert!(
            matches!(
                gix::utils::str::precompose_path(repo.git_dir().into()),
                Cow::Borrowed(_),
            ),
            "there is no change, as the path is already precomposed"
        );
        assert!(matches!(
            gix::utils::str::precompose_path(work_dir.into()),
            Cow::Borrowed(_),
        ));
    } else {
        assert!(
            matches!(gix::utils::str::precompose_path(repo.git_dir().into()), Cow::Owned(_),),
            "this has an effect as the path isn't precomposed, a necessity on filesystems that don't fold decomposition"
        );
        assert!(matches!(
            gix::utils::str::precompose_path(work_dir.into()),
            Cow::Owned(_),
        ));
    }
    assert!(
        repo.workdir_path("").expect("non-bare").is_dir(),
        "decomposed or not, we generate a valid path given what Git would store"
    );

    Ok(())
}

#[test]
fn non_bare_reftable() -> crate::Result {
    let Some(root) = gix_testtools::scripted_fixture_read_only_with_git_version("make_reftable_repo.sh", |version| {
        version >= (2, 44, 0)
    })?
    else {
        return Ok(());
    };
    let repo = gix::open_opts(root.join("reftable-clone"), gix::open::Options::isolated())?;
    assert!(
        repo.head_id().is_err(),
        "Trying to do anything with head will fail as we don't support reftables yet"
    );
    assert!(!repo.is_bare());
    assert_eq!(repo.kind(), gix::repository::Kind::Common);
    assert_ne!(
        repo.workdir(),
        None,
        "Otherwise it can be used, but it's hard to do without refs"
    );
    Ok(())
}

#[test]
fn bare_repo_with_index() -> crate::Result {
    let repo = named_subrepo_opts(
        "make_basic_repo.sh",
        "bare-repo-with-index.git",
        gix::open::Options::isolated(),
    )?;
    assert!(
        repo.is_bare(),
        "it's properly classified as it reads the configuration (and has no worktree)"
    );
    assert_eq!(repo.kind(), gix::repository::Kind::Common);
    assert_eq!(repo.workdir(), None);
    Ok(())
}

#[test]
fn git_index_file_overrides_the_index_in_the_git_dir() -> crate::Result {
    let repository = gix_testtools::scripted_fixture_writable("make_basic_repo.sh")?;
    let index_file = repository.path().join(".git/temporary-index");
    let repo = gix::open_opts(repository.path(), gix::open::Options::isolated())?;
    assert_eq!(
        repo.index_path(),
        repo.git_dir().join("index"),
        "this repo has no override"
    );
    #[cfg(feature = "index")]
    assert_eq!(
        repo.index()?.entries().len(),
        1,
        "the regular index is distinguishable from the override"
    );

    let mut repo = gix::open_opts(
        repository.path(),
        gix::open::Options::isolated().config_overrides([format!("gitoxide.core.indexFile={}", index_file.display())]),
    )?;
    assert_eq!(repo.index_path(), index_file, "the override was picked up");

    let changed_index_file = repository.path().join(".git/changed-index");
    {
        let mut config = repo.config_snapshot_mut();
        config.set_value(
            &gix::config::tree::gitoxide::Core::INDEX_FILE,
            changed_index_file.to_string_lossy(),
        )?;
    }
    assert_eq!(
        repo.index_path(),
        index_file,
        "the index location is repository state established during opening, like the git-dir and worktree"
    );

    #[cfg(feature = "index")]
    {
        gix::index::File::from_state(gix::index::State::new(repo.object_hash()), index_file)
            .write(Default::default())?;
        assert!(
            repo.index()?.entries().is_empty(),
            "the configured index is read, and it's initially empty"
        );
    }
    Ok(())
}

#[test]
#[cfg(feature = "index")]
fn git_index_file_missing_yields_no_index() -> crate::Result {
    let repository = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;
    let index_file = repository.join(".git/missing");
    let repo = gix::open_opts(
        &repository,
        gix::open::Options::isolated().config_overrides([format!("gitoxide.core.indexFile={}", index_file.display())]),
    )?;

    assert!(
        repo.git_dir().join("index").is_file(),
        "the regular index would be found without the override"
    );
    assert!(repo.try_index()?.is_none(), "a missing override reports no index");
    assert!(
        repo.index_or_empty()?.entries().is_empty(),
        "the usual empty-index semantics apply"
    );
    Ok(())
}

#[test]
fn git_index_file_empty_is_invalid_even_with_lenient_config() -> crate::Result {
    assert!(
        gix::config::tree::gitoxide::Core::INDEX_FILE
            .validated_assignment("".into())
            .is_err(),
        "the key itself rejects empty values"
    );
    let repository = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;
    let err = gix::open_opts(
        repository,
        gix::open::Options::isolated().config_overrides(["gitoxide.core.indexFile="]),
    )
    .expect_err("an empty index path must be rejected");

    assert_eq!(
        err.source().expect("configuration error").to_string(),
        "The key \"gitoxide.core.indexFile=\" (possibly from GIT_INDEX_FILE) was invalid",
        "an empty index path is never ignored, even though configuration is lenient by default"
    );
    Ok(())
}

#[test]
#[cfg(feature = "index")]
fn git_index_file_receives_writes_while_the_git_dir_index_is_locked() -> crate::Result {
    let repository = gix_testtools::scripted_fixture_writable("make_basic_repo.sh")?;
    let index_file = repository.path().join(".git/temporary-index");
    let repo = gix::open_opts(
        repository.path(),
        gix::open::Options::isolated().config_overrides([format!("gitoxide.core.indexFile={}", index_file.display())]),
    )?;
    let git_dir_index = repo.git_dir().join("index");
    let git_dir_index_before = std::fs::read(&git_dir_index)?;

    // `git commit <paths>` holds this lock for the duration of the commit, hooks included.
    assert!(!index_file.exists());
    std::fs::write(repo.git_dir().join("index.lock"), [])?;
    let mut index = (**repo.index_or_empty()?).clone();
    index.write(Default::default())?;

    assert!(index_file.is_file(), "the write lands on the configured index");
    assert_eq!(
        std::fs::read(&git_dir_index)?,
        git_dir_index_before,
        "the regular index is left untouched"
    );
    Ok(())
}

#[test]
fn non_bare_repo_with_git_extension() -> crate::Result {
    let repo = named_subrepo_opts("make_basic_repo.sh", "repo.git", gix::open::Options::isolated())?;
    assert_eq!(repo.kind(), gix::repository::Kind::Common);
    assert!(!repo.is_bare());
    assert!(
        repo.workdir()
            .expect("non-bare repository has a worktree")
            .ends_with("repo.git"),
        "the repo.git directory itself is the worktree"
    );
    assert!(
        repo.git_dir().ends_with("repo.git/.git"),
        "the repository metadata is in repo.git/.git"
    );
    Ok(())
}

#[test]
fn non_bare_turned_bare() -> crate::Result {
    let repo = named_subrepo_opts(
        "make_worktree_repo.sh",
        "non-bare-turned-bare",
        gix::open::Options::isolated(),
    )?;
    assert!(
        repo.is_bare(),
        "the configuration dictates this, even though it looks like a main worktree"
    );
    assert_eq!(repo.kind(), gix::repository::Kind::Common);
    assert_eq!(repo.workdir(), None);
    Ok(())
}

#[test]
fn worktree_of_bare_repo() -> crate::Result {
    let repo = named_subrepo_opts(
        "make_worktree_repo.sh",
        "worktree-of-bare-repo",
        gix::open::Options::isolated(),
    )?;
    assert_ne!(
        repo.workdir(),
        None,
        "we have opened the repo through a worktree, which is never bare"
    );
    assert!(
        !repo
            .worktree()
            .expect("the worktree is available, it's linked")
            .is_main(),
        "linked worktrees can exist for any repository, even bare"
    );
    assert!(
        repo.is_bare(),
        "this repository is bare per configuration, and the worktree is linked"
    );
    assert_eq!(repo.kind(), gix::repository::Kind::LinkedWorkTree);
    Ok(())
}

#[test]
fn worktree_of_natively_bare_repo() -> crate::Result {
    let repo = named_subrepo_opts(
        "make_worktree_repo.sh",
        "worktree-of-natively-bare-repo",
        gix::open::Options::isolated(),
    )?;
    assert_ne!(
        repo.workdir(),
        None,
        "we have opened the repo through a worktree, which is never bare"
    );
    assert!(
        !repo
            .worktree()
            .expect("the worktree is available, it's linked")
            .is_main(),
        "linked worktrees can exist for any repository, even bare"
    );
    assert!(
        repo.is_bare(),
        "the shared config has core.bare=true, which a linked worktree inherits even though it has a workdir"
    );
    assert_eq!(repo.kind(), gix::repository::Kind::LinkedWorkTree);
    Ok(())
}

#[test]
fn natively_bare_repo_itself_is_common() -> crate::Result {
    let repo = named_subrepo_opts(
        "make_worktree_repo.sh",
        "natively-bare-repo",
        gix::open::Options::isolated(),
    )?;
    assert!(repo.is_bare());
    assert_eq!(repo.workdir(), None, "the bare repository itself has no worktree");
    assert_eq!(
        repo.git_dir(),
        repo.common_dir(),
        "there is no linked-worktree redirection"
    );
    assert_eq!(repo.kind(), gix::repository::Kind::Common);
    Ok(())
}

#[test]
fn non_bare_non_git_repo_without_worktree() -> crate::Result {
    let repo = named_subrepo_opts(
        "make_basic_repo.sh",
        "non-bare-without-worktree",
        gix::open::Options::isolated(),
    )?;
    assert!(!repo.is_bare());
    assert_eq!(repo.kind(), gix::repository::Kind::Common);
    assert_eq!(repo.workdir(), None, "it doesn't assume that workdir exists");

    let repo = gix::open_opts(
        repo.git_dir().join("objects").join(".."),
        gix::open::Options::isolated(),
    )?;
    assert!(!repo.is_bare());
    assert_eq!(repo.kind(), gix::repository::Kind::Common);
    assert_eq!(
        repo.workdir(),
        None,
        "it figures this out even if a non-normalized gitdir is used"
    );
    Ok(())
}

#[test]
fn none_bare_repo_without_index() -> crate::Result {
    let mut repo = named_subrepo_opts(
        "make_basic_repo.sh",
        "non-bare-repo-without-index",
        gix::open::Options::isolated(),
    )?;
    assert!(!repo.is_bare(), "worktree isn't dependent on an index file");
    assert_eq!(repo.kind(), gix::repository::Kind::Common);
    assert!(repo.worktree().is_some());
    assert_eq!(
        repo.workdir_path(BString::from("this")).map(|p| p.is_file()),
        Some(true)
    );
    #[expect(clippy::needless_borrows_for_generic_args)]
    let actual = repo.workdir_path(&BString::from("this")).map(|p| p.is_file());
    assert_eq!(actual, Some(true));
    assert!(
        repo.workdir_path("this")
            .expect("non-bare")
            .strip_prefix(repo.workdir().expect("non-bare"))
            .is_ok(),
        "this is a minimal path"
    );

    let old = repo.set_workdir(None).expect("should never fail");
    assert_eq!(
        old.as_ref().and_then(|wd| wd.file_name()?.to_str()),
        Some("non-bare-repo-without-index")
    );
    assert!(repo.workdir().is_none(), "the workdir was unset");
    assert!(repo.worktree().is_none(), "the worktree was unset");
    assert!(
        !repo.is_bare(),
        "this is based on `core.bare`, not on the lack of worktree"
    );
    assert_eq!(repo.kind(), gix::repository::Kind::Common);

    assert_eq!(
        repo.set_workdir(old.clone()).expect("does not fail as it exists"),
        None,
        "nothing was set before"
    );
    assert_eq!(repo.workdir(), old.as_deref());

    let worktree = repo.worktree().expect("should be present after setting");
    assert!(worktree.is_main(), "it's still the main worktree");
    Ok(())
}

#[test]
fn non_bare_split_worktree() -> crate::Result {
    for (name, worktree_exists) in [
        ("repo-with-worktree-in-config-unborn-no-worktreedir", false),
        ("repo-with-worktree-in-config-unborn", true),
        ("repo-with-worktree-in-config", true),
    ] {
        let repo = named_subrepo_opts("make_worktree_repo.sh", name, gix::open::Options::isolated())?;
        assert!(repo.git_dir().is_dir());
        assert!(
            !repo.is_bare(),
            "worktree is actually configured, and it's non-bare by configuration"
        );
        assert_eq!(repo.kind(), gix::repository::Kind::Common);
        assert_eq!(
            repo.workdir().expect("worktree is configured").is_dir(),
            worktree_exists
        );
    }
    Ok(())
}

#[test]
fn non_bare_split_worktree_invalid_worktree_path_boolean() -> crate::Result {
    let err = named_subrepo_opts(
        "make_worktree_repo.sh",
        "repo-with-worktree-in-config-unborn-worktreedir-missing-value",
        gix::open::Options::isolated().strict_config(true),
    )
    .unwrap_err();
    assert_eq!(
        err.source().expect("present").to_string(),
        "The key \"core.worktree\" (possibly from GIT_WORK_TREE) was invalid",
        "in strict mode, we fail just like git does"
    );
    Ok(())
}

#[test]
fn non_bare_split_worktree_invalid_worktree_path_empty() -> crate::Result {
    // "repo-with-worktree-in-config-unborn-worktreedir-missing-value",
    let err = named_subrepo_opts(
        "make_worktree_repo.sh",
        "repo-with-worktree-in-config-unborn-empty-worktreedir",
        gix::open::Options::isolated(),
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            gix::open::Error::Config(gix::config::Error::PathInterpolation { .. })
        ),
        "DEVIATION: could not read path at core.worktree as empty is always invalid, git tries to use an empty path, even though it's better to reject it"
    );
    Ok(())
}

#[test]
fn bare_with_worktree_is_still_bare() -> crate::Result {
    let repo = named_subrepo_opts("make_config_repos.sh", "bare-link", gix::open::Options::isolated())?;
    assert!(
        repo.is_bare(),
        "the configuration file states that it's bare and we respect that"
    );
    assert_eq!(repo.kind(), gix::repository::Kind::Common);
    assert_eq!(
        repo.workdir(),
        None,
        "we aren't grabby and don't provide a worktree then"
    );
    assert!(repo.worktree().is_none(), "the same here: don't recognise a worktree");
    Ok(())
}

mod missing_config_file {

    use crate::util::named_subrepo_opts;

    #[test]
    fn bare() -> crate::Result {
        let repo = named_subrepo_opts("make_config_repos.sh", "bare-no-config", gix::open::Options::isolated())?;
        assert!(
            repo.is_bare(),
            "without config, we can't really know what the repo is actually but can guess by not having a worktree"
        );
        assert_eq!(repo.kind(), gix::repository::Kind::Common);
        assert_eq!(repo.workdir(), None);
        assert!(repo.worktree().is_none());
        assert_eq!(
            repo.config_snapshot().meta().source,
            gix::config::Source::Local,
            "config always refers to the local one for safety"
        );
        Ok(())
    }

    #[test]
    fn non_bare() -> crate::Result {
        let repo = named_subrepo_opts(
            "make_config_repos.sh",
            "worktree-no-config",
            gix::open::Options::isolated(),
        )?;
        assert!(
            !repo.is_bare(),
            "without config, we can't really know what the repo is actually but can guess as there is a worktree"
        );
        assert_eq!(repo.kind(), gix::repository::Kind::Common);
        assert!(repo.workdir().is_some());
        assert!(repo.worktree().is_some());
        assert_eq!(
            repo.config_snapshot().meta().source,
            gix::config::Source::Local,
            "config always refers to the local one for safety"
        );
        Ok(())
    }
}

mod not_a_repository {

    #[test]
    fn shows_proper_error() -> crate::Result {
        for name in ["empty-dir", "with-files"] {
            let name = format!("not-a-repo-{name}");
            let repo_path = gix_testtools::scripted_fixture_read_only("make_config_repos.sh")?.join(name);
            let err = gix::open_opts(&repo_path, gix::open::Options::isolated()).unwrap_err();
            assert!(matches!(err, gix::open::Error::NotARepository { path, .. } if path == repo_path));
        }
        Ok(())
    }
}

mod object_format_extension {
    use crate::util::named_subrepo_opts;

    #[test]
    fn rejects_object_format_on_v0_repo() -> crate::Result {
        // objectFormat is a "v1-only" extension: git refuses to operate on a version-0 repo that
        // sets it, even for sha1 (unlike grandfathered extensions like preciousObjects, which v0
        // still honours). This rejection was introduced in git 2.29.0 (2020). We match it.
        for name in [
            "objectformat-sha256-with-repository-format-v0",
            "objectformat-sha1-with-repository-format-v0",
        ] {
            let err = named_subrepo_opts("make_config_repos.sh", name, gix::open::Options::isolated())
                .expect_err("a v0 repository setting extensions.objectFormat must be rejected");
            assert!(
                matches!(
                    err,
                    gix::open::Error::Config(gix::config::Error::ObjectFormatRequiresV1)
                ),
                "objectFormat on a v0 repository must be rejected, got {err:?} for {name}"
            );
        }
        Ok(())
    }

    #[test]
    fn rejects_future_repository_format_versions() -> crate::Result {
        let err = named_subrepo_opts(
            "make_config_repos.sh",
            "repository-format-v2-with-objectformat-sha1",
            gix::open::Options::isolated(),
        )
        .expect_err("future repository format versions must be rejected");
        assert!(
            matches!(
                err,
                gix::open::Error::Config(gix::config::Error::UnsupportedRepositoryFormatVersion { version: 2 })
            ),
            "future repository format versions must be rejected before interpreting extensions, got {err:?}"
        );
        Ok(())
    }
}

mod open_path_as_is {

    use crate::util::{named_subrepo_opts, repo_opts};

    fn open_path_as_is() -> gix::open::Options {
        gix::open::Options::isolated().open_path_as_is(true)
    }

    #[test]
    fn bare_repos_open_normally() -> crate::Result {
        assert!(named_subrepo_opts("make_basic_repo.sh", "bare.git", open_path_as_is())?.is_bare());
        Ok(())
    }

    #[test]
    fn worktrees_cannot_be_opened() -> crate::Result {
        let err = repo_opts("make_basic_repo.sh", open_path_as_is()).unwrap_err();
        assert!(matches!(err, gix::open::Error::NotARepository { .. }));
        Ok(())
    }

    #[test]
    fn git_dir_within_worktrees_open_normally() -> crate::Result {
        assert!(!named_subrepo_opts("make_basic_repo.sh", ".git", open_path_as_is())?.is_bare());
        Ok(())
    }
}

mod submodules {
    use std::path::Path;

    #[test]
    fn by_their_worktree_checkout_and_git_modules_dir() {
        let dir = gix_testtools::scripted_fixture_read_only("make_submodules.sh").unwrap();
        let parent_repo = Path::new("with-submodules");
        let modules = parent_repo.join(".git").join("modules");
        for module in ["m1", "dir/m1"] {
            let submodule_m1_workdir = parent_repo.join(module);
            let submodule_m1_gitdir = modules.join(module);

            for discover_dir in [
                submodule_m1_workdir.clone(),
                submodule_m1_workdir.join("subdir"),
                submodule_m1_gitdir.clone(),
            ] {
                let repo = discover_repo(discover_dir).unwrap();
                assert_eq!(repo.kind(), gix::repository::Kind::Submodule);
                assert_eq!(repo.workdir().expect("non-bare"), dir.join(&submodule_m1_workdir));
                assert_eq!(repo.git_dir(), dir.join(&submodule_m1_gitdir));

                let repo = gix::open_opts(repo.workdir().expect("non-bare"), gix::open::Options::isolated()).unwrap();
                assert_eq!(repo.kind(), gix::repository::Kind::Submodule);
                assert_eq!(repo.workdir().expect("non-bare"), dir.join(&submodule_m1_workdir));
                assert_eq!(repo.git_dir(), dir.join(&submodule_m1_gitdir));
            }
        }
    }

    fn discover_repo(name: impl AsRef<Path>) -> crate::Result<gix::Repository> {
        let dir = gix_testtools::scripted_fixture_read_only("make_submodules.sh")?;
        let repo_dir = dir.join(name);
        Ok(gix::ThreadSafeRepository::discover_opts(
            repo_dir,
            Default::default(),
            gix_sec::trust::Mapping {
                full: crate::restricted(),
                reduced: crate::restricted(),
            },
        )?
        .to_thread_local())
    }
}

mod object_caches {

    use crate::util::named_subrepo_opts;

    #[test]
    fn default_git_and_custom_caches() -> crate::Result {
        let opts = gix::open::Options::isolated();
        let repo = named_subrepo_opts("make_config_repos.sh", "object-caches", opts)?;
        assert_eq!(
            repo.objects.has_object_cache(),
            cfg!(all(feature = "parallel", feature = "comfort"))
        );
        assert_eq!(
            repo.objects.has_pack_cache(),
            cfg!(all(feature = "parallel", feature = "comfort"))
        );
        Ok(())
    }

    #[test]
    fn disabled() -> crate::Result {
        let opts = gix::open::Options::isolated();
        let repo = named_subrepo_opts("make_config_repos.sh", "disabled-object-caches", opts)?;
        assert!(!repo.objects.has_object_cache());
        assert!(!repo.objects.has_pack_cache());
        Ok(())
    }
}

mod pack_alloc_limit_bytes {
    use gix_odb::HeaderExt;
    use gix_odb::find::Header;
    use gix_sec::Trust;

    use crate::util::repo_opts;

    #[test]
    fn limits_packed_object_allocations() -> crate::Result {
        let repo = repo_opts("make_packed_and_loose.sh", crate::util::restricted())?.to_thread_local();
        let packed_only_id = repo
            .objects
            .iter()?
            .find_map(|id| {
                let id = id.ok()?;
                matches!(repo.objects.header(id).ok()?, Header::Packed(_)).then_some(id)
            })
            .expect("fixture contains packed-only objects");
        assert!(
            repo.find_object(packed_only_id).is_ok(),
            "without a configured allocation limit packed objects are readable"
        );

        let limited = repo_opts(
            "make_packed_and_loose.sh",
            gix::open::Options::isolated().config_overrides(["gitoxide.objects.allocLimit=1"]),
        )?
        .to_thread_local();
        assert!(
            limited.find_object(packed_only_id).is_err(),
            "a tiny allocation limit rejects packed object reads"
        );
        Ok(())
    }

    #[test]
    fn limits_loose_object_allocations() -> crate::Result {
        let repo = repo_opts("make_packed_and_loose.sh", crate::util::restricted())?.to_thread_local();
        let loose_only_blob_id = repo
            .objects
            .iter()?
            .find_map(|id| {
                let id = id.ok()?;
                match repo.objects.header(id).ok()? {
                    Header::Loose {
                        kind: gix_object::Kind::Blob,
                        size,
                    } if size > 1 => Some(id),
                    _ => None,
                }
            })
            .expect("fixture contains loose-only blobs");
        assert!(
            repo.find_object(loose_only_blob_id).is_ok(),
            "without a configured allocation limit loose objects are readable"
        );

        let limited = repo_opts(
            "make_packed_and_loose.sh",
            gix::open::Options::isolated().config_overrides(["gitoxide.objects.allocLimit=1"]),
        )?
        .to_thread_local();
        assert!(
            matches!(
                limited.find_header(loose_only_blob_id)?,
                Header::Loose {
                    kind: gix_object::Kind::Blob,
                    size,
                } if size > 1
            ),
            "loose headers can always be found, independently of the allocation limit"
        );
        assert!(
            limited.find_object(loose_only_blob_id).is_err(),
            "a tiny allocation limit rejects loose object reads"
        );
        Ok(())
    }

    #[test]
    fn reduced_trust_sets_a_default_limit_unless_disabled() -> crate::Result {
        let base = repo_opts("make_packed_and_loose.sh", crate::util::restricted())?.to_thread_local();
        let packed_only_id = base
            .objects
            .iter()?
            .map(Result::unwrap)
            .next()
            .expect("fixture contains packed-only objects");
        assert!(
            base.find_object(packed_only_id).is_ok(),
            "trusted repositories keep reading packed objects without an implicit limit"
        );

        let reduced = repo_opts(
            "make_packed_and_loose.sh",
            crate::util::restricted()
                .with(Trust::Reduced)
                .config_overrides(["gitoxide.objects.allocLimitIfReducedTrust=1"]),
        )?
        .to_thread_local();
        assert!(
            reduced.find_object(packed_only_id).is_err(),
            "reduced trust applies the configured fallback allocation limit if none was configured"
        );

        let reduced_without_fallback = repo_opts(
            "make_packed_and_loose.sh",
            crate::util::restricted()
                .with(Trust::Reduced)
                .config_overrides(["gitoxide.objects.allocLimitIfReducedTrust=0"]),
        )?
        .to_thread_local();
        assert!(
            reduced_without_fallback.find_object(packed_only_id).is_ok(),
            "the reduced-trust fallback can be disabled explicitly"
        );

        Ok(())
    }
}

mod object_refresh {
    use std::time::Duration;

    use gix_odb::store::RefreshMode;

    use crate::util::repo_opts;

    #[test]
    fn defaults_to_a_configurable_freshness_window() -> crate::Result {
        let mut default = repo_opts("make_packed_and_loose.sh", crate::util::restricted())?.to_thread_local();
        assert_eq!(
            default.objects.refresh_mode(),
            RefreshMode::AfterDuration(Duration::from_secs(1)),
            "gix avoids repeated miss-triggered filesystem scans by default"
        );
        {
            let mut config = default.config_snapshot_mut();
            config.set_raw_value("gitoxide.objects.refreshAfter", "50")?;
            config.commit()?;
        }
        assert_eq!(
            default.objects.refresh_mode(),
            RefreshMode::AfterDuration(Duration::from_millis(50)),
            "committing configuration updates the active object handle"
        );

        let mut configured = repo_opts(
            "make_packed_and_loose.sh",
            gix::open::Options::isolated().config_overrides(["gitoxide.objects.refreshAfter=250"]),
        )?
        .to_thread_local();
        assert_eq!(
            configured.objects.refresh_mode(),
            RefreshMode::AfterDuration(Duration::from_millis(250)),
            "gitoxide.objects.refreshAfter configures the freshness window"
        );
        Ok(())
    }
}

mod worktree {
    use gix::open;

    #[test]
    fn with_worktree_configs() -> gix_testtools::Result {
        let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let fixture_dir = gix_testtools::scripted_fixture_read_only("make_worktree_repo_with_configs.sh")?;
        let worktree_base = manifest_dir.join(&fixture_dir).join("repo/.git/worktrees");

        {
            let base = open(fixture_dir.join("repo"))?;
            let base_config = base.config_snapshot();

            assert_eq!(
                base.workdir(),
                Some(fixture_dir.join("repo").as_path()),
                "the main worktree"
            );
            assert_eq!(base.git_dir(), fixture_dir.join("repo/.git"), "git dir and…");
            assert_eq!(
                base.common_dir(),
                fixture_dir.join("repo/.git"),
                "…common dir are the same"
            );

            assert_eq!(
                base_config.string("worktree.setting").expect("exists"),
                "set in the main worktree"
            );
            assert_eq!(
                base_config.string("shared.setting").expect("exists"),
                "set in the shared config"
            );
            assert_eq!(
                base_config.string("override.setting").expect("exists"),
                "set in the shared config"
            );
        }

        {
            let wt1 = open(fixture_dir.join("wt-1"))?;
            let wt1_config = wt1.config_snapshot();
            assert_eq!(
                wt1.workdir(),
                Some(fixture_dir.join("wt-1").as_path()),
                "a linked worktree in its own location"
            );
            assert_eq!(
                wt1.git_dir(),
                worktree_base.join("wt-1"),
                "whose git-dir is within the common dir"
            );
            assert_eq!(
                wt1.common_dir(),
                worktree_base.join("wt-1/../.."),
                "the common dir is the `git-dir` of the repository with the main worktree"
            );

            assert_eq!(wt1_config.string("worktree.setting").expect("exists"), "set in wt-1");
            assert_eq!(
                wt1_config.string("shared.setting").expect("exists"),
                "set in the shared config"
            );
            assert_eq!(
                wt1_config.string("override.setting").expect("exists"),
                "set in the shared config"
            );
        }

        {
            let wt2 = open(fixture_dir.join("wt-2"))?;
            let wt2_config = wt2.config_snapshot();
            assert_eq!(
                wt2.workdir(),
                Some(fixture_dir.join("wt-2").as_path()),
                "another linked worktree as sibling to wt-1"
            );
            assert_eq!(wt2.git_dir(), worktree_base.join("wt-2"));
            assert_eq!(wt2.common_dir(), worktree_base.join("wt-2/../.."));

            assert_eq!(wt2_config.string("worktree.setting").expect("exists"), "set in wt-2");
            assert_eq!(
                wt2_config.string("shared.setting").expect("exists"),
                "set in the shared config"
            );
            assert_eq!(
                wt2_config.string("override.setting").expect("exists"),
                "override in wt-2"
            );
        }

        Ok(())
    }
}
