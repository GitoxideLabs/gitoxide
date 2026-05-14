use std::{
    borrow::Cow,
    ops::{Deref, Range},
};

use bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_sec::Trust;
use smallvec::SmallVec;

use crate::{
    file::{
        self, Index, Section, Size,
        mutable::{Whitespace, escape_value},
    },
    lookup, parse,
    parse::{Event, section::ValueName},
    value::{normalize, normalize_bstr, normalize_bstring},
};

/// A opaque type that represents a mutable reference to a section.
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct SectionMut<'a, 'event> {
    section: &'a mut Section<'event>,
    implicit_newline: bool,
    whitespace: Whitespace<'event>,
    newline: SmallVec<[u8; 2]>,
}

/// Mutating methods.
impl<'event> SectionMut<'_, 'event> {
    /// Adds an entry to the end of this section name `value_name` and `value`. If `value` is `None`, no equal sign will be written leaving
    /// just the key. This is useful for boolean values which are true if merely the key exists.
    pub fn push<'b>(&mut self, value_name: ValueName<'event>, value: Option<&'b BStr>) -> &mut Self {
        self.push_with_comment_inner(value_name, value, None);
        self
    }

    /// Adds an entry to the end of this section name `value_name` and `value`. If `value` is `None`, no equal sign will be written leaving
    /// just the key. This is useful for boolean values which are true if merely the key exists.
    /// `comment` has to be the text to put right after the value and behind a `#` character. Note that newlines are silently transformed
    /// into spaces.
    pub fn push_with_comment<'b, 'c>(
        &mut self,
        value_name: ValueName<'event>,
        value: Option<&'b BStr>,
        comment: impl Into<&'c BStr>,
    ) -> &mut Self {
        self.push_with_comment_inner(value_name, value, comment.into().into());
        self
    }

    fn push_with_comment_inner(&mut self, value_name: ValueName<'event>, value: Option<&BStr>, comment: Option<&BStr>) {
        let body = &mut self.section.body.0;
        if let Some(ws) = &self.whitespace.pre_key {
            body.push(Event::Whitespace(ws.clone()));
        }

        body.push(Event::SectionValueName(value_name));
        match value {
            Some(value) => {
                body.extend(self.whitespace.key_value_separators());
                body.push(Event::Value(escape_value(value).into()));
            }
            None => body.push(Event::Value(Cow::Borrowed("".into()))),
        }
        if let Some(comment) = comment {
            body.push(Event::Whitespace(Cow::Borrowed(" ".into())));
            body.push(Event::Comment(parse::Comment {
                tag: b'#',
                text: Cow::Owned({
                    let mut c = Vec::with_capacity(comment.len());
                    let mut bytes = comment.iter().peekable();
                    if !bytes.peek().is_none_or(|b| b.is_ascii_whitespace()) {
                        c.insert(0, b' ');
                    }
                    c.extend(bytes.map(|b| if *b == b'\n' { b' ' } else { *b }));
                    c.into()
                }),
            }));
        }
        if self.implicit_newline {
            body.push(Event::Newline(BString::from(self.newline.to_vec()).into()));
        }
    }

    /// Removes all events until a key value pair is removed. This will also
    /// remove the whitespace preceding the key value pair, if any is found.
    pub fn pop(&mut self) -> Option<(ValueName<'_>, Cow<'event, BStr>)> {
        let mut values = Vec::new();
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
                        return Some((k, normalize(value)));
                    }

                    return Some((
                        k,
                        normalize_bstring({
                            let mut s = BString::default();
                            for value in values.into_iter().rev() {
                                s.push_str(value.as_ref());
                            }
                            s
                        }),
                    ));
                }
                Event::Value(v) | Event::ValueNotDone(v) | Event::ValueDone(v) => values.push(v),
                _ => (),
            }
        }
        None
    }

    /// Sets the last key value pair if it exists, or adds the new value.
    /// Returns the previous value if it replaced a value, or None if it adds
    /// the value.
    pub fn set(&mut self, value_name: ValueName<'event>, value: &BStr) -> Option<Cow<'event, BStr>> {
        match self.key_and_value_range_by(&value_name) {
            None => {
                self.push(value_name, Some(value));
                None
            }
            Some((key_range, value_range)) => {
                let value_range = value_range.unwrap_or(key_range.end - 1..key_range.end);
                let range_start = value_range.start;
                let ret = self.remove_internal(value_range, false);
                self.section
                    .body
                    .0
                    .insert(range_start, Event::Value(escape_value(value).into()));
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
    pub fn remove(&mut self, value_name: &str) -> Option<Cow<'event, BStr>> {
        let key = ValueName::from_str_unchecked(value_name);
        let (key_range, _value_range) = self.key_and_value_range_by(&key)?;
        Some(self.remove_internal(key_range, true))
    }

    /// Adds a new line event. Note that you don't need to call this unless
    /// you've disabled implicit newlines.
    pub fn push_newline(&mut self) -> &mut Self {
        self.section
            .body
            .0
            .push(Event::Newline(Cow::Owned(BString::from(self.newline.to_vec()))));
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
    pub fn set_leading_whitespace(&mut self, whitespace: Option<Cow<'event, BStr>>) -> &mut Self {
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
        self.whitespace.pre_key.as_deref()
    }

    /// Returns the whitespace to be used before and after the `=` between the key
    /// and the value.
    ///
    /// For example, `k = v` will have `(Some(" "), Some(" "))`, whereas `k=\tv` will
    /// have `(None, Some("\t"))`.
    #[must_use]
    pub fn separator_whitespace(&self) -> (Option<&BStr>, Option<&BStr>) {
        (self.whitespace.pre_sep.as_deref(), self.whitespace.post_sep.as_deref())
    }
}

// Internal methods that may require exact indices for faster operations.
impl<'a, 'event> SectionMut<'a, 'event> {
    pub(crate) fn new(section: &'a mut Section<'event>, newline: SmallVec<[u8; 2]>) -> Self {
        let whitespace = Whitespace::from_body(&section.body);
        Self {
            section,
            implicit_newline: true,
            whitespace,
            newline,
        }
    }

    pub(crate) fn get(
        &self,
        key: &ValueName<'_>,
        start: Index,
        end: Index,
    ) -> Result<Cow<'_, BStr>, lookup::existing::Error> {
        let mut expect_value = false;
        let mut concatenated_value = BString::default();

        for event in &self.section.0[start.0..end.0] {
            match event {
                Event::SectionValueName(event_key) if event_key == key => expect_value = true,
                Event::Value(v) if expect_value => return Ok(normalize_bstr(v.as_ref())),
                Event::ValueNotDone(v) if expect_value => {
                    concatenated_value.push_str(v.as_ref());
                }
                Event::ValueDone(v) if expect_value => {
                    concatenated_value.push_str(v.as_ref());
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

    pub(crate) fn set_internal(&mut self, index: Index, key: ValueName<'event>, value: &BStr) -> Size {
        let mut size = 0;

        let body = &mut self.section.body.0;
        body.insert(index.0, Event::Value(escape_value(value).into()));
        size += 1;

        let sep_events = self.whitespace.key_value_separators();
        size += sep_events.len();
        body.splice(index.0..index.0, sep_events.into_iter().rev())
            .for_each(|_| {});

        body.insert(index.0, Event::SectionValueName(key));
        size += 1;

        Size(size)
    }

    /// Performs the removal, assuming the range is valid.
    fn remove_internal(&mut self, range: Range<usize>, fix_whitespace: bool) -> Cow<'event, BStr> {
        let events = &mut self.section.body.0;
        if fix_whitespace && events.get(range.end).is_some_and(|ev| matches!(ev, Event::Newline(_))) {
            events.remove(range.end);
        }
        let value = events
            .drain(range.clone())
            .fold(Cow::Owned(BString::default()), |mut acc: Cow<'_, BStr>, e| {
                if let Event::Value(v) | Event::ValueNotDone(v) | Event::ValueDone(v) = e {
                    acc.to_mut().extend(&**v);
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

impl<'event> Deref for SectionMut<'_, 'event> {
    type Target = file::Section<'event>;

    fn deref(&self) -> &Self::Target {
        self.section
    }
}

impl<'event> file::section::Body<'event> {
    pub(crate) fn as_mut(&mut self) -> &mut Vec<Event<'event>> {
        &mut self.0
    }
}
