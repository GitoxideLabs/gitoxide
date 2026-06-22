use std::ops::Range;

use bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_sec::Trust;
use smallvec::SmallVec;

use crate::{
    file::{
        self, Index, Section, SectionData, Size,
        mutable::{Whitespace, escape_value},
    },
    lookup, parse,
    parse::{Event, Span, section::ValueName},
    value::{normalize, normalize_bstr, normalize_bstring},
};

/// A opaque type that represents a mutable reference to a section.
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct SectionMut<'a> {
    section: &'a mut SectionData,
    backing: &'a mut Vec<u8>,
    implicit_newline: bool,
    whitespace: Whitespace,
    newline: SmallVec<[u8; 2]>,
}

/// Mutating methods.
impl SectionMut<'_> {
    /// Adds an entry to the end of this section name `value_name` and `value`. If `value` is `None`, no equal sign will be written leaving
    /// just the key. This is useful for boolean values which are true if merely the key exists.
    pub fn push(&mut self, value_name: ValueName, value: Option<&BStr>) -> &mut Self {
        self.push_with_comment_inner(value_name, value, None);
        self
    }

    /// Adds an entry to the end of this section name `value_name` and `value`. If `value` is `None`, no equal sign will be written leaving
    /// just the key. This is useful for boolean values which are true if merely the key exists.
    /// `comment` has to be the text to put right after the value and behind a `#` character. Note that newlines are silently transformed
    /// into spaces.
    pub fn push_with_comment<'b, 'c>(
        &mut self,
        value_name: ValueName,
        value: Option<&'b BStr>,
        comment: impl Into<&'c BStr>,
    ) -> &mut Self {
        self.push_with_comment_inner(value_name, value, comment.into().into());
        self
    }

    fn push_with_comment_inner(&mut self, value_name: ValueName, value: Option<&BStr>, comment: Option<&BStr>) {
        let body = &mut self.section.body.0;
        if let Some(ws) = &self.whitespace.pre_key {
            body.push(Event::Whitespace(Span::append(self.backing, ws)));
        }

        let mut value_name = value_name.to_owned();
        value_name.0.intern(self.backing);
        body.push(Event::SectionValueName(value_name));
        match value {
            Some(value) => {
                body.extend(self.whitespace.key_value_separators(self.backing));
                body.push(Event::Value(Span::append(self.backing, &escape_value(value))));
            }
            None => body.push(Event::Value(Span::append(self.backing, b""))),
        }
        if let Some(comment) = comment {
            body.push(Event::Whitespace(Span::append(self.backing, b" ")));
            body.push(Event::Comment(parse::Comment {
                tag: b'#',
                text: {
                    let mut c = Vec::with_capacity(comment.len());
                    let mut bytes = comment.iter().peekable();
                    if !bytes.peek().is_none_or(|b| b.is_ascii_whitespace()) {
                        c.insert(0, b' ');
                    }
                    c.extend(bytes.map(|b| if *b == b'\n' { b' ' } else { *b }));
                    Span::append(self.backing, &c)
                },
            }));
        }
        if self.implicit_newline {
            body.push(Event::Newline(Span::append(self.backing, &self.newline)));
        }
    }

    /// Removes all events until a key value pair is removed. This will also
    /// remove the whitespace preceding the key value pair, if any is found.
    pub fn pop(&mut self) -> Option<(ValueName, BString)> {
        let mut values: Vec<BString> = Vec::new();
        // events are popped in reverse order
        let body = &mut self.section.body.0;
        while let Some(e) = body.pop() {
            match e {
                Event::SectionValueName(k) => {
                    // pop leading whitespace
                    if let Some(Event::Whitespace(_)) = body.last() {
                        body.pop();
                    }

                    if values.len() == 1 {
                        let value = values.pop().expect("vec is non-empty but popped to empty value");
                        return Some((k, normalize(value.as_ref())));
                    }

                    return Some((
                        k,
                        normalize_bstring({
                            let mut s = BString::default();
                            for value in values.into_iter().rev() {
                                s.push_str(value.as_slice());
                            }
                            s
                        }),
                    ));
                }
                Event::Value(v) | Event::ValueNotDone(v) | Event::ValueDone(v) => {
                    values.push(v.to_bstring_in(self.backing));
                }
                _ => (),
            }
        }
        None
    }

    /// Sets the last key value pair if it exists, or adds the new value.
    /// Returns the previous value if it replaced a value, or None if it adds
    /// the value.
    pub fn set(&mut self, value_name: ValueName, value: &BStr) -> Option<BString> {
        let value_name = value_name.to_owned();
        match self.section.body.key_and_value_range_by_in(self.backing, &value_name) {
            None => {
                self.push(value_name, Some(value));
                None
            }
            Some((key_range, value_range)) => {
                let value_range = value_range.unwrap_or(key_range.end - 1..key_range.end);
                let range_start = value_range.start;
                let ret = self.remove_internal(value_range, false);
                self.section.body.0.insert(
                    range_start,
                    Event::Value(Span::append(self.backing, &escape_value(value))),
                );
                Some(ret)
            }
        }
    }

    /// Set the trust level in the meta-data of this section to `trust`.
    pub fn set_trust(&mut self, trust: Trust) -> &mut Self {
        let mut meta = (*self.section.meta).clone();
        meta.trust = trust;
        self.section.meta = meta.into();
        self
    }

    /// Removes the latest value by key and returns it, if it exists.
    pub fn remove(&mut self, value_name: &str) -> Option<BString> {
        let key = ValueName::from_str_unchecked(value_name);
        let (key_range, _value_range) = self.section.body.key_and_value_range_by_in(self.backing, &key)?;
        Some(self.remove_internal(key_range, true))
    }

    /// Adds a new line event. Note that you don't need to call this unless
    /// you've disabled implicit newlines.
    pub fn push_newline(&mut self) -> &mut Self {
        self.section
            .body
            .0
            .push(Event::Newline(Span::append(self.backing, &self.newline)));
        self
    }

    /// Return the newline used when calling [`push_newline()`][Self::push_newline()].
    pub fn newline(&self) -> &BStr {
        self.newline.as_slice().as_bstr()
    }

    /// Enables or disables automatically adding newline events after adding
    /// a value. This is _enabled by default_.
    pub fn set_implicit_newline(&mut self, on: bool) -> &mut Self {
        self.implicit_newline = on;
        self
    }

    /// Sets the exact whitespace to use before each newly created key-value pair,
    /// with only whitespace characters being permissible.
    ///
    /// The default is 2 tabs.
    /// Set to `None` to disable adding whitespace before a key value.
    ///
    /// # Panics
    ///
    /// If non-whitespace characters are used. This makes the method only suitable for validated
    /// or known input.
    pub fn set_leading_whitespace(&mut self, whitespace: Option<BString>) -> &mut Self {
        assert!(
            whitespace
                .as_deref()
                .is_none_or(|ws| ws.iter().all(u8::is_ascii_whitespace)),
            "input whitespace must only contain whitespace characters."
        );
        self.whitespace.pre_key = whitespace;
        self
    }

    /// Returns the whitespace this section will insert before the
    /// beginning of a key, if any.
    #[must_use]
    pub fn leading_whitespace(&self) -> Option<&BStr> {
        self.whitespace.pre_key.as_ref().map(|v| v.as_slice().as_bstr())
    }

    /// Return the immutable section view backing this mutable handle.
    pub fn section(&self) -> Section<'_> {
        Section::from_data(self.section, self.backing)
    }

    /// Return the header of the section backing this mutable handle.
    pub fn header(&self) -> file::section::Header<'_> {
        file::section::Header {
            header: &self.section.header,
            backing: self.backing,
        }
    }

    /// Return the unique `id` of the section backing this mutable handle.
    pub fn id(&self) -> file::SectionId {
        self.section.id
    }

    /// Return the section body backing this mutable handle.
    pub fn body(&self) -> file::section::BodyRef<'_> {
        file::section::BodyRef {
            body: &self.section.body,
            backing: self.backing,
        }
    }

    /// Serialize the section backing this mutable handle into a `BString`.
    #[must_use]
    pub fn to_bstring(&self) -> BString {
        self.section().to_bstring()
    }

    /// Return additional information about this section's origin.
    pub fn meta(&self) -> &file::Metadata {
        &self.section.meta
    }

    /// Retrieves the last matching value in this section with the given value name, if present.
    #[must_use]
    pub fn value(&self, value_name: impl AsRef<str>) -> Option<BString> {
        self.section
            .body
            .value_implicit_in(self.backing, value_name.as_ref())
            .flatten()
    }

    /// Retrieves the last matching value in this section, including implicit values.
    #[must_use]
    pub fn value_implicit(&self, value_name: &str) -> Option<Option<BString>> {
        self.section.body.value_implicit_in(self.backing, value_name)
    }

    /// Retrieves all values that have the provided value name.
    #[must_use]
    pub fn values(&self, value_name: &str) -> Vec<BString> {
        self.section.body.values_in(self.backing, value_name)
    }

    /// Returns an iterator visiting all value names in order.
    pub fn value_names(&self) -> impl Iterator<Item = ValueName> + '_ {
        self.section.body.as_ref().iter().filter_map(move |e| match e {
            Event::SectionValueName(k) => Some(ValueName(k.0.to_bstring_in(self.backing).into())),
            _ => None,
        })
    }

    /// Returns true if the section contains the provided value name.
    #[must_use]
    pub fn contains_value_name(&self, value_name: &str) -> bool {
        self.section.body.contains_value_name_in(self.backing, value_name)
    }

    /// Returns the number of values in the section.
    #[must_use]
    pub fn num_values(&self) -> usize {
        self.section.body.num_values()
    }

    /// Returns if the section is empty.
    #[must_use]
    pub fn is_void(&self) -> bool {
        self.section.body.is_void()
    }

    /// Returns the whitespace to be used before and after the `=` between the key
    /// and the value.
    ///
    /// For example, `k = v` will have `(Some(" "), Some(" "))`, whereas `k=\tv` will
    /// have `(None, Some("\t"))`.
    #[must_use]
    pub fn separator_whitespace(&self) -> (Option<&BStr>, Option<&BStr>) {
        (
            self.whitespace.pre_sep.as_ref().map(|v| v.as_slice().as_bstr()),
            self.whitespace.post_sep.as_ref().map(|v| v.as_slice().as_bstr()),
        )
    }
}

// Internal methods that may require exact indices for faster operations.
impl<'a> SectionMut<'a> {
    pub(crate) fn new(section: &'a mut SectionData, backing: &'a mut Vec<u8>, newline: SmallVec<[u8; 2]>) -> Self {
        let whitespace = Whitespace::from_body(&section.body, backing);
        Self {
            section,
            backing,
            implicit_newline: true,
            whitespace,
            newline,
        }
    }

    pub(crate) fn get(&self, key: &ValueName, start: Index, end: Index) -> Result<BString, lookup::existing::Error> {
        let mut expect_value = false;
        let mut concatenated_value = BString::default();

        for event in &self.section.body.0[start.0..end.0] {
            match event {
                Event::SectionValueName(event_key) if event_key.eq_ignore_ascii_case_in(self.backing, key) => {
                    expect_value = true;
                }
                Event::Value(v) if expect_value => return Ok(normalize_bstr(v.as_slice_in(self.backing).as_bstr())),
                Event::ValueNotDone(v) if expect_value => {
                    concatenated_value.push_str(v.as_slice_in(self.backing));
                }
                Event::ValueDone(v) if expect_value => {
                    concatenated_value.push_str(v.as_slice_in(self.backing));
                    return Ok(normalize_bstring(concatenated_value));
                }
                _ => (),
            }
        }

        Err(lookup::existing::Error::KeyMissing)
    }

    pub(crate) fn delete(&mut self, start: Index, end: Index) {
        self.section.body.0.drain(start.0..end.0);
    }

    pub(crate) fn set_internal(&mut self, index: Index, key: ValueName, value: &BStr) -> Size {
        let mut size = 0;

        let body = &mut self.section.body.0;
        body.insert(index.0, Event::Value(Span::append(self.backing, &escape_value(value))));
        size += 1;

        let sep_events = self.whitespace.key_value_separators(self.backing);
        size += sep_events.len();
        body.splice(index.0..index.0, sep_events.into_iter().rev())
            .for_each(|_| {});

        let mut key = key.to_owned();
        key.0.intern(self.backing);
        body.insert(index.0, Event::SectionValueName(key));
        size += 1;

        Size(size)
    }

    /// Performs the removal, assuming the range is valid.
    fn remove_internal(&mut self, range: Range<usize>, fix_whitespace: bool) -> BString {
        let events = &mut self.section.body.0;
        if fix_whitespace && events.get(range.end).is_some_and(|ev| matches!(ev, Event::Newline(_))) {
            events.remove(range.end);
        }
        let value = events.drain(range.clone()).fold(BString::default(), |mut acc, e| {
            if let Event::Value(v) | Event::ValueNotDone(v) | Event::ValueDone(v) = e {
                acc.push_str(v.as_slice_in(self.backing));
            }
            acc
        });
        if fix_whitespace
            && range
                .start
                .checked_sub(1)
                .and_then(|pos| events.get(pos))
                .is_some_and(|ev| matches!(ev, Event::Whitespace(_)))
        {
            events.remove(range.start - 1);
        }
        value
    }
}

impl file::section::Body {
    pub(crate) fn as_mut(&mut self) -> &mut Vec<Event> {
        &mut self.0
    }
}
