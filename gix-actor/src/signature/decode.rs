pub(crate) mod function {
    use bstr::ByteSlice;
    use gix_error::OptionExt;
    use gix_error::Result;
    use gix_error::validation;

    use crate::{IdentityRef, SignatureRef};

    /// Parse a signature from the bytes input `i`, and change it to point to the unparsed bytes afterwards.
    pub fn decode<'a>(i: &mut &'a [u8]) -> Result<SignatureRef<'a>> {
        let identity = identity(i)?;
        if i.first() == Some(&b' ') {
            *i = &i[1..];
        }

        let time_len = i.iter().position(|b| !is_time_byte(*b)).unwrap_or(i.len());
        let (time, rest) = i.split_at(time_len);
        *i = rest;
        // SAFETY: The parser validated that there are only ASCII characters with `is_time_byte()`.
        #[expect(unsafe_code)]
        let time = unsafe { std::str::from_utf8_unchecked(time) };

        Ok(SignatureRef {
            name: identity.name,
            email: identity.email,
            time,
        })
    }

    /// Parse an identity from the bytes input `i` (like `name <email>`).
    pub fn identity<'a>(i: &mut &'a [u8]) -> Result<IdentityRef<'a>> {
        let eol_idx = i.find_byte(b'\n').unwrap_or(i.len());
        let right_delim_idx = i[..eol_idx]
            .rfind_byte(b'>')
            .ok_or_raise(|| validation("Closing '>' not found"))?;
        let i_name_and_email = &i[..right_delim_idx];
        let skip_from_right = i_name_and_email.iter().rev().take_while(|b| **b == b'>').count();
        let left_delim_idx = i_name_and_email
            .find_byte(b'<')
            .ok_or_raise(|| validation("Opening '<' not found"))?;
        let skip_from_left = i[left_delim_idx..].iter().take_while(|b| **b == b'<').count();
        let mut name = i[..left_delim_idx].as_bstr();
        name = name.strip_suffix(b" ").unwrap_or(name).as_bstr();

        let email = i
            .get(left_delim_idx + skip_from_left..right_delim_idx - skip_from_right)
            .ok_or_raise(|| validation("Skipped parts run into each other"))?
            .as_bstr();
        *i = i.get(right_delim_idx + 1..).unwrap_or(&[]);
        Ok(IdentityRef { name, email })
    }

    fn is_time_byte(b: u8) -> bool {
        matches!(b, b'+' | b'-' | b'0'..=b'9' | b' ' | b'\t')
    }
}
pub use function::identity;

#[cfg(test)]
mod tests {
    mod parse_signature {
        use gix_error::Result;

        use crate::SignatureRef;

        fn decode(mut i: &[u8]) -> Result<(&[u8], SignatureRef<'_>)> {
            SignatureRef::from_bytes_consuming(&mut i).map(|signature| (i, signature))
        }

        fn signature(name: &'static str, email: &'static str, time: &'static str) -> SignatureRef<'static> {
            SignatureRef {
                name: name.into(),
                email: email.into(),
                time,
            }
        }

        #[test]
        fn tz_minus() {
            let actual = decode(b"Sebastian Thiel <byronimo@gmail.com> 1528473343 -0230")
                .expect("parse to work")
                .1;
            assert_eq!(
                actual,
                signature("Sebastian Thiel", "byronimo@gmail.com", "1528473343 -0230")
            );
            assert_eq!(actual.seconds(), 1528473343);
            assert_eq!(
                actual.time().expect("valid"),
                gix_date::Time {
                    seconds: 1528473343,
                    offset: -9000,
                }
            );
        }

        #[test]
        fn tz_plus() {
            assert_eq!(
                decode(b"Sebastian Thiel <byronimo@gmail.com> 1528473343 +0230")
                    .expect("parse to work")
                    .1,
                signature("Sebastian Thiel", "byronimo@gmail.com", "1528473343 +0230")
            );
        }

        #[test]
        fn email_with_space() {
            assert_eq!(
                decode(b"Sebastian Thiel <\tbyronimo@gmail.com > 1528473343 +0230")
                    .expect("parse to work")
                    .1,
                signature("Sebastian Thiel", "\tbyronimo@gmail.com ", "1528473343 +0230")
            );
        }

        #[test]
        fn negative_offset_0000() {
            assert_eq!(
                decode(b"Sebastian Thiel <byronimo@gmail.com> 1528473343 -0000")
                    .expect("parse to work")
                    .1,
                signature("Sebastian Thiel", "byronimo@gmail.com", "1528473343 -0000")
            );
        }

        #[test]
        fn negative_offset_double_dash() {
            assert_eq!(
                decode(b"name <name@example.com> 1288373970 --700")
                    .expect("parse to work")
                    .1,
                signature("name", "name@example.com", "1288373970 --700")
            );
        }

        #[test]
        fn empty_name_and_email() {
            assert_eq!(
                decode(b" <> 12345 -1215").expect("parse to work").1,
                signature("", "", "12345 -1215")
            );
        }

        #[test]
        fn invalid_signature() {
            insta::assert_debug_snapshot!(decode(b"hello < 12345 -1215")
                    .expect_err("parse fails as > is missing"), "invalid signature", @"Closing '>' not found");
        }

        #[test]
        fn invalid_time() {
            assert_eq!(
                decode(b"hello <> abc -1215").expect("parse to work").1,
                signature("hello", "", "")
            );
        }
    }
}
