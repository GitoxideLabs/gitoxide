use gix_fs::SharedPermissions;
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
    gix_fs::set_shared_repository_permissions(&missing, SharedPermissions::default())?;
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
    gix_fs::set_shared_repository_permissions(&file, SharedPermissions::Group)?;
    assert_eq!(
        fs::metadata(&file)?.permissions().mode() & 0o777,
        0o660,
        "group sharing adds permission bits"
    );
    gix_fs::set_shared_repository_permissions(&file, SharedPermissions::Explicit(0o640))?;
    assert_eq!(
        fs::metadata(&file)?.permissions().mode() & 0o777,
        0o640,
        "an explicit policy can remove permission bits"
    );
    fs::set_permissions(&file, fs::Permissions::from_mode(0o444))?;
    gix_fs::set_shared_repository_permissions(&file, SharedPermissions::Group)?;
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
    use gix_fs::SharedPermissions::{Everybody, Explicit, Group, Umask};
    use std::os::unix::fs::PermissionsExt;
    let adjust = |mode, shared_repository_permissions| {
        gix_fs::adjust_shared_repository_permissions(
            std::fs::Permissions::from_mode(mode),
            shared_repository_permissions,
        )
        .mode()
    };

    assert_eq!(adjust(0o640, Umask), 0o640, "umask retains the post-umask mode");
    assert_eq!(adjust(0o604, Group), 0o664, "group sharing adds permissions");
    assert_eq!(
        adjust(0o604, Explicit(0o660)),
        0o660,
        "an explicit group-like mode removes permissions outside the mode"
    );
    assert_eq!(
        adjust(0o444, Group),
        0o444,
        "sharing does not make read-only files writable"
    );
    assert_eq!(
        adjust(0o700, Everybody),
        0o775,
        "executable files gain execute bits alongside read bits"
    );
    assert_eq!(
        adjust(0o1755, Explicit(0o2640)),
        0o1750,
        "an explicit mode replaces permission bits, preserves executability and leaves special bits alone"
    );
    assert_eq!(
        adjust(0o040700, Group),
        if cfg!(any(target_os = "freebsd", target_os = "openbsd")) {
            0o040770
        } else {
            0o042770
        },
        "shared directories gain search access and setgid according to Git's platform defaults, including on macOS"
    );
}
