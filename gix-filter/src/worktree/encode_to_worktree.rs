pub(crate) mod function {
    use gix_encoding::Encoding;
    use gix_error::{Result, ResultExt, validation};

    /// Encode UTF-8 `src_utf8` for the working tree, replacing `buf` while retaining its allocation.
    ///
    /// Empty input stays empty, including for encodings which normally produce a BOM.
    /// Encoding errors leave `buf` empty, and no replacement characters are inserted.
    pub fn encode_to_worktree(src_utf8: &[u8], worktree_encoding: Encoding, buf: &mut Vec<u8>) -> Result {
        buf.clear();
        if src_utf8.is_empty() {
            return Ok(());
        }
        gix_encoding::convert_to(src_utf8, Encoding::Utf8, worktree_encoding, buf).or_raise(|| {
            validation(format!(
                "Could not encode UTF-8 input as '{}'",
                worktree_encoding.name()
            ))
        })
    }
}
