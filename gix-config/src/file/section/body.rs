use std::{borrow::Cow, iter::FusedIterator, ops::Range};

use bstr::{BStr, BString, ByteVec};

use crate::{
    parse::{section::ValueName, Event},
    value::{normalize, normalize_bstr, normalize_bstring},
};

/// A opaque type that represents a section body.
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Debug, Default)]
pub struct Body<'event>(pub(crate) Vec<Event<'event>>);

/// Access
impl<'event> Body<'event> {
    /// Retrieves the last matching value in a section with the given value name, if present.
    ///
    /// Note that we consider values without separator `=` non-existing, i.e. `[core]\na` would not exist.
    /// If that's expected, [Self::value_implicit()] must be used instead.
    #[must_use]
    pub fn value(&self, value_name: impl AsRef<str>) -> Option<Cow<'_, BStr>> {
        self.value_implicit(value_name.as_ref()).flatten()
    }

    /// Retrieves the last matching value in a section with the given value name, if present, and indicates
    /// an implicit value with `Some(None)`, and a non-existing one as `None`
    #[must_use]
    pub fn value_implicit(&self, value_name: &str) -> Option<Option<Cow<'_, BStr>>> {
        let key = ValueName::from_str_unchecked(value_name);
        let (_key_range, range) = self.key_and_value_range_by(&key)?;
        let range = match range {
            None => return Some(None),
            Some(range) => range,
        };
        let mut concatenated = BString::default();

        for event in &self.0[range] {
            match event {
                Event::Value(v) => {
                    return Some(Some(normalize_bstr(v.as_ref())));
                }
                Event::ValueNotDone(v) => {
                    concatenated.push_str(v.as_ref());
                }
                Event::ValueDone(v) => {
                    concatenated.push_str(v.as_ref());
                    return Some(Some(normalize_bstring(concatenated)));
                }
                _ => (),
            }
        }
        None
    }

    /// Retrieves all values that have the provided value name. This may return
    /// an empty vec, which implies there were no values with the provided key.
    #[must_use]
    pub fn values(&self, value_name: &str) -> Vec<Cow<'_, BStr>> {
        let key = &ValueName::from_str_unchecked(value_name);
        let mut values = Vec::new();
        let mut expect_value = false;
        let mut concatenated_value = BString::default();

        for event in &self.0 {
            match event {
                Event::SectionValueName(event_key) if event_key == key => expect_value = true,
                Event::Value(v) if expect_value => {
                    expect_value = false;
                    values.push(normalize_bstr(v.as_ref()));
                }
                Event::ValueNotDone(v) if expect_value => {
                    concatenated_value.push_str(v.as_ref());
                }
                Event::ValueDone(v) if expect_value => {
                    expect_value = false;
                    concatenated_value.push_str(v.as_ref());
                    values.push(normalize_bstring(std::mem::take(&mut concatenated_value)));
                }
                _ => (),
            }
        }

        values
    }

    /// Returns an iterator visiting all value names in order.
    pub fn value_names(&self) -> impl Iterator<Item = &ValueName<'event>> {
        self.0.iter().filter_map(|e| match e {
            Event::SectionValueName(k) => Some(k),
            _ => None,
        })
    }

    /// Returns true if the section contains the provided value name.
    #[must_use]
    pub fn contains_value_name(&self, value_name: &str) -> bool {
        let key = &ValueName::from_str_unchecked(value_name);
        self.0.iter().any(|e| {
            matches!(e,
                Event::SectionValueName(k) if k == key
            )
        })
    }

    /// Returns the number of values in the section.
    #[must_use]
    pub fn num_values(&self) -> usize {
        self.0
            .iter()
            .filter(|e| matches!(e, Event::SectionValueName(_)))
            .count()
    }

    /// Returns if the section is empty.
    /// Note that this may count whitespace, see [`num_values()`][Self::num_values()] for
    /// another way to determine semantic emptiness.
    #[must_use]
    pub fn is_void(&self) -> bool {
        self.0.is_empty()
    }
}

impl Body<'_> {
    pub(crate) fn as_ref(&self) -> &[Event<'_>] {
        &self.0
    }

    /// Returns the range containing the value events for the `value_name`, with value range being `None` if there is
    /// no key-value separator and only a 'fake' Value event with an empty string in side.
    /// If the value is not found, `None` is returned.
    pub(crate) fn key_and_value_range_by(
        &self,
        value_name: &ValueName<'_>,
    ) -> Option<(Range<usize>, Option<Range<usize>>)> {
        let mut value_range = Range::default();
        let mut key_start = None;
        for (i, e) in self.0.iter().enumerate().rev() {
            match e {
                Event::SectionValueName(k) => {
                    if k == value_name {
                        key_start = Some(i);
                        break;
                    }
                    value_range = Range::default();
                }
                Event::Value(_) => {
                    (value_range.start, value_range.end) = (i, i);
                }
                Event::ValueNotDone(_) | Event::ValueDone(_) => {
                    if value_range.end == 0 {
                        value_range.end = i;
                    } else {
                        value_range.start = i;
                    }
                }
                _ => (),
            }
        }
        key_start.map(|key_start| {
            // value end needs to be offset by one so that the last value's index
            // is included in the range
            #[allow(clippy::range_plus_one)]
            let value_range = value_range.start..value_range.end + 1;
            let key_range = key_start..value_range.end;
            (key_range, (value_range.start != key_start + 1).then_some(value_range))
        })
    }
}

/// An owning iterator of a section body. Created by [`Body::into_iter`], yielding
/// un-normalized (`key`, `value`) pairs.
// TODO: tests
pub struct BodyIter<'event>(std::vec::IntoIter<Event<'event>>);

impl<'event> IntoIterator for Body<'event> {
    type Item = (ValueName<'event>, Cow<'event, BStr>);

    type IntoIter = BodyIter<'event>;

    fn into_iter(self) -> Self::IntoIter {
        BodyIter(self.0.into_iter())
    }
}

impl<'event> Iterator for BodyIter<'event> {
    type Item = (ValueName<'event>, Cow<'event, BStr>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut key = None;
        let mut partial_value = BString::default();
        let mut value = None;

        for event in self.0.by_ref() {
            match event {
                Event::SectionValueName(k) => key = Some(k),
                Event::Value(v) => {
                    value = Some(v);
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

        key.zip(value.map(normalize))
    }
}

impl FusedIterator for BodyIter<'_> {}
