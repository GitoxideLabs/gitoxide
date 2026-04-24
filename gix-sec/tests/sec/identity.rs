#[test]
fn is_path_owned_by_current_user() -> crate::Result {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("file");
    std::fs::write(&file, [])?;
    assert!(gix_sec::identity::is_path_owned_by_current_user(&file)?);
    assert!(gix_sec::identity::is_path_owned_by_current_user(dir.path())?);
    Ok(())
}

/// Ownership checks intentionally inspect the symlink itself rather than following the target.
/// This matches Git's longstanding `lstat()`-based behavior, which treats a user-owned symlink as
/// owned by that user even if its target is owned by someone else.
#[test]
#[cfg(all(unix, not(target_os = "wasi")))]
fn symlink_ownership_checks_inspect_the_link_itself() -> crate::Result {
    use std::os::unix::fs as unix_fs;
    use std::os::unix::fs::MetadataExt;

    let current_uid = unsafe { libc::geteuid() };
    let Some(candidate) = ["/etc/passwd", "/etc/hosts", "/bin/sh", "/bin/ls", "/dev/null"]
        .into_iter()
        .map(std::path::Path::new)
        .find(|path| path.exists() && std::fs::metadata(path).is_ok_and(|meta| meta.uid() != current_uid))
    else {
        return Ok(());
    };

    let dir = tempfile::tempdir()?;
    let symlink = dir.path().join("trusted-link");
    unix_fs::symlink(candidate, &symlink)?;

    assert!(
        gix_sec::identity::is_path_owned_by_current_user(&symlink)?,
        "ownership checks intentionally trust the user-owned symlink itself, matching Git"
    );
    Ok(())
}

#[test]
#[cfg(windows)]
fn windows_home() -> crate::Result {
    let home = gix_path::env::home_dir().expect("home dir is available");
    assert!(gix_sec::identity::is_path_owned_by_current_user(&home)?);
    Ok(())
}
