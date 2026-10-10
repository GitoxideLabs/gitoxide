use std::path::{Path, PathBuf};

use gix::Repository;
use gix_testtools::TestResult;

#[derive(Debug, PartialEq, Eq)]
struct Paths {
    git_dir: PathBuf,
    common_dir: PathBuf,
    workdir: Option<PathBuf>,
    index_path: PathBuf,
    objects: PathBuf,
}

impl Paths {
    fn of(repo: &Repository) -> Self {
        Self {
            git_dir: repo.git_dir().to_owned(),
            common_dir: repo.common_dir().to_owned(),
            workdir: repo.workdir().map(Path::to_owned),
            index_path: repo.index_path(),
            objects: repo.objects.store_ref().path().to_owned(),
        }
    }

    fn absolute(&self, current_dir: &Path) -> Self {
        Self {
            git_dir: current_dir.join(&self.git_dir),
            common_dir: current_dir.join(&self.common_dir),
            workdir: self.workdir.as_ref().map(|workdir| current_dir.join(workdir)),
            index_path: current_dir.join(&self.index_path),
            objects: current_dir.join(&self.objects),
        }
    }
}

#[test]
fn relative_paths_retain_runtime_state_and_the_selected_index_after_changing_cwd() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable("make_basic_repo.sh")?;
    let elsewhere = gix_testtools::tempfile::TempDir::new()?;
    let _cwd = gix_testtools::set_current_dir(fixture.path())?;
    std::fs::copy(".git/index", ".git/temporary-index")?;
    let mut repo = gix::open_opts(
        "some/..",
        crate::restricted().config_overrides(["gitoxide.core.indexFile=temporary-index"]),
    )?
    .with_object_memory();
    let current_dir = repo.current_dir().to_owned();
    let original_paths = Paths::of(&repo);
    assert!(
        original_paths.git_dir.is_relative()
            && original_paths
                .git_dir
                .components()
                .any(|component| component == std::path::Component::ParentDir),
        "the input Git directory is relative and contains a parent component to detect canonicalization"
    );
    assert_eq!(
        original_paths.index_path,
        current_dir.join(&original_paths.git_dir).join("temporary-index"),
        "the configured index is absolute from opening and retains the lexical Git directory spelling"
    );
    let commit_id = repo.head_id()?.detach();

    #[cfg(feature = "index")]
    let original_index = repo.index()?;
    repo.config_snapshot_mut().set_raw_value("core.abbrev", "8")?;
    repo.config_snapshot_mut()
        .set_raw_value("gitoxide.core.indexFile", "different-index")?;
    repo.set_namespace("location")?;
    repo.reference(
        "refs/heads/namespace-only",
        commit_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "create namespace-only branch",
    )?;
    let namespace = repo.namespace().cloned();
    let config = repo.config_snapshot().to_bstring();
    let blob_id = repo.write_blob(b"only in object memory before anchoring")?.detach();
    let clone = repo.clone();
    let _moved_cwd = gix_testtools::set_current_dir(elsewhere.path())?;

    let returned_repo: &mut Repository = repo.make_paths_absolute()?;
    assert_eq!(
        Paths::of(returned_repo),
        original_paths.absolute(&current_dir),
        "every selected path is joined to the captured CWD without canonicalization or reselecting the index"
    );
    assert_eq!(repo.current_dir(), current_dir, "the opening CWD remains unchanged");
    assert_eq!(
        repo.config_snapshot().to_bstring(),
        config,
        "anchoring retains the complete configuration snapshot, including runtime edits"
    );
    assert_eq!(repo.namespace(), namespace.as_ref(), "the runtime namespace survives");
    assert_eq!(
        repo.find_reference("refs/heads/namespace-only")?.id(),
        commit_id,
        "the retained namespace resolves its disk reference after leaving the opening CWD"
    );
    assert_eq!(
        repo.clear_namespace(),
        namespace,
        "the retained namespace can be removed temporarily to access physical HEAD"
    );
    assert!(
        repo.try_find_reference("refs/heads/namespace-only")?.is_none(),
        "the namespace-only branch is not visible in the default namespace"
    );
    assert_eq!(
        repo.head_id()?,
        commit_id,
        "physical disk HEAD still resolves after leaving the opening CWD"
    );
    repo.set_namespace("location")?;
    assert_eq!(
        repo.namespace(),
        namespace.as_ref(),
        "the runtime namespace is restored"
    );

    assert_eq!(
        repo.find_object(commit_id)?.kind,
        gix::object::Kind::Commit,
        "the reconstructed ODB reads existing disk objects from the original repository"
    );
    assert_eq!(
        repo.find_blob(blob_id)?.data,
        b"only in object memory before anchoring",
        "objects written only to memory are carried into the reconstructed ODB"
    );
    let next_blob_id = repo.write_blob(b"only in object memory after anchoring")?.detach();
    let disk = gix::open_opts(repo.git_dir(), crate::restricted())?;
    assert!(
        !disk.has_object(blob_id) && !disk.has_object(next_blob_id),
        "object memory stays enabled, so neither old nor new memory objects reach disk"
    );
    assert_eq!(
        Paths::of(&clone),
        original_paths,
        "existing clones keep their relative paths"
    );

    assert!(
        clone.has_object(blob_id),
        "an existing clone retains its own memory objects"
    );
    #[cfg(feature = "index")]
    {
        let index = repo.index()?;
        assert_eq!(
            index.path(),
            repo.index_path(),
            "the cached index already uses the absolute selected path"
        );
        assert!(
            gix_parallel::OwnShared::ptr_eq(&index, &original_index),
            "anchoring retains the same cached index snapshot instead of reloading it"
        );
        assert_eq!(
            index.entries().len(),
            1,
            "the override still contains the fixture's tracked file"
        );
        assert_eq!(
            original_index.path(),
            original_paths.index_path,
            "the outstanding snapshot retains the index path selected during opening"
        );
    }
    let absolute_paths = Paths::of(&repo);
    let object_store = std::ptr::from_ref(repo.objects.store_ref());
    #[cfg(feature = "index")]
    let index = repo.index()?;
    repo.make_paths_absolute()?;
    #[cfg(feature = "index")]
    assert!(
        std::ptr::eq(&*index, &*repo.index()?),
        "an already-absolute repository keeps its existing index cache"
    );
    assert_eq!(Paths::of(&repo), absolute_paths, "repeated anchoring is idempotent");
    assert_eq!(
        std::ptr::from_ref(repo.objects.store_ref()),
        object_store,
        "an already-absolute repository does not reconstruct its ODB"
    );
    Ok(())
}

#[test]
fn bare_repositories_and_missing_checkouts_do_not_require_canonicalization() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable("make_basic_repo.sh")?;
    let elsewhere = gix_testtools::tempfile::TempDir::new()?;
    let _cwd = gix_testtools::set_current_dir(fixture.path())?;
    let mut bare = gix::open_opts("bare.git", crate::restricted())?;
    let bare_paths = Paths::of(&bare).absolute(bare.current_dir());
    let mut missing = gix::open_opts(".", crate::restricted())?;
    missing.set_workdir(Some(PathBuf::from("some/very/../very")))?;
    let missing_paths = Paths::of(&missing).absolute(missing.current_dir());
    let commit_id = missing.head_id()?.detach();
    std::fs::remove_dir_all("some")?;
    let _moved_cwd = gix_testtools::set_current_dir(elsewhere.path())?;

    bare.make_paths_absolute()?;
    assert_eq!(
        Paths::of(&bare),
        bare_paths,
        "bare repository paths use the captured CWD"
    );
    assert!(bare.is_bare(), "anchoring does not change the bare repository's kind");
    assert!(bare.workdir().is_none(), "anchoring does not invent a bare checkout");
    assert!(
        bare.head()?.is_unborn(),
        "the bare repository's disk HEAD is still accessible"
    );

    missing.make_paths_absolute()?;
    assert_eq!(
        Paths::of(&missing),
        missing_paths,
        "a nonexistent checkout is anchored lexically, preserving its parent component"
    );
    assert!(
        !missing
            .workdir()
            .expect("the selected checkout remains present in memory")
            .exists(),
        "anchoring does not recreate a missing checkout"
    );
    assert_eq!(
        missing.head_id()?,
        commit_id,
        "a missing checkout does not prevent reading HEAD"
    );
    assert_eq!(
        missing.find_object(commit_id)?.kind,
        gix::object::Kind::Commit,
        "a missing checkout does not prevent reading disk objects"
    );
    Ok(())
}

#[test]
#[cfg(feature = "worktree-mutation")]
fn linked_worktree_paths_are_anchored_independently() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let (source, _fixture) = crate::basic_rw_repo()?;
    let elsewhere = gix_testtools::tempfile::TempDir::new()?;
    let _cwd = gix_testtools::set_current_dir(source.workdir().expect("the fixture has a checkout"))?;
    let mut source = gix::open_opts(".", crate::restricted())?;
    source
        .config_snapshot_mut()
        .set_raw_value("worktree.useRelativePaths", "true")?;
    let commit_id = source.head_id()?.detach();
    source.add_worktree(
        source.current_dir().join("linked"),
        gix::worktree::add::Head::Detached(commit_id),
        gix::progress::Discard,
        &std::sync::atomic::AtomicBool::default(),
    )?;
    let mut linked = gix::open_opts(".git/worktrees/linked", crate::restricted())?;
    let original_paths = Paths::of(&linked);
    assert!(
        original_paths.git_dir.is_relative() && original_paths.common_dir.is_relative(),
        "both the private Git directory and the redirected common directory need anchoring"
    );
    assert_ne!(
        original_paths.git_dir, original_paths.common_dir,
        "the linked worktree has distinct private and common directories"
    );
    let expected_paths = original_paths.absolute(linked.current_dir());
    let _moved_cwd = gix_testtools::set_current_dir(elsewhere.path())?;

    linked.make_paths_absolute()?;
    assert_eq!(
        Paths::of(&linked),
        expected_paths,
        "private Git directory, common directory, checkout, index, and shared ODB are anchored independently"
    );
    assert_eq!(
        linked.kind(),
        gix::repository::Kind::LinkedWorkTree,
        "the linked-worktree relationship survives reconstruction"
    );
    assert_eq!(
        linked.head_id()?,
        commit_id,
        "HEAD is read from the private Git directory"
    );
    assert_eq!(
        linked.find_object(commit_id)?.kind,
        gix::object::Kind::Commit,
        "objects are read from the common directory's shared ODB"
    );
    assert_eq!(
        linked.index()?.path(),
        linked.index_path(),
        "the linked worktree reads its own index, not the main worktree's index"
    );
    Ok(())
}
