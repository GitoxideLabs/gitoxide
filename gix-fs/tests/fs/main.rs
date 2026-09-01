type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

mod capabilities;
mod dir;
mod read_dir;
mod snapshot;
mod stack;

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
        adjust(0o1755, -0o640),
        0o1640,
        "a negative mode replaces permission bits but retains unrelated mode bits"
    );
}
