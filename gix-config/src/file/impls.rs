use std::{borrow::Cow, fmt::Display, str::FromStr};

use bstr::{BStr, BString, ByteVec};

use crate::{
    File,
    file::Metadata,
    parse,
    parse::{Event, section},
    value::normalize,
};

impl FromStr for File<'static> {
    type Err = parse::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse::Events::from_bytes_owned(s.as_bytes(), None)
            .map(|events| File::from_parse_events_no_includes(events, Metadata::api()))
    }
}

impl<'a> TryFrom<&'a str> for File<'a> {
    type Error = parse::Error;

    /// Convenience constructor. Attempts to parse the provided string into a
    /// [`File`]. See [`Events::from_str()`][crate::parse::Events::from_str()] for more information.
    fn try_from(s: &'a str) -> Result<File<'a>, Self::Error> {
        parse::Events::from_str(s).map(|events| Self::from_parse_events_no_includes(events, Metadata::api()))
    }
}

impl<'a> TryFrom<&'a BStr> for File<'a> {
    type Error = parse::Error;

    /// Convenience constructor. Attempts to parse the provided byte string into
    /// a [`File`]. See [`Events::from_bytes()`][parse::Events::from_bytes()] for more information.
    fn try_from(value: &'a BStr) -> Result<File<'a>, Self::Error> {
        parse::Events::from_bytes(value, None)
            .map(|events| Self::from_parse_events_no_includes(events, Metadata::api()))
    }
}

impl From<File<'_>> for BString {
    fn from(c: File<'_>) -> Self {
        c.to_bstring()
    }
}

impl Display for File<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.to_bstring(), f)
    }
}

impl PartialEq for File<'_> {
    fn eq(&self, other: &Self) -> bool {
        fn find_key<'a>(mut it: impl Iterator<Item = &'a Event<'a>>) -> Option<&'a section::ValueName<'a>> {
            it.find_map(|e| match e {
                Event::SectionValueName(k) => Some(k),
                _ => None,
            })
        }
        fn collect_value<'a>(it: impl Iterator<Item = &'a Event<'a>>) -> Cow<'a, BStr> {
            let mut partial_value = BString::default();
            let mut value = None;

            for event in it {
                match event {
                    Event::SectionValueName(_) => break,
                    Event::Value(v) => {
                        value = v.clone().into();
                        break;
                    }
                    Event::ValueNotDone(v) => partial_value.push_str(v.as_ref()),
                    Event::ValueDone(v) => {
                        partial_value.push_str(v.as_ref());
                        value = Some(partial_value.into());
                        break;
                    }
                    _ => (),
                }
            }
            value.map(normalize).unwrap_or_default()
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
            if !(lhs.header.name == rhs.header.name && lhs.header.subsection_name == rhs.header.subsection_name) {
                return false;
            }

            let (mut lhs, mut rhs) = (lhs.body.0.iter(), rhs.body.0.iter());
            while let (Some(lhs_key), Some(rhs_key)) = (find_key(&mut lhs), find_key(&mut rhs)) {
                if lhs_key != rhs_key {
                    return false;
                }
                if collect_value(&mut lhs) != collect_value(&mut rhs) {
                    return false;
                }
            }
        }
        true
    }
}
