use gix_hash::ObjectId;

use crate::{log::Line, store_impl::file::log::LineRef};

impl LineRef<'_> {
    /// Convert this instance into its mutable counterpart
    pub fn to_owned(&self) -> Line {
        (*self).into()
    }
}

mod write {
    use std::io;

    use gix_object::bstr::{BStr, ByteSlice};

    use crate::log::Line;

    /// The Error produced by [`Line::write_to()`] (but wrapped in an io error).
    #[derive(Debug)]
    enum Error {
        IllegalCharacter,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::IllegalCharacter => f.write_str(r"Messages must not contain newlines (\n)"),
            }
        }
    }

    impl std::error::Error for Error {}

    impl From<Error> for io::Error {
        fn from(err: Error) -> Self {
            io::Error::other(err)
        }
    }

    /// Output
    impl Line {
        /// Serialize this instance to `out` in the git serialization format for ref log lines.
        pub fn write_to(&self, out: &mut dyn io::Write) -> io::Result<()> {
            write!(out, "{} {} ", self.previous_oid, self.new_oid)?;
            self.signature.write_to(out)?;
            writeln!(out, "\t{}", check_newlines(self.message.as_ref())?)
        }
    }

    fn check_newlines(input: &BStr) -> Result<&BStr, Error> {
        if input.find_byte(b'\n').is_some() {
            return Err(Error::IllegalCharacter);
        }
        Ok(input)
    }
}

impl LineRef<'_> {
    /// The previous object id of the ref. It will be a null hash if there was no previous id as
    /// this ref is being created.
    pub fn previous_oid(&self) -> ObjectId {
        ObjectId::from_hex(self.previous_oid).expect("parse validation")
    }
    /// The new object id of the ref, or a null hash if it is removed.
    pub fn new_oid(&self) -> ObjectId {
        ObjectId::from_hex(self.new_oid).expect("parse validation")
    }
}

impl<'a> From<LineRef<'a>> for Line {
    fn from(v: LineRef<'a>) -> Self {
        Line {
            previous_oid: v.previous_oid(),
            new_oid: v.new_oid(),
            signature: v.signature.into(),
            message: v.message.into(),
        }
    }
}

///
pub mod decode {
    use gix_object::bstr::{BStr, ByteSlice};

    use crate::{file::log::LineRef, parse::hex_hash_any};

    ///
    mod error {
        use gix_object::bstr::{BString, ByteSlice};

        /// The error returned by [`from_bytes(…)`][super::Line::from_bytes()]
        #[derive(Debug)]
        pub struct Error {
            pub input: BString,
        }

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    r"{:?} did not match '<old-hexsha> <new-hexsha> <name> <<email>> <timestamp> <tz>\t<message>'",
                    self.input
                )
            }
        }

        impl std::error::Error for Error {}

        impl Error {
            pub(crate) fn new(input: &[u8]) -> Self {
                Error {
                    input: input.as_bstr().to_owned(),
                }
            }
        }
    }
    pub use error::Error;

    impl<'a> LineRef<'a> {
        /// Decode a reflog line from the given bytes.
        ///
        /// Valid input starts with the previous object id, the new object id, a
        /// signature, and an optional tab-separated message, for example:
        ///
        /// `0123456789012345678901234567890123456789 89abcdef89abcdef89abcdef89abcdef89abcdef Name <name@example.com> 1700000000 +0000\tmessage`
        pub fn from_bytes(input: &'a [u8]) -> Result<LineRef<'a>, Error> {
            decode(input).map_err(|_| Error::new(first_line(input)))
        }
    }

    /// Return the first line from `input`, without its trailing newline.
    ///
    /// If `input` contains no newline, all of `input` is returned.
    fn first_line(input: &[u8]) -> &[u8] {
        let line_end = input.iter().position(|b| *b == b'\n').unwrap_or(input.len());
        &input[..line_end]
    }

    /// Parse one reflog line from `bytes`.
    ///
    /// Only one line is parsed; any bytes after the first newline are
    /// ignored. If the line has no tab separator, the message is empty.
    ///
    /// Return an error if the first line does not match the reflog line
    /// format.
    fn decode(bytes: &[u8]) -> Result<LineRef<'_>, ()> {
        let line = first_line(bytes);
        let (mut head, message) = match line.find_byte(b'\t') {
            Some(tab) => (&line[..tab], line[tab + 1..].as_bstr()),
            None => (line, BStr::new(b"")),
        };

        let old = hex_hash_any(&mut head)?;
        head = head.strip_prefix(b" ").ok_or(())?;
        let new = hex_hash_any(&mut head)?;
        head = head.strip_prefix(b" ").ok_or(())?;
        let signature = gix_actor::signature::decode(&mut head).map_err(|_| ())?;
        if !head.is_empty() {
            return Err(());
        }
        Ok(LineRef {
            previous_oid: old,
            new_oid: new,
            signature,
            message,
        })
    }

    #[cfg(test)]
    mod test_decode {
        use super::*;

        /// Convert a hexadecimal hash into its corresponding `ObjectId` or _panic_.
        fn hex_to_oid(hex: &str) -> gix_hash::ObjectId {
            gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
        }

        fn with_newline(mut v: Vec<u8>) -> Vec<u8> {
            v.push(b'\n');
            v
        }

        mod invalid {
            use super::decode;

            #[test]
            fn completely_bogus_shows_error_with_context() {
                let input = b"definitely not a log entry".as_slice();
                decode(input).expect_err("this should fail");
            }

            #[test]
            fn missing_whitespace_between_signature_and_message() {
                let line = "0000000000000000000000000000000000000000 0000000000000000000000000000000000000000 one <foo@example.com> 1234567890 -0000message";
                decode(line.as_bytes()).expect_err("this should fail");
            }
        }

        const NULL_SHA1: &[u8] = b"0000000000000000000000000000000000000000";

        #[test]
        fn entry_with_empty_message() {
            let line_without_nl: Vec<_> = b"0000000000000000000000000000000000000000 0000000000000000000000000000000000000000 name <foo@example.com> 1234567890 -0000".to_vec();
            let line_with_nl = with_newline(line_without_nl.clone());
            for input in &[line_without_nl, line_with_nl] {
                assert_eq!(
                    decode(input.as_slice()).expect("successful parsing"),
                    LineRef {
                        previous_oid: NULL_SHA1.as_bstr(),
                        new_oid: NULL_SHA1.as_bstr(),
                        signature: gix_actor::SignatureRef {
                            name: b"name".as_bstr(),
                            email: b"foo@example.com".as_bstr(),
                            time: "1234567890 -0000"
                        },
                        message: b"".as_bstr(),
                    }
                );
            }
        }

        #[test]
        fn entry_with_message_without_newline_and_with_newline() {
            let line_without_nl: Vec<_> = b"a5828ae6b52137b913b978e16cd2334482eb4c1f 89b43f80a514aee58b662ad606e6352e03eaeee4 Sebastian Thiel <foo@example.com> 1618030561 +0800\tpull --ff-only: Fast-forward".to_vec();
            let line_with_nl = with_newline(line_without_nl.clone());

            for input in &[line_without_nl, line_with_nl] {
                let res = decode(input.as_slice()).expect("successful parsing");
                let actual = LineRef {
                    previous_oid: b"a5828ae6b52137b913b978e16cd2334482eb4c1f".as_bstr(),
                    new_oid: b"89b43f80a514aee58b662ad606e6352e03eaeee4".as_bstr(),
                    signature: gix_actor::SignatureRef {
                        name: b"Sebastian Thiel".as_bstr(),
                        email: b"foo@example.com".as_bstr(),
                        time: "1618030561 +0800",
                    },
                    message: b"pull --ff-only: Fast-forward".as_bstr(),
                };
                assert_eq!(res, actual);
                assert_eq!(
                    actual.previous_oid(),
                    hex_to_oid("a5828ae6b52137b913b978e16cd2334482eb4c1f")
                );
                assert_eq!(actual.new_oid(), hex_to_oid("89b43f80a514aee58b662ad606e6352e03eaeee4"));
            }
        }

        #[test]
        fn two_lines_in_a_row_with_and_without_newline() {
            let lines = b"0000000000000000000000000000000000000000 0000000000000000000000000000000000000000 one <foo@example.com> 1234567890 -0000\t\n0000000000000000000000000000000000000000 0000000000000000000000000000000000000000 two <foo@example.com> 1234567890 -0000\thello";
            let parsed = decode(lines.as_slice()).expect("parse single line");
            assert_eq!(parsed.message, b"".as_bstr(), "first message is empty");
        }
    }
}
