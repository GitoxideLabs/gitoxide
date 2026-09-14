mod backoff;
mod bstr;
mod btoi;
mod buffers;
mod str;

#[test]
fn git_is_space() {
    for byte in u8::MIN..=u8::MAX {
        assert_eq!(
            gix_utils::git_is_space(byte),
            b" \t\n\r".contains(&byte),
            "only space, horizontal tab, newline and carriage return are Git whitespace, got {byte:#04x}"
        );
    }
}
