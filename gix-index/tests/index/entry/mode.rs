use gix_index::entry::{Mode, mode::Change};

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
