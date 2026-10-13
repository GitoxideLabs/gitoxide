use gix_error::{bail, validation};

use crate::{
    Result,
    bstr::{BString, ByteSlice},
};

/// Return the configured comment prefix for text processing without an editor.
///
/// Read `core.commentChar` and `core.commentString` in assignment order, rejecting empty values
/// and newlines. If absent or set to `auto`, return `#`, as `git stripspace` does. This does not
/// perform automatic prefix selection against editable message content.
///
/// Configuration errors are classified as validation and retain the key and input as metadata.
pub fn comment_prefix(config: &gix_config::File) -> Result<BString> {
    let mut prefix = BString::from("#");
    for section in config.sections() {
        if !section.header().name().eq_ignore_ascii_case(b"core") || section.header().subsection_name().is_some() {
            continue;
        }
        for (name, value) in section.body() {
            if !name.eq_ignore_ascii_case("commentChar") && !name.eq_ignore_ascii_case("commentString") {
                continue;
            }
            let key = format!("core.{}", name.to_ascii_lowercase());
            if value.is_empty() {
                // shortcut: implicit values use this diagnostic until section iteration exposes whether '=' was present.
                bail!(
                    validation(format!("{key} must have at least one character"))
                        .with("key", key)
                        .with_input(value.as_bstr())
                );
            }
            if value.contains(&b'\n') {
                bail!(
                    validation(format!("{key} cannot contain newline"))
                        .with("key", key)
                        .with_input(value.as_bstr())
                );
            }
            prefix = if value.eq_ignore_ascii_case(b"auto") {
                "#".into()
            } else {
                value
            };
        }
    }
    Ok(prefix)
}
