use gix_testtools::TestResult;

mod capabilities;
mod dir;
mod file;
mod read_dir;
mod snapshot;
mod stack;
mod symlink;
#[cfg(feature = "walkdir")]
mod walkdir;

#[test]
fn default_sharing_policy_does_not_access_the_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let missing = dir.path().join("missing/file");
    gix_fs::set_shared_repository_permissions(&missing, 0)?;
    assert!(
        !missing.exists(),
        "the default sharing policy performs no filesystem access"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn shared_repository_permissions_preserve_readonly_and_apply_explicit_modes() -> TestResult {
    use std::{fs, os::unix::fs::PermissionsExt};

    let dir = tempfile::tempdir()?;
    let file = dir.path().join("file");
    fs::write(&file, b"contents")?;
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600))?;
    gix_fs::set_shared_repository_permissions(&file, 0o660)?;
    assert_eq!(
        fs::metadata(&file)?.permissions().mode() & 0o777,
        0o660,
        "a positive policy adds permission bits"
    );
    gix_fs::set_shared_repository_permissions(&file, -0o640)?;
    assert_eq!(
        fs::metadata(&file)?.permissions().mode() & 0o777,
        0o640,
        "an explicit policy can remove permission bits"
    );
    fs::set_permissions(&file, fs::Permissions::from_mode(0o444))?;
    gix_fs::set_shared_repository_permissions(&file, 0o660)?;
    assert_eq!(
        fs::metadata(&file)?.permissions().mode() & 0o777,
        0o444,
        "sharing preserves a read-only file"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn shared_repository_permissions_are_applied_after_the_umask() {
    use std::os::unix::fs::PermissionsExt;
    let adjust = |mode, shared_repository_permissions| {
        gix_fs::adjust_shared_repository_permissions(
            std::fs::Permissions::from_mode(mode),
            shared_repository_permissions,
        )
        .mode()
    };

    assert_eq!(adjust(0o640, 0), 0o640, "zero retains the post-umask mode");
    assert_eq!(adjust(0o600, 0o660), 0o660, "a positive mode adds permissions");
    assert_eq!(
        adjust(0o444, 0o660),
        0o444,
        "sharing does not make read-only files writable"
    );
    assert_eq!(
        adjust(0o700, 0o664),
        0o775,
        "executable files gain execute bits alongside read bits"
    );
    assert_eq!(
        adjust(0o1755, -0o640),
        0o1750,
        "a negative mode replaces permission bits, preserves executability and retains unrelated mode bits"
    );
    assert_eq!(
        adjust(0o040700, 0o660),
        if cfg!(any(target_os = "freebsd", target_os = "openbsd")) {
            0o040770
        } else {
            0o042770
        },
        "shared directories gain search access and setgid according to Git's platform defaults, including on macOS"
    );
}
