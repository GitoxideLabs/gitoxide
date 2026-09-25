use gix_error::Result;
use gix_error::{ErrorExt, Exn, Message, ResultExt, message};

use gix_hash::ObjectId;
use gix_object::bstr::BString;

use crate::{FullName, Target, parse::hex_hash, store_impl::file::loose::Reference};

enum MaybeUnsafeState {
    Id(ObjectId),
    UnvalidatedPath(BString),
}

impl TryFrom<MaybeUnsafeState> for Target {
    type Error = Exn;

    /// Invalid symbolic targets include [metadata](gix_error::Error::metadata()) `target` (bytes), as read from the
    /// reference contents.
    fn try_from(v: MaybeUnsafeState) -> std::result::Result<Self, Self::Error> {
        Ok(match v {
            MaybeUnsafeState::Id(id) => Target::Object(id),
            MaybeUnsafeState::UnvalidatedPath(name) => {
                Target::Symbolic(match gix_validate::reference::name(name.as_ref()) {
                    Ok(_) => FullName(name),
                    Err(_) if name == "refs/heads/.invalid" => {
                        return Err(
                            message("This reference uses an unsupported storage backend, such as reftable")
                                .raise_erased(),
                        );
                    }
                    Err(err) => {
                        return Err(err
                            .and_raise(Message::new("Invalid symbolic reference target").with("target", name))
                            .erased());
                    }
                })
            }
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
            target: Target::try_from(parse(path_contents, object_hash).map_err(|()| {
                gix_error::corruption("Reference content could not be parsed")
                    .with("input", path_contents)
                    .raise_erased()
            })?)
            .or_raise_erased(|| Message::new("Could not decode reference").with("input", path_contents))?,
        })
    }
}

/// Parse the contents of a loose reference file.
///
/// A *symbolic* reference starts with `ref: `, may have additional spaces before
/// the path, and returns [`MaybeUnsafeState::UnvalidatedPath`] with the path
/// bytes up to the next NUL byte (just like Git), line ending, or the end of input. The path
/// is validated later when it is converted into a [`Target`].
///
/// A *direct* reference starts with a hexadecimal object id and returns
/// [`MaybeUnsafeState::Id`].
///
/// If neither reference form can be parsed, an error is returned.
fn parse(mut i: &[u8], object_hash: gix_hash::Kind) -> std::result::Result<MaybeUnsafeState, ()> {
    if let Some(rest) = i.strip_prefix(b"ref: ") {
        i = rest;
        while i.first() == Some(&b' ') {
            i = &i[1..];
        }
        let path_end = i
            .iter()
            .position(|b| *b == b'\0' || *b == b'\r' || *b == b'\n')
            .unwrap_or(i.len());
        let path = i[..path_end].into();
        Ok(MaybeUnsafeState::UnvalidatedPath(path))
    } else {
        let hex = hex_hash(&mut i, object_hash)?;
        if i.first().is_some_and(u8::is_ascii_hexdigit) {
            return Err(());
        }
        Ok(MaybeUnsafeState::Id(ObjectId::from_hex(hex).expect("prior validation")))
    }
}
