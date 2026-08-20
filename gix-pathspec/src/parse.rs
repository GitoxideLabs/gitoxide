use std::borrow::Cow;

use bstr::{BStr, BString, ByteSlice};
use gix_error::ValidationError;

use crate::{Defaults, MagicSignature, Pattern, SearchMode};

/// The error returned by [parse()][crate::parse()].
pub type Error = gix_error::ValidationError;

impl Pattern {
    /// Try to parse a path-spec pattern from the given `input` bytes.
    pub fn from_bytes(
        input: &[u8],
        Defaults {
            signature,
            search_mode,
            literal,
        }: Defaults,
    ) -> Result<Self, Error> {
        if input.is_empty() {
            return Err(ValidationError::new_with_input(
                "An empty string is not a valid pathspec",
                input,
            ));
        }
        if literal {
            return Ok(Self::from_literal(input, signature));
        }
        if input.as_bstr() == ":" {
            return Ok(Pattern {
                nil: true,
                ..Default::default()
            });
        }

        let mut p = Pattern {
            signature,
            search_mode: SearchMode::default(),
            ..Default::default()
        };

        let mut cursor = 0;
        if input.first() == Some(&b':') {
            cursor += 1;
            p.signature |= parse_short_keywords(input, &mut cursor)?;
            if let Some(b'(') = input.get(cursor) {
                cursor += 1;
                parse_long_keywords(input, &mut p, &mut cursor)?;
            }
        }

        if search_mode != Default::default() && p.search_mode == Default::default() {
            p.search_mode = search_mode;
        }
        let mut path = &input[cursor..];
        if path.last() == Some(&b'/') {
            p.signature |= MagicSignature::MUST_BE_DIR;
            path = &path[..path.len() - 1];
        }
        p.path = path.into();
        Ok(p)
    }

    /// Take `input` literally without parsing anything. This will also set our mode to `literal` to allow this pathspec to match `input` verbatim, and
    /// use `default_signature` as magic signature.
    pub fn from_literal(input: &[u8], default_signature: MagicSignature) -> Self {
        Pattern {
            path: input.into(),
            signature: default_signature,
            search_mode: SearchMode::Literal,
            ..Default::default()
        }
    }
}

fn parse_short_keywords(input: &[u8], cursor: &mut usize) -> Result<MagicSignature, Error> {
    let unimplemented_chars = b"\"#%&'-',;<=>@_`~";

    let mut signature = MagicSignature::empty();
    while let Some(&b) = input.get(*cursor) {
        *cursor += 1;
        signature |= match b {
            b'/' => MagicSignature::TOP,
            b'^' | b'!' => MagicSignature::EXCLUDE,
            b':' => break,
            _ if unimplemented_chars.contains(&b) => {
                return Err(ValidationError::new_with_input("Unimplemented short keyword", vec![b]));
            }
            _ => {
                *cursor -= 1;
                break;
            }
        }
    }

    Ok(signature)
}

fn parse_long_keywords(input: &[u8], p: &mut Pattern, cursor: &mut usize) -> Result<(), Error> {
    let end = input
        .find(")")
        .ok_or_else(|| ValidationError::new_with_input("Missing ')' at the end of pathspec signature", input))?;

    let input = &input[*cursor..end];
    *cursor = end + 1;

    if input.is_empty() {
        return Ok(());
    }

    split_on_non_escaped_char(input, b',', |keyword| {
        // Git skips empty keywords instead of rejecting them, so `:(top,)`, `:(,top)` and
        // `:(top,,icase)` are all valid there.
        if keyword.is_empty() {
            return Ok(());
        }
        let attr_prefix = b"attr:";
        match keyword {
            b"attr" => {}
            b"top" => p.signature |= MagicSignature::TOP,
            b"icase" => p.signature |= MagicSignature::ICASE,
            b"exclude" => p.signature |= MagicSignature::EXCLUDE,
            b"literal" => match p.search_mode {
                SearchMode::PathAwareGlob => {
                    return Err(ValidationError::new_with_input(
                        "'literal' and 'glob' keywords cannot be used together in the same pathspec",
                        keyword,
                    ));
                }
                _ => p.search_mode = SearchMode::Literal,
            },
            b"glob" => match p.search_mode {
                SearchMode::Literal => {
                    return Err(ValidationError::new_with_input(
                        "'literal' and 'glob' keywords cannot be used together in the same pathspec",
                        keyword,
                    ));
                }
                _ => p.search_mode = SearchMode::PathAwareGlob,
            },
            _ if keyword.starts_with(attr_prefix) => {
                if p.attributes.is_empty() {
                    p.attributes = parse_attributes(&keyword[attr_prefix.len()..])?;
                } else {
                    return Err(ValidationError::new_with_input(
                        "Only one attribute specification is allowed in the same pathspec",
                        keyword,
                    ));
                }
            }
            _ => {
                return Err(ValidationError::new_with_input(
                    "Found invalid keyword in pathspec signature",
                    keyword,
                ));
            }
        }
        Ok(())
    })
}

fn split_on_non_escaped_char(
    input: &[u8],
    split_char: u8,
    mut f: impl FnMut(&[u8]) -> Result<(), Error>,
) -> Result<(), Error> {
    // Mirrors `strcspn_escaped()` in Git's `pathspec.c`: a backslash consumes the byte that
    // follows it, so `\,` is a literal comma while `\\,` is an escaped backslash followed by a
    // separator. Scanning byte-by-byte also lets a separator at index 0 be seen, which a
    // two-byte window cannot.
    let mut i = 0;
    let mut last = 0;
    while i < input.len() {
        if input[i] == b'\\' {
            i += 2;
            continue;
        }
        if input[i] == split_char {
            f(&input[last..i])?;
            last = i + 1;
        }
        i += 1;
    }
    f(&input[last..])
}

fn parse_attributes(input: &[u8]) -> Result<Vec<gix_attributes::Assignment>, Error> {
    if input.is_empty() {
        return Err(ValidationError::new("Attribute specification cannot be empty"));
    }

    input
        .split(|&b| b == b' ')
        .filter(|attr| !attr.is_empty())
        .map(|attr| {
            let (name, state) = match attr.first() {
                Some(b'!') => (&attr[1..], gix_attributes::State::Unspecified),
                Some(b'-') => (&attr[1..], gix_attributes::State::Unset),
                _ => match attr.find_byte(b'=') {
                    Some(pos) => {
                        let (name, value) = attr.split_at(pos);
                        let value = &value[1..];
                        let value = if value.contains(&b'\\') {
                            Cow::Owned(unescape_and_check_attr_value(value.into())?)
                        } else {
                            check_attribute_value(value.into())?;
                            Cow::Borrowed(value.as_bstr())
                        };
                        (name, gix_attributes::StateRef::from_bytes(value.as_ref()).to_owned())
                    }
                    None => (attr, gix_attributes::State::Set),
                },
            };
            let name = gix_attributes::NameRef::try_from(name.as_bstr())?;
            Ok(gix_attributes::Assignment {
                name: name.to_owned(),
                state,
            })
        })
        .collect()
}

fn unescape_and_check_attr_value(value: &BStr) -> Result<BString, Error> {
    let mut out = BString::from(Vec::with_capacity(value.len()));
    let mut bytes = value.iter();
    while let Some(mut b) = bytes.next().copied() {
        if b == b'\\' {
            b = *bytes.next().ok_or_else(|| {
                ValidationError::new_with_input(
                    r"Escape character '\' is not allowed as the last character in an attribute value",
                    value,
                )
            })?;
        }

        out.push(validated_attr_value_byte(b)?);
    }
    Ok(out)
}

fn check_attribute_value(input: &BStr) -> Result<(), Error> {
    match input.iter().copied().find(|b| !is_valid_attr_value(*b)) {
        Some(b) => Err(ValidationError::new_with_input(
            "Invalid character in attribute value",
            vec![b],
        )),
        None => Ok(()),
    }
}

fn is_valid_attr_value(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b",-_".contains(&byte)
}

fn validated_attr_value_byte(byte: u8) -> Result<u8, Error> {
    if is_valid_attr_value(byte) {
        Ok(byte)
    } else {
        Err(ValidationError::new_with_input(
            "Invalid character in attribute value",
            vec![byte],
        ))
    }
}
