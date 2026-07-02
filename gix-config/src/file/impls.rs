use std::{fmt::Display, str::FromStr};

use bstr::{BStr, BString, ByteSlice, ByteVec};

use crate::{
    File,
    file::Metadata,
    parse,
    parse::{Event, section},
    value::normalize,
};

impl FromStr for File {
    type Err = parse::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse::Events::from_bytes_owned(s.as_bytes(), None)
            .map(|events| File::from_parse_events_no_includes(events, Metadata::api()))
    }
}

impl TryFrom<&str> for File {
    type Error = parse::Error;

    /// Convenience constructor. Attempts to parse the provided string into a
    /// [`File`]. See [`Events::from_str()`][crate::parse::Events::from_str()] for more information.
    fn try_from(s: &str) -> Result<File, Self::Error> {
        parse::Events::from_bytes_owned(s.as_bytes(), None)
            .map(|events| Self::from_parse_events_no_includes(events, Metadata::api()))
    }
}

impl TryFrom<&BStr> for File {
    type Error = parse::Error;

    /// Convenience constructor. Attempts to parse the provided byte string into
    /// a [`File`]. See [`Events::from_bytes()`][parse::Events::from_bytes()] for more information.
    fn try_from(value: &BStr) -> Result<File, Self::Error> {
        parse::Events::from_bytes_owned(value, None)
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
        fn find_key<'a>(mut it: impl Iterator<Item = &'a Event>) -> Option<&'a section::ValueName> {
            it.find_map(|e| match e {
                Event::SectionValueName(k) => Some(k),
                _ => None,
            })
        }
        fn collect_value<'a>(it: impl Iterator<Item = &'a Event>, backing: &[u8]) -> BString {
            let mut partial_value = BString::default();
            let mut value = None;

            for event in it {
                match event {
                    Event::SectionValueName(_) => break,
                    Event::Value(v) => {
                        value = Some(v.to_bstring_in(backing));
                        break;
                    }
                    Event::ValueNotDone(v) => partial_value.push_str(v.as_slice_in(backing)),
                    Event::ValueDone(v) => {
                        partial_value.push_str(v.as_slice_in(backing));
                        value = Some(partial_value);
                        break;
                    }
                    _ => (),
                }
            }
            value
                .as_ref()
                .map(|value| normalize(value.as_slice().as_bstr()))
                .unwrap_or_default()
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
                .0
                .as_bstr_in(&self.event_backing)
                .eq_ignore_ascii_case(rhs.header.name.0.as_bstr_in(&other.event_backing))
                || lhs
                    .header
                    .subsection_name
                    .as_ref()
                    .map(|name| name.as_bstr_in(&self.event_backing))
                    != rhs
                        .header
                        .subsection_name
                        .as_ref()
                        .map(|name| name.as_bstr_in(&other.event_backing))
            {
                return false;
            }

            let (mut lhs, mut rhs) = (lhs.body.0.iter(), rhs.body.0.iter());
            while let (Some(lhs_key), Some(rhs_key)) = (find_key(&mut lhs), find_key(&mut rhs)) {
                if !lhs_key
                    .0
                    .as_bstr_in(&self.event_backing)
                    .eq_ignore_ascii_case(rhs_key.0.as_bstr_in(&other.event_backing))
                {
                    return false;
                }
                if collect_value(&mut lhs, &self.event_backing) != collect_value(&mut rhs, &other.event_backing) {
                    return false;
                }
            }
        }
        true
    }
}
