use gix::config::tree::{Branch, Core, Key, Pack, gitoxide};

use crate::{named_repo, repo_rw, repo_rw_opts};

fn write_config_with_new_mtime(path: &std::path::Path, config: &gix_config::File) -> crate::Result {
    let previous = std::fs::metadata(path)?.modified()?;
    std::fs::write(path, config.to_bstring())?;
    let changed = previous
        .checked_sub(std::time::Duration::from_secs(2))
        .expect("fixture modification time is after the Unix epoch");
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)?
        .set_modified(changed)?;
    assert_ne!(std::fs::metadata(path)?.modified()?, previous, "mtime must change");
    Ok(())
}

fn options_with_includes() -> gix::open::Options {
    let mut permissions = gix::open::Permissions::isolated();
    permissions.config.includes = true;
    gix::open::Options::isolated().permissions(permissions)
}

#[cfg(feature = "credentials")]
mod credential_helpers;

#[test]
fn commit_auto_rollback() -> crate::Result {
    let mut repo = named_repo("make_basic_repo.sh")?;
    let default_abbrev = repo.head_id()?.to_string()[..7].to_owned();
    let short_abbrev = repo.head_id()?.to_string()[..4].to_owned();
    assert_eq!(repo.head_id()?.shorten()?.to_string(), default_abbrev);

    {
        let mut config = repo.config_snapshot_mut();
        config.set_raw_value(Core::ABBREV, "4")?;
        let repo = config.commit_auto_rollback()?;
        assert_eq!(repo.head_id()?.shorten()?.to_string(), short_abbrev);
    }

    assert_eq!(repo.head_id()?.shorten()?.to_string(), default_abbrev);

    let repo = {
        let mut config = repo.config_snapshot_mut();
        config.set_raw_value(Core::ABBREV, "4")?;
        let mut repo = config.commit_auto_rollback()?;
        assert_eq!(repo.head_id()?.shorten()?.to_string(), short_abbrev);
        // access to the mutable repo underneath
        repo.object_cache_size_if_unset(16 * 1024);
        repo.rollback()?
    };
    assert_eq!(repo.head_id()?.shorten()?.to_string(), default_abbrev);

    Ok(())
}

mod trusted_path {
    use crate::util::named_repo;

    #[test]
    fn optional_is_respected() -> crate::Result {
        let mut repo = named_repo("make_basic_repo.sh")?;
        repo.config_snapshot_mut().set_raw_value("my.path", "does-not-exist")?;

        let actual = repo.config_snapshot().trusted_path("my.path")?.expect("is set");
        assert_eq!(
            actual,
            std::path::PathBuf::from("does-not-exist"),
            "the path isn't evaluated by default, and may not exist"
        );

        repo.config_snapshot_mut()
            .set_raw_value("my.path", ":(optional)does-not-exist")?;
        let actual = repo.config_snapshot().trusted_path("my.path")?;
        assert_eq!(actual, None, "non-existing paths aren't returned to the caller");
        Ok(())
    }
}

#[test]
fn snapshot_mut_commit_and_forget() -> crate::Result {
    let mut repo = named_repo("make_basic_repo.sh")?;
    let repo = {
        let mut repo = repo.config_snapshot_mut();
        repo.set_value(&Core::ABBREV, "4")?;
        repo.commit()?
    };
    assert_eq!(repo.config_snapshot().integer("core.abbrev").expect("set"), 4);
    {
        let mut repo = repo.config_snapshot_mut();
        repo.set_raw_value(Core::ABBREV, "8")?;
        repo.forget();
    }
    assert_eq!(repo.config_snapshot().integer("core.abbrev"), Some(4));
    Ok(())
}

#[test]
fn committing_loose_compression_requires_reopening_the_object_store() -> crate::Result {
    use gix::objs::Write;

    fn loose_object_size(repo: &gix::Repository, id: gix::ObjectId) -> std::io::Result<u64> {
        let hex = id.to_string();
        std::fs::metadata(repo.git_dir().join("objects").join(&hex[..2]).join(&hex[2..])).map(|meta| meta.len())
    }

    let (mut repo, _tmp) = repo_rw("make_basic_repo.sh")?;
    let mut data = vec![b'a'; 128 * 1024];
    let compressed = repo.objects.write_buf(gix::objs::Kind::Blob, &data)?;
    let compressed_size = loose_object_size(&repo, compressed)?;

    let mut config = repo.config_snapshot_mut();
    config.set_value(&Core::LOOSE_COMPRESSION, "0")?;
    config.commit()?;

    data[0] = b'b';
    let still_compressed = repo.objects.write_buf(gix::objs::Kind::Blob, &data)?;
    let still_compressed_size = loose_object_size(&repo, still_compressed)?;

    let git_dir = repo.git_dir().to_owned();
    let options = repo
        .open_options()
        .clone()
        .config_overrides(["core.looseCompression=0"]);
    repo = gix::open_opts(git_dir, options)?;

    data[1] = b'b';
    let uncompressed = repo.write_blob(&data)?;
    let uncompressed_size = loose_object_size(&repo, uncompressed.detach())?;
    assert!(
        uncompressed_size > compressed_size * 10 && uncompressed_size > still_compressed_size * 10,
        "the override should take effect after reopening the object store: {compressed_size}, {still_compressed_size} vs {uncompressed_size}"
    );
    Ok(())
}

#[test]
fn compression_levels() -> crate::Result {
    use gix::zlib::Compression;

    let mut repo = named_repo("make_basic_repo.sh")?;
    assert_eq!(repo.loose_compression(), Compression::BEST_SPEED);
    assert_eq!(repo.pack_compression()?, Compression::DEFAULT);

    let mut config = repo.config_snapshot_mut();
    config.set_value(&Core::COMPRESSION, "4")?;
    config.commit()?;
    assert_eq!(repo.loose_compression(), Compression::new(4).expect("valid level"));
    assert_eq!(repo.pack_compression()?, Compression::new(4).expect("valid level"));

    let mut config = repo.config_snapshot_mut();
    config.set_value(&Core::LOOSE_COMPRESSION, "2")?;
    config.set_value(&Pack::COMPRESSION, "8")?;
    config.commit()?;
    assert_eq!(repo.loose_compression(), Compression::new(2).expect("valid level"));
    assert_eq!(repo.pack_compression()?, Compression::new(8).expect("valid level"));

    Ok(())
}

#[test]
fn values_are_set_in_memory_only() {
    let mut repo = named_repo("make_config_repo.sh").unwrap();
    let repo_clone = repo.clone();
    let key = "hallo.welt";
    let key_subsection = "branch.main.merge";
    assert_eq!(repo.config_snapshot().boolean(key), None, "no value there just yet");
    assert_eq!(repo.config_snapshot().string(key_subsection), None);

    {
        let mut config = repo.config_snapshot_mut();
        config.set_raw_value("hallo.welt", "true").unwrap();
        config
            .set_subsection_value(&Branch::MERGE, "main", "refs/heads/foo")
            .unwrap();
    }

    assert_eq!(
        repo.config_snapshot().boolean(key),
        Some(true),
        "value was set and applied"
    );
    assert_eq!(
        repo.config_snapshot()
            .string(key_subsection)
            .expect("value was just set"),
        "refs/heads/foo"
    );

    assert_eq!(
        repo_clone.config_snapshot().boolean(key),
        None,
        "values are not written back automatically nor are they shared between clones"
    );
    assert_eq!(repo_clone.config_snapshot().string(key_subsection), None);
}

#[test]
fn set_value_in_subsection() {
    let mut repo = named_repo("make_config_repo.sh").unwrap();
    {
        let mut config = repo.config_snapshot_mut();
        config
            .set_value(&gitoxide::Credentials::TERMINAL_PROMPT, "yes")
            .unwrap();
        assert_eq!(
            config
                .string(&*gitoxide::Credentials::TERMINAL_PROMPT.logical_name())
                .expect("just set"),
            "yes"
        );
    }
}

#[test]
fn apply_cli_overrides() -> crate::Result {
    let mut repo = named_repo("make_config_repo.sh").unwrap();
    repo.config_snapshot_mut().append_config(
        [
            "a.b=c",
            "remote.origin.url = url",
            "implicit.bool-true",
            "implicit.bool-false = ",
        ],
        gix_config::Source::Cli,
    )?;

    let config = repo.config_snapshot();
    assert_eq!(config.string("a.b").expect("present"), "c");
    assert_eq!(config.string("remote.origin.url").expect("present"), "url");
    assert_eq!(
        config.string("implicit.bool-true"),
        None,
        "no keysep is interpreted as 'not present' as we don't make up values"
    );
    assert_eq!(
        config.string("implicit.bool-false").expect("present"),
        "",
        "empty values are fine"
    );
    assert_eq!(
        config.boolean("implicit.bool-false"),
        Some(false),
        "empty values are boolean true"
    );
    assert_eq!(
        config.boolean("implicit.bool-true"),
        Some(true),
        "values without key-sep are true"
    );

    Ok(())
}

#[test]
fn reload_reloads_on_disk_changes() -> crate::Result {
    let (mut repo, _tmp) = repo_rw("make_config_repo.sh")?;
    assert_eq!(repo.config_snapshot().integer("core.abbrev"), None);
    let original_index = repo.index_path();
    let changed_index = repo.git_dir().join("changed-index");

    let config_path = repo.git_dir().join("config");
    let mut config = gix_config::File::from_path_no_includes(config_path.clone(), gix_config::Source::Local)?;
    config.set_raw_value("core.abbrev", "4")?;
    config.set_raw_value("gitoxide.core.indexFile", gix_path::into_bstr(&changed_index).as_ref())?;
    std::fs::write(config_path, config.to_bstring())?;

    assert_eq!(repo.config_snapshot().integer("core.abbrev"), None);
    assert_eq!(repo.index_path(), original_index, "repository locations remain cached");

    repo.reload()?;

    assert_eq!(repo.config_snapshot().integer("core.abbrev"), Some(4));
    assert_eq!(
        repo.index_path(),
        changed_index,
        "reload reapplies repository locations"
    );
    Ok(())
}

#[test]
fn reload_discards_in_memory_only_changes() -> crate::Result {
    let mut repo = named_repo("make_config_repo.sh")?;

    repo.config_snapshot_mut().set_raw_value(Core::ABBREV, "4")?;
    assert_eq!(repo.config_snapshot().integer("core.abbrev"), Some(4));

    repo.reload()?;
    assert_eq!(repo.config_snapshot().integer("core.abbrev"), None);
    Ok(())
}

#[test]
fn config_access_refreshes_file_changes_but_snapshot_access_does_not() -> crate::Result {
    let (mut repo, _tmp) = repo_rw_opts("make_config_repo.sh", options_with_includes())?;
    let original_index = repo.index_path();
    let changed_index = repo.git_dir().join("changed-index");
    let included_path = repo.git_dir().parent().expect("worktree repository").join("a.config");
    let mut included = gix_config::File::from_path_no_includes(included_path.clone(), gix_config::Source::Local)?;
    included.set_raw_value(Core::ABBREV, "4")?;
    included.set_raw_value("gitoxide.core.indexFile", gix_path::into_bstr(&changed_index).as_ref())?;
    write_config_with_new_mtime(&included_path, &included)?;

    assert_eq!(
        repo.config_snapshot_mut().integer("core.abbrev")?,
        None,
        "snapshot access must remain free of freshness checks"
    );
    assert_eq!(repo.config()?.integer("core.abbrev"), Some(4));
    assert_eq!(
        repo.head_id()?.shorten()?.to_string().len(),
        4,
        "cached values are refreshed"
    );
    assert_eq!(
        repo.index_path(),
        original_index,
        "configuration refresh must not retarget repository paths"
    );

    std::fs::remove_file(&included_path)?;
    assert_eq!(
        repo.config()?.string("a.local-override").expect("base value remains"),
        "base",
        "deleting an included file refreshes the configuration"
    );

    std::fs::write(&included_path, included.to_bstring())?;
    assert_eq!(
        repo.config()?.integer("core.abbrev"),
        Some(4),
        "recreating a previously observed include refreshes it again"
    );
    Ok(())
}

#[test]
fn config_mut_preserves_runtime_api_sections_without_duplicating_open_overrides() -> crate::Result {
    let options = options_with_includes().config_overrides([
        "user.name=gitoxide",
        "user.email=gitoxide@localhost",
        "refresh.open=from-options",
    ]);
    let (mut repo, _tmp) = repo_rw_opts("make_config_repo.sh", options)?;
    repo.config_snapshot_mut()
        .append_config(["refresh.runtime=from-api"], gix_config::Source::Api)?;

    let included_path = repo.git_dir().parent().expect("worktree repository").join("a.config");
    let mut included = gix_config::File::from_path_no_includes(included_path.clone(), gix_config::Source::Local)?;
    included.set_raw_value("refresh.disk", "changed")?;
    write_config_with_new_mtime(&included_path, &included)?;

    let config = repo.config_mut()?;
    assert_eq!(config.string("refresh.open").expect("opening override"), "from-options");
    assert_eq!(config.string("refresh.runtime").expect("runtime API value"), "from-api");
    assert_eq!(config.string("refresh.disk").expect("refreshed disk value"), "changed");
    assert_eq!(
        config
            .sections()
            .filter(|section| {
                section.meta().source == gix_config::Source::Api && section.header().name() == "refresh"
            })
            .count(),
        2,
        "the opening and runtime API sections occur exactly once"
    );
    config.forget();

    included.set_raw_value("refresh.disk", "changed-again")?;
    write_config_with_new_mtime(&included_path, &included)?;
    let config = repo.config_mut()?;
    assert_eq!(config.string("refresh.runtime").expect("runtime API value"), "from-api");
    assert_eq!(
        config.string("refresh.disk").expect("refreshed disk value"),
        "changed-again"
    );
    assert_eq!(
        config
            .sections()
            .filter(|section| {
                section.meta().source == gix_config::Source::Api && section.header().name() == "refresh"
            })
            .count(),
        2,
        "repeated refreshes neither duplicate nor lose API sections"
    );
    Ok(())
}
