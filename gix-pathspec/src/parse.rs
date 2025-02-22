use std::borrow::Cow;

use bstr::{BStr, BString, ByteSlice, ByteVec};

use crate::{Defaults, MagicSignature, Pattern, SearchMode};

/// The error returned by [parse()][crate::parse()].
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("An empty string is not a valid pathspec")]
    EmptyString,
    #[error("Found {keyword:?} in signature, which is not a valid keyword")]
    InvalidKeyword { keyword: BString },
    #[error("Unimplemented short keyword: {short_keyword:?}")]
    Unimplemented { short_keyword: char },
    #[error("Missing ')' at the end of pathspec signature")]
    MissingClosingParenthesis,
    #[error("Attribute has non-ascii characters or starts with '-': {attribute:?}")]
    InvalidAttribute { attribute: BString },
    #[error("Invalid character in attribute value: {character:?}")]
    InvalidAttributeValue { character: char },
    #[error("Escape character '\\' is not allowed as the last character in an attribute value")]
    TrailingEscapeCharacter,
    #[error("Attribute specification cannot be empty")]
    EmptyAttribute,
    #[error("Only one attribute specification is allowed in the same pathspec")]
    MultipleAttributeSpecifications,
    #[error("'literal' and 'glob' keywords cannot be used together in the same pathspec")]
    IncompatibleSearchModes,
}

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
            return Err(Error::EmptyString);
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
                return Err(Error::Unimplemented {
                    short_keyword: b.into(),
                });
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
    let end = input.find(")").ok_or(Error::MissingClosingParenthesis)?;

    let input = &input[*cursor..end];
    *cursor = end + 1;

    if input.is_empty() {
        return Ok(());
    }

    split_on_non_escaped_char(input, b',', |keyword| {
        let attr_prefix = b"attr:";
        match keyword {
            b"attr" => {}
            b"top" => p.signature |= MagicSignature::TOP,
            b"icase" => p.signature |= MagicSignature::ICASE,
            b"exclude" => p.signature |= MagicSignature::EXCLUDE,
            b"literal" => match p.search_mode {
                SearchMode::PathAwareGlob => return Err(Error::IncompatibleSearchModes),
                _ => p.search_mode = SearchMode::Literal,
            },
            b"glob" => match p.search_mode {
                SearchMode::Literal => return Err(Error::IncompatibleSearchModes),
                _ => p.search_mode = SearchMode::PathAwareGlob,
            },
            _ if keyword.starts_with(attr_prefix) => {
                if p.attributes.is_empty() {
                    p.attributes = parse_attributes(&keyword[attr_prefix.len()..])?;
                } else {
                    return Err(Error::MultipleAttributeSpecifications);
                }
            }
            _ => {
                return Err(Error::InvalidKeyword {
                    keyword: BString::from(keyword),
                });
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
    let mut i = 0;
    let mut last = 0;
    for window in input.windows(2) {
        i += 1;
        if window[0] != b'\\' && window[1] == split_char {
            let keyword = &input[last..i];
            f(keyword)?;
            last = i + 1;
        }
    }
    let last_keyword = &input[last..];
    f(last_keyword)
}

fn parse_attributes(input: &[u8]) -> Result<Vec<gix_attributes::Assignment>, Error> {
    if input.is_empty() {
        return Err(Error::EmptyAttribute);
    }

    let unescaped = unescape_attribute_values(input.into())?;

    gix_attributes::parse::Iter::new(unescaped.as_bstr())
        .map(|res| res.map(gix_attributes::AssignmentRef::to_owned))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| Error::InvalidAttribute { attribute: e.attribute })
}

fn unescape_attribute_values(input: &BStr) -> Result<Cow<'_, BStr>, Error> {
    if !input.contains(&b'=') {
        return Ok(Cow::Borrowed(input));
    }

    let mut out: Cow<'_, BStr> = Cow::Borrowed("".into());

    for attr in input.split(|&c| c == b' ') {
        let split_point = attr.find_byte(b'=').map_or_else(|| attr.len(), |i| i + 1);
        let (name, value) = attr.split_at(split_point);

        if value.contains(&b'\\') {
            let out = out.to_mut();
            out.push_str(name);
            out.push_str(unescape_and_check_attr_value(value.into())?);
            out.push(b' ');
        } else {
            check_attribute_value(value.as_bstr())?;
            match out {
                Cow::Borrowed(_) => {
                    let end = out.len() + attr.len() + 1;
                    out = Cow::Borrowed(&input[0..end.min(input.len())]);
                }
                Cow::Owned(_) => {
                    let out = out.to_mut();
                    out.push_str(name);
                    out.push_str(value);
                    out.push(b' ');
                }
            }
        }
    }

    Ok(out)
}

fn unescape_and_check_attr_value(value: &BStr) -> Result<BString, Error> {
    let mut out = BString::from(Vec::with_capacity(value.len()));
    let mut bytes = value.iter();
    while let Some(mut b) = bytes.next().copied() {
        if b == b'\\' {
            b = *bytes.next().ok_or(Error::TrailingEscapeCharacter)?;
        }

        out.push(validated_attr_value_byte(b)?);
    }
    Ok(out)
}

fn check_attribute_value(input: &BStr) -> Result<(), Error> {
    match input.iter().copied().find(|b| !is_valid_attr_value(*b)) {
        Some(b) => Err(Error::InvalidAttributeValue { character: b as char }),
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
        Err(Error::InvalidAttributeValue {
            character: byte as char,
        })
    }
}
