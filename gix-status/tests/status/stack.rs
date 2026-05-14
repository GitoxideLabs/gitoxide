fn stack() -> gix_status::SymlinkCheck {
    stack_in("base")
}

fn stack_in(dir: &str) -> gix_status::SymlinkCheck {
    gix_status::SymlinkCheck::new(
        crate::scripted_fixture_read_only("symlink_stack.sh")
            .expect("valid script")
            .join(dir),
    )
}

#[test]
fn paths_not_going_through_symlink_directories_are_ok_and_point_to_correct_item() -> crate::Result {
    for root in ["base", "symlink-base"] {
        let mut stack = stack_in(root);
        for (rela_path, expectation) in [
            ("root-filelink", is_symlink as fn(&std::fs::Metadata) -> bool),
            ("root-dirlink", is_symlinked_dir),
            ("file", is_file),
            ("dir/file-in-dir", is_file),
            ("dir", is_dir),
            ("dir/subdir", is_dir),
            ("dir/filelink", is_symlink),
            ("dir/dirlink", is_symlinked_dir),
        ] {
            assert!(
                expectation(&stack.verified_path(rela_path)?.symlink_metadata()?),
                "{rela_path:?} expectation failed"
            );
        }
    }
    Ok(())
}

#[test]
fn leaf_file_does_not_have_to_exist() -> crate::Result {
    assert!(!stack().verified_path("dir/does-not-exist")?.exists());
    Ok(())
}

#[test]
#[cfg(not(windows))]
fn intermediate_directories_have_to_exist_or_not_found_error() -> crate::Result {
    assert_eq!(
        stack().verified_path("nonexisting-dir/file").unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    Ok(())
}

#[test]
#[cfg(windows)]
fn intermediate_directories_do_not_have_exist_for_success() -> crate::Result {
    assert!(stack().verified_path("nonexisting-dir/file").is_ok());
    Ok(())
}

#[test]
#[cfg_attr(
    windows,
    ignore = "on windows, symlinks appear to be files or dirs, is_symlink() doesn't work"
)]
fn paths_leading_through_symlinks_are_rejected() {
    let mut stack = stack();
    assert_eq!(
        stack.verified_path("root-dirlink/file-in-dir").unwrap_err().kind(),
        std::io::ErrorKind::Other,
        "root-dirlink is a symlink to a directory"
    );

    assert_eq!(
        stack.verified_path("dir/dirlink/nothing").unwrap_err().kind(),
        std::io::ErrorKind::Other,
        "root-dirlink is a symlink to a directory"
    );
}

fn is_symlink(m: &std::fs::Metadata) -> bool {
    m.is_symlink()
}

fn is_symlinked_dir(m: &std::fs::Metadata) -> bool {
    m.is_symlink()
}
fn is_file(m: &std::fs::Metadata) -> bool {
    m.is_file()
}
fn is_dir(m: &std::fs::Metadata) -> bool {
    m.is_dir()
}
