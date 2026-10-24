use gix_index::entry::{Mode, mode::Change};

#[test]
fn is_dir_for_pattern_matching() {
    for (mode, expected) in [
        (Mode::DIR, true),
        (Mode::COMMIT, true),
        (Mode::FILE, false),
        (Mode::FILE_EXECUTABLE, false),
        (Mode::SYMLINK, false),
        (Mode::empty(), false),
    ] {
        assert_eq!(
            mode.is_dir_for_pattern_matching(),
            expected,
            "{mode:?}: decoded modes have the same classification on every host"
        );
    }
}

#[test]
fn apply() {
    assert_eq!(Change::ExecutableBit.apply(Mode::FILE), Mode::FILE_EXECUTABLE);
    assert_eq!(Change::ExecutableBit.apply(Mode::FILE_EXECUTABLE), Mode::FILE);
    assert_eq!(
        Change::Type {
            new_mode: Mode::SYMLINK
        }
        .apply(Mode::FILE),
        Mode::SYMLINK
    );
}

#[test]
fn debug() {
    assert_eq!(
        format!("{:?}", Mode::FILE),
        "Mode(FILE)",
        "Assure the debug output is easy to understand"
    );

    assert_eq!(
        format!("{:?}", Mode::from_bits(0o120744)),
        "Some(Mode(FILE | SYMLINK | 0x40))",
        "strange modes are also mostly legible"
    );
}

#[test]
fn observed_symlinks_retain_their_type() {
    for has_symlinks in [false, true] {
        for executable_bit in [false, true] {
            for mode in [Mode::FILE, Mode::FILE_EXECUTABLE, Mode::SYMLINK] {
                assert_eq!(
                    mode.change_to_match_fs_with_values(false, false, true, false, has_symlinks, executable_bit),
                    (mode != Mode::SYMLINK).then_some(Change::Type {
                        new_mode: Mode::SYMLINK
                    }),
                    "actual symlinks keep their type regardless of checkout capabilities"
                );
            }
            assert_eq!(
                Mode::SYMLINK.change_to_match_fs_with_values(true, false, false, false, has_symlinks, executable_bit),
                has_symlinks.then_some(Change::Type { new_mode: Mode::FILE }),
                "regular files emulate tracked symlinks only when symlink support is disabled"
            );
        }
    }
}

#[test]
fn executable_non_file_nodes_are_not_type_changes() {
    for is_executable in [false, true] {
        for executable_bit in [false, true] {
            assert_eq!(
                Mode::FILE_EXECUTABLE.change_to_match_fs_with_values(
                    false,
                    false,
                    false,
                    is_executable,
                    true,
                    executable_bit
                ),
                (executable_bit && !is_executable).then_some(Change::ExecutableBit),
                "FIFO/socket metadata retains executable-bit handling without inventing a regular-file type"
            );
        }
    }
}
