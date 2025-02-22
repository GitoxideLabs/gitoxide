use smallvec::SmallVec;

use crate::{
    parse,
    parse::{Event, Section},
};

/// A type store without allocation all events that are typically preceding the first section.
pub type FrontMatterEvents<'a> = SmallVec<[Event<'a>; 8]>;

/// A zero-copy `git-config` file parser.
///
/// This is parser exposes low-level syntactic events from a `git-config` file.
/// Generally speaking, you'll want to use [`File`] as it wraps
/// around the parser to provide a higher-level abstraction to a `git-config`
/// file, including querying, modifying, and updating values.
///
/// This parser guarantees that the events emitted are sufficient to
/// reconstruct a `git-config` file identical to the source `git-config`
/// when writing it.
///
/// # Differences between a `.ini` parser
///
/// While the `git-config` format closely resembles the [`.ini` file format],
/// there are subtle differences that make them incompatible. For one, the file
/// format is not well defined, and there exists no formal specification to
/// adhere to.
///
/// For concrete examples, some notable differences are:
/// - `git-config` sections permit subsections via either a quoted string
///   (`[some-section "subsection"]`) or via the deprecated dot notation
///   (`[some-section.subsection]`). Successful parsing these section names is not
///   well defined in typical `.ini` parsers. This parser will handle these cases
///   perfectly.
/// - Comment markers are not strictly defined either. This parser will always
///   and only handle a semicolon or octothorpe (also known as a hash or number
///   sign).
/// - Global properties may be allowed in `.ini` parsers, but is strictly
///   disallowed by this parser.
/// - Only `\t`, `\n`, `\b` `\\` are valid escape characters.
/// - Quoted and semi-quoted values will be parsed (but quotes will be included
///   in event outputs). An example of a semi-quoted value is `5"hello world"`,
///   which should be interpreted as `5hello world` after
///   [normalization][crate::value::normalize()].
/// - Line continuations via a `\` character is supported (inside or outside of quotes)
/// - Whitespace handling similarly follows the `git-config` specification as
///   closely as possible, where excess whitespace after a non-quoted value are
///   trimmed, and line continuations onto a new line with excess spaces are kept.
/// - Only equal signs (optionally padded by spaces) are valid name/value
///   delimiters.
///
/// Note that things such as case-sensitivity or duplicate sections are
/// _not_ handled. This parser is a low level _syntactic_ interpreter
/// and higher level wrappers around this parser, which may
/// or may not be zero-copy, should handle _semantic_ values. This also means
/// that string-like values are not interpreted. For example, `hello"world"`
/// would be read at a high level as `helloworld` but this parser will return
/// the former instead, with the extra quotes. This is because it is not the
/// responsibility of the parser to interpret these values, and doing so would
/// necessarily require a copy, which this parser avoids.
///
/// # Trait Implementations
///
/// - This struct does _not_ implement [`FromStr`] due to lifetime
///   constraints implied on the required `from_str` method. Instead, it provides
///   [`From<&'_ str>`].
///
/// # Idioms
///
/// If you do want to use this parser, there are some idioms that may help you
/// with interpreting sequences of events.
///
/// ## `Value` events do not immediately follow `Key` events
///
/// Consider the following `git-config` example:
///
/// ```text
/// [core]
///   autocrlf = input
/// ```
///
/// Because this parser guarantees perfect reconstruction, there are many
/// non-significant events that occur in addition to the ones you may expect:
///
/// ```
/// # use gix_config::parse::{Event, Events, section};
/// # use std::borrow::Cow;
/// # use std::convert::TryFrom;
/// # let section_header = section::Header::new("core", None).unwrap();
/// # let section_data = "[core]\n  autocrlf = input";
/// # assert_eq!(Events::from_str(section_data).unwrap().into_vec(), vec![
/// Event::SectionHeader(section_header),
/// Event::Newline(Cow::Borrowed("\n".into())),
/// Event::Whitespace(Cow::Borrowed("  ".into())),
/// Event::SectionValueName(section::ValueName::try_from("autocrlf")?),
/// Event::Whitespace(Cow::Borrowed(" ".into())),
/// Event::KeyValueSeparator,
/// Event::Whitespace(Cow::Borrowed(" ".into())),
/// Event::Value(Cow::Borrowed("input".into())),
/// # ]);
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
///
/// Note the two whitespace events between the key and value pair! Those two
/// events actually refer to the whitespace between the name and value and the
/// equal sign. So if the config instead had `autocrlf=input`, those whitespace
/// events would no longer be present.
///
/// ## `KeyValueSeparator` event is not guaranteed to emit
///
/// Consider the following `git-config` example:
///
/// ```text
/// [core]
///   autocrlf
/// ```
///
/// This is a valid config with a `autocrlf` key having an implicit `true`
/// value. This means that there is not a `=` separating the key and value,
/// which means that the corresponding event won't appear either:
///
/// ```
/// # use gix_config::parse::{Event, Events, section};
/// # use std::borrow::Cow;
/// # use std::convert::TryFrom;
/// # let section_header = section::Header::new("core", None).unwrap();
/// # let section_data = "[core]\n  autocrlf";
/// # assert_eq!(Events::from_str(section_data).unwrap().into_vec(), vec![
/// Event::SectionHeader(section_header),
/// Event::Newline(Cow::Borrowed("\n".into())),
/// Event::Whitespace(Cow::Borrowed("  ".into())),
/// Event::SectionValueName(section::ValueName::try_from("autocrlf")?),
/// Event::Value(Cow::Borrowed("".into())),
/// # ]);
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
///
/// ## Quoted values are not unquoted
///
/// Consider the following `git-config` example:
///
/// ```text
/// [core]
/// autocrlf=true""
/// filemode=fa"lse"
/// ```
///
/// Both these events, when fully processed, should normally be `true` and
/// `false`. However, because this parser is zero-copy, we cannot process
/// partially quoted values, such as the `false` example. As a result, to
/// maintain consistency, the parser will just take all values as literals. The
/// relevant event stream emitted is thus emitted as:
///
/// ```
/// # use gix_config::parse::{Event, Events, section};
/// # use std::borrow::Cow;
/// # use std::convert::TryFrom;
/// # let section_header = section::Header::new("core", None).unwrap();
/// # let section_data = "[core]\nautocrlf=true\"\"\nfilemode=fa\"lse\"";
/// # assert_eq!(Events::from_str(section_data).unwrap().into_vec(), vec![
/// Event::SectionHeader(section_header),
/// Event::Newline(Cow::Borrowed("\n".into())),
/// Event::SectionValueName(section::ValueName::try_from("autocrlf")?),
/// Event::KeyValueSeparator,
/// Event::Value(Cow::Borrowed(r#"true"""#.into())),
/// Event::Newline(Cow::Borrowed("\n".into())),
/// Event::SectionValueName(section::ValueName::try_from("filemode")?),
/// Event::KeyValueSeparator,
/// Event::Value(Cow::Borrowed(r#"fa"lse""#.into())),
/// # ]);
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
///
/// ## Whitespace after line continuations are part of the value
///
/// Consider the following `git-config` example:
///
/// ```text
/// [some-section]
/// file=a\
///     c
/// ```
///
/// Because how `git-config` treats continuations, the whitespace preceding `c`
/// are in fact part of the value of `file`. The fully interpreted key/value
/// pair is actually `file=a    c`. As a result, the parser will provide this
/// split value accordingly:
///
/// ```
/// # use gix_config::parse::{Event, Events, section};
/// # use std::borrow::Cow;
/// # use std::convert::TryFrom;
/// # let section_header = section::Header::new("some-section", None).unwrap();
/// # let section_data = "[some-section]\nfile=a\\\n    c";
/// # assert_eq!(Events::from_str(section_data).unwrap().into_vec(), vec![
/// Event::SectionHeader(section_header),
/// Event::Newline(Cow::Borrowed("\n".into())),
/// Event::SectionValueName(section::ValueName::try_from("file")?),
/// Event::KeyValueSeparator,
/// Event::ValueNotDone(Cow::Borrowed("a".into())),
/// Event::Newline(Cow::Borrowed("\n".into())),
/// Event::ValueDone(Cow::Borrowed("    c".into())),
/// # ]);
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
///
/// [`File`]: crate::File
/// [`.ini` file format]: https://en.wikipedia.org/wiki/INI_file
/// [`git`'s documentation]: https://git-scm.com/docs/git-config#_configuration_file
/// [`FromStr`]: std::str::FromStr
/// [`From<&'_ str>`]: std::convert::From
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Events<'a> {
    /// Events seen before the first section.
    pub frontmatter: FrontMatterEvents<'a>,
    /// All parsed sections.
    pub sections: Vec<Section<'a>>,
}

impl Events<'static> {
    /// Parses the provided bytes, returning an [`Events`] that contains allocated
    /// and owned events. This is similar to [`Events::from_bytes()`], but performance
    /// is degraded as it requires allocation for every event.
    ///
    /// Use `filter` to only include those events for which it returns true.
    pub fn from_bytes_owned<'a>(
        input: &'a [u8],
        filter: Option<fn(&Event<'a>) -> bool>,
    ) -> Result<Events<'static>, parse::Error> {
        from_bytes(input, &|e| e.to_owned(), filter)
    }
}

impl<'a> Events<'a> {
    /// Attempt to zero-copy parse the provided bytes. On success, returns a
    /// [`Events`] that provides methods to accessing leading comments and sections
    /// of a `git-config` file and can be converted into an iterator of [`Event`]
    /// for higher level processing.
    ///
    /// Use `filter` to only include those events for which it returns true.
    pub fn from_bytes(input: &'a [u8], filter: Option<fn(&Event<'a>) -> bool>) -> Result<Events<'a>, parse::Error> {
        from_bytes(input, &std::convert::identity, filter)
    }

    /// Attempt to zero-copy parse the provided `input` string.
    ///
    /// Prefer the [`from_bytes()`][Self::from_bytes()] method if UTF8 encoding
    /// isn't guaranteed.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'a str) -> Result<Events<'a>, parse::Error> {
        Self::from_bytes(input.as_bytes(), None)
    }

    /// Consumes the parser to produce an iterator of all contained events.
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter(self) -> impl std::iter::FusedIterator<Item = parse::Event<'a>> {
        self.frontmatter.into_iter().chain(
            self.sections
                .into_iter()
                .flat_map(|section| std::iter::once(parse::Event::SectionHeader(section.header)).chain(section.events)),
        )
    }

    /// Place all contained events into a single `Vec`.
    pub fn into_vec(self) -> Vec<parse::Event<'a>> {
        self.into_iter().collect()
    }
}

impl<'a> TryFrom<&'a str> for Events<'a> {
    type Error = parse::Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl<'a> TryFrom<&'a [u8]> for Events<'a> {
    type Error = parse::Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        Events::from_bytes(value, None)
    }
}

fn from_bytes<'a, 'b>(
    input: &'a [u8],
    convert: &dyn Fn(Event<'a>) -> Event<'b>,
    filter: Option<fn(&Event<'a>) -> bool>,
) -> Result<Events<'b>, parse::Error> {
    let mut header = None;
    let mut events = Vec::with_capacity(256);
    let mut frontmatter = FrontMatterEvents::default();
    let mut sections = Vec::new();
    parse::from_bytes(input, &mut |e: Event<'_>| match e {
        Event::SectionHeader(next_header) => {
            match header.take() {
                None => {
                    frontmatter = std::mem::take(&mut events).into_iter().collect();
                }
                Some(prev_header) => {
                    sections.push(parse::Section {
                        header: prev_header,
                        events: std::mem::take(&mut events),
                    });
                }
            }
            header = match convert(Event::SectionHeader(next_header)) {
                Event::SectionHeader(h) => h,
                _ => unreachable!("BUG: convert must not change the event type, just the lifetime"),
            }
            .into();
        }
        event => {
            if filter.map_or(true, |f| f(&event)) {
                events.push(convert(event));
            }
        }
    })?;

    match header {
        None => {
            frontmatter = events.into_iter().collect();
        }
        Some(prev_header) => {
            sections.push(parse::Section {
                header: prev_header,
                events: std::mem::take(&mut events),
            });
        }
    }
    Ok(Events { frontmatter, sections })
}
