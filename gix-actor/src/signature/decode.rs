pub(crate) mod function {
    use crate::{IdentityRef, SignatureRef};
    use bstr::ByteSlice;
    use winnow::error::ErrMode;
    use winnow::stream::Stream;
    use winnow::{
        combinator::{opt, separated_pair},
        error::{AddContext, ParserError, StrContext},
        prelude::*,
        stream::AsChar,
        token::take_while,
    };

    /// Parse a signature from the bytes input `i` using `nom`.
    pub fn decode<'a, E: ParserError<&'a [u8]> + AddContext<&'a [u8], StrContext>>(
        i: &mut &'a [u8],
    ) -> ModalResult<SignatureRef<'a>, E> {
        separated_pair(
            identity,
            opt(b" "),
            opt((
                take_while(0.., |b: u8| b == b'+' || b == b'-' || b.is_space() || b.is_dec_digit()).map(|v: &[u8]| v),
            ))
            .map(|maybe_bytes| {
                if let Some((bytes,)) = maybe_bytes {
                    bytes.into()
                } else {
                    b"".into()
                }
            }),
        )
        .context(StrContext::Expected("<name> <<email>> <timestamp> <+|-><HHMM>".into()))
        .map(|(identity, time)| SignatureRef {
            name: identity.name,
            email: identity.email,
            time,
        })
        .parse_next(i)
    }

    /// Parse an identity from the bytes input `i` (like `name <email>`) using `nom`.
    pub fn identity<'a, E: ParserError<&'a [u8]> + AddContext<&'a [u8], StrContext>>(
        i: &mut &'a [u8],
    ) -> ModalResult<IdentityRef<'a>, E> {
        let start = i.checkpoint();
        let eol_idx = i.find_byte(b'\n').unwrap_or(i.len());
        let right_delim_idx = i[..eol_idx]
            .rfind_byte(b'>')
            .ok_or(ErrMode::Cut(E::from_input(i).add_context(
                i,
                &start,
                StrContext::Label("Closing '>' not found"),
            )))?;
        let i_name_and_email = &i[..right_delim_idx];
        let skip_from_right = i_name_and_email.iter().rev().take_while(|b| **b == b'>').count();
        let left_delim_idx = i_name_and_email
            .find_byte(b'<')
            .ok_or(ErrMode::Cut(E::from_input(i).add_context(
                &i_name_and_email,
                &start,
                StrContext::Label("Opening '<' not found"),
            )))?;
        let skip_from_left = i[left_delim_idx..].iter().take_while(|b| **b == b'<').count();
        let mut name = i[..left_delim_idx].as_bstr();
        name = name.strip_suffix(b" ").unwrap_or(name).as_bstr();

        let email = i
            .get(left_delim_idx + skip_from_left..right_delim_idx - skip_from_right)
            .ok_or(ErrMode::Cut(E::from_input(i).add_context(
                &i_name_and_email,
                &start,
                StrContext::Label("Skipped parts run into each other"),
            )))?
            .as_bstr();
        *i = i.get(right_delim_idx + 1..).unwrap_or(&[]);
        Ok(IdentityRef { name, email })
    }
}
pub use function::identity;

#[cfg(test)]
mod tests {
    mod parse_signature {
        use bstr::ByteSlice;
        use gix_testtools::to_bstr_err;
        use winnow::prelude::*;

        use crate::{signature, SignatureRef};

        fn decode<'i>(
            i: &mut &'i [u8],
        ) -> ModalResult<SignatureRef<'i>, winnow::error::TreeError<&'i [u8], winnow::error::StrContext>> {
            signature::decode.parse_next(i)
        }

        fn signature(name: &'static str, email: &'static str, time: &'static str) -> SignatureRef<'static> {
            SignatureRef {
                name: name.as_bytes().as_bstr(),
                email: email.as_bytes().as_bstr(),
                time: time.as_bytes().as_bstr(),
            }
        }

        #[test]
        fn tz_minus() {
            assert_eq!(
                decode
                    .parse_peek(b"Sebastian Thiel <byronimo@gmail.com> 1528473343 -0230")
                    .expect("parse to work")
                    .1,
                signature("Sebastian Thiel", "byronimo@gmail.com", "1528473343 -0230")
            );
        }

        #[test]
        fn tz_plus() {
            assert_eq!(
                decode
                    .parse_peek(b"Sebastian Thiel <byronimo@gmail.com> 1528473343 +0230")
                    .expect("parse to work")
                    .1,
                signature("Sebastian Thiel", "byronimo@gmail.com", "1528473343 +0230")
            );
        }

        #[test]
        fn email_with_space() {
            assert_eq!(
                decode
                    .parse_peek(b"Sebastian Thiel <\tbyronimo@gmail.com > 1528473343 +0230")
                    .expect("parse to work")
                    .1,
                signature("Sebastian Thiel", "\tbyronimo@gmail.com ", "1528473343 +0230")
            );
        }

        #[test]
        fn negative_offset_0000() {
            assert_eq!(
                decode
                    .parse_peek(b"Sebastian Thiel <byronimo@gmail.com> 1528473343 -0000")
                    .expect("parse to work")
                    .1,
                signature("Sebastian Thiel", "byronimo@gmail.com", "1528473343 -0000")
            );
        }

        #[test]
        fn negative_offset_double_dash() {
            assert_eq!(
                decode
                    .parse_peek(b"name <name@example.com> 1288373970 --700")
                    .expect("parse to work")
                    .1,
                signature("name", "name@example.com", "1288373970 --700")
            );
        }

        #[test]
        fn empty_name_and_email() {
            assert_eq!(
                decode.parse_peek(b" <> 12345 -1215").expect("parse to work").1,
                signature("", "", "12345 -1215")
            );
        }

        #[test]
        fn invalid_signature() {
            assert_eq!(
                        decode.parse_peek(b"hello < 12345 -1215")
                            .map_err(to_bstr_err)
                            .expect_err("parse fails as > is missing")
                            .to_string(),
                        " at 'hello < 12345 -1215'\n  0: invalid Closing '>' not found at 'hello < 12345 -1215'\n  1: expected `<name> <<email>> <timestamp> <+|-><HHMM>` at 'hello < 12345 -1215'\n"
                    );
        }

        #[test]
        fn invalid_time() {
            assert_eq!(
                decode.parse_peek(b"hello <> abc -1215").expect("parse to work").1,
                signature("hello", "", "")
            );
        }
    }
}
