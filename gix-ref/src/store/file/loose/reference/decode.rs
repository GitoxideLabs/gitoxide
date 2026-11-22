use gix_error::{ErrorExt, Message, Result, ResultExt, bail};

use gix_hash::ObjectId;
use gix_object::bstr::{BStr, ByteSlice};

use crate::{FullName, FullNameRef, Target, parse::hex_hash, store_impl::file::loose::Reference};

/// The result of parsing a loose reference, before validating any symbolic target name.
///
/// Symbolic target bytes borrow from the file contents and are copied into owned storage only after validation.
enum ParsedTarget<'a> {
    /// A direct reference's object ID, decoded from hexadecimal for the configured hash kind.
    Id(ObjectId),
    /// A symbolic reference's target name, borrowed from the file contents and not yet validated.
    Symbolic(&'a BStr),
}

impl ParsedTarget<'_> {
    /// Invalid symbolic targets include [metadata](gix_error::Error::metadata()) `target` (bytes), as read from the
    /// reference contents.
    fn into_target(self, symbolic_target_buf: impl FnOnce() -> Vec<u8>) -> Result<Target> {
        Ok(match self {
            ParsedTarget::Id(id) => Target::Object(id),
            ParsedTarget::Symbolic(name) => Target::Symbolic(match gix_validate::reference::name(name) {
                Ok(name) => FullNameRef::new_unchecked(name).to_owned_with_buffer(&mut symbolic_target_buf()),
                Err(_) if name == "refs/heads/.invalid" => {
                    bail!(gix_error::unsupported(
                        "This reference uses an unsupported storage backend, such as reftable"
                    ));
                }
                Err(err) => {
                    bail!(err.and_raise(Message::new("Invalid symbolic reference target").with("target", name)));
                }
            }),
        })
    }
}

impl Reference {
    /// Create a new reference named `name` from the loose reference file contents in `path_contents`,
    /// parsing object ids as `object_hash`.
    ///
    /// Errors include [metadata](gix_error::Error::metadata()) `input` (bytes), the supplied reference contents.
    pub fn try_from_path(name: FullName, path_contents: &[u8], object_hash: gix_hash::Kind) -> Result<Self> {
        Ok(Reference {
            name,
            target: Self::decode_target(path_contents, object_hash, Vec::new)?,
        })
    }

    pub(in crate::store_impl::file) fn decode_target(
        path_contents: &[u8],
        object_hash: gix_hash::Kind,
        symbolic_target_buf: impl FnOnce() -> Vec<u8>,
    ) -> Result<Target> {
        parse(path_contents, object_hash)
            .map_err(|()| gix_error::corruption("Reference content could not be parsed").with_input(path_contents))?
            .into_target(symbolic_target_buf)
            .or_raise(|| Message::new("Could not decode reference").with_input(path_contents))
    }
}

/// Parse the contents of a loose reference file.
///
/// A *symbolic* reference starts with `ref: `, may have additional spaces before
/// the path, and returns [`ParsedTarget::Symbolic`] with the path
/// bytes up to the next NUL byte (just like Git), line ending, or the end of input. The path
/// is validated later when it is converted into a [`Target`].
///
/// A *direct* reference starts with a hexadecimal object id and returns
/// [`ParsedTarget::Id`].
///
/// If neither reference form can be parsed, an error is returned.
fn parse(mut i: &[u8], object_hash: gix_hash::Kind) -> std::result::Result<ParsedTarget<'_>, ()> {
    if let Some(rest) = i.strip_prefix(b"ref: ") {
        i = rest;
        while i.first() == Some(&b' ') {
            i = &i[1..];
        }
        let path_end = i
            .iter()
            .position(|b| *b == b'\0' || *b == b'\r' || *b == b'\n')
            .unwrap_or(i.len());
        Ok(ParsedTarget::Symbolic(i[..path_end].as_bstr()))
    } else {
        let hex = hex_hash(&mut i, object_hash)?;
        if i.first().is_some_and(u8::is_ascii_hexdigit) {
            return Err(());
        }
        Ok(ParsedTarget::Id(ObjectId::from_hex(hex).expect("prior validation")))
    }
}
