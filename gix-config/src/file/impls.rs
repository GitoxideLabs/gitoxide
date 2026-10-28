use std::{borrow::Cow, fmt::Display, str::FromStr};

use bstr::{BStr, BString, ByteVec};

use crate::{File, file::Metadata, parse, parse::Event, value::normalize};

impl FromStr for File {
    type Err = parse::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse::Events::from_bytes(s.as_bytes(), None)
            .map(|events| File::from_parse_events_no_includes(events, Metadata::api()))
    }
}

impl TryFrom<&str> for File {
    type Error = parse::Error;

    /// Convenience constructor. Attempts to parse the provided string into a
    /// [`File`]. See [`Events::from_str()`][crate::parse::Events::from_str()] for more information.
    fn try_from(s: &str) -> Result<File, Self::Error> {
        parse::Events::from_bytes(s.as_bytes(), None)
            .map(|events| Self::from_parse_events_no_includes(events, Metadata::api()))
    }
}

impl TryFrom<&BStr> for File {
    type Error = parse::Error;

    /// Convenience constructor. Attempts to parse the provided byte string into
    /// a [`File`]. See [`Events::from_bytes()`][parse::Events::from_bytes()] for more information.
    fn try_from(value: &BStr) -> Result<File, Self::Error> {
        parse::Events::from_bytes(value, None)
            .map(|events| Self::from_parse_events_no_includes(events, Metadata::api()))
    }
}

impl From<File> for BString {
    fn from(c: File) -> Self {
        c.to_bstring()
    }
}

impl Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.to_bstring(), f)
    }
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        fn find_key<'a>(mut it: impl Iterator<Item = &'a Event>) -> Option<&'a crate::parse::Span> {
            it.find_map(|e| match e {
                Event::SectionValueName(k) => Some(k),
                _ => None,
            })
        }
        fn collect_value<'a>(it: impl Iterator<Item = &'a Event>, backing: &'a [u8]) -> Cow<'a, BStr> {
            let mut partial_value = BString::default();

            for event in it {
                match event {
                    Event::SectionValueName(_) => break,
                    Event::Value(v) => return Cow::Borrowed(v.as_bstr_in(backing)),
                    Event::ValueNotDone(v) => partial_value.push_str(v.as_slice_in(backing)),
                    Event::ValueDone(v) => {
                        partial_value.push_str(v.as_slice_in(backing));
                        return Cow::Owned(partial_value);
                    }
                    _ => (),
                }
            }
            Cow::Borrowed(BStr::new(b""))
        }
        if self.section_order.len() != other.section_order.len() {
            return false;
        }

        for (lhs, rhs) in self
            .section_order
            .iter()
            .zip(&other.section_order)
            .map(|(lhs, rhs)| (&self.sections[lhs], &other.sections[rhs]))
        {
            if !lhs
                .header
                .name
                .as_bstr_in(&self.backing)
                .eq_ignore_ascii_case(rhs.header.name.as_bstr_in(&other.backing))
                || lhs.header.subsection_name(&self.backing) != rhs.header.subsection_name(&other.backing)
            {
                return false;
            }

            let (mut lhs, mut rhs) = (lhs.body.0.iter(), rhs.body.0.iter());
            while let (Some(lhs_key), Some(rhs_key)) = (find_key(&mut lhs), find_key(&mut rhs)) {
                if !lhs_key
                    .as_bstr_in(&self.backing)
                    .eq_ignore_ascii_case(rhs_key.as_bstr_in(&other.backing))
                {
                    return false;
                }
                let lhs_value = collect_value(&mut lhs, &self.backing);
                let rhs_value = collect_value(&mut rhs, &other.backing);
                if normalize(lhs_value.as_ref()) != normalize(rhs_value.as_ref()) {
                    return false;
                }
            }
        }
        true
    }
}

impl Eq for File {}
