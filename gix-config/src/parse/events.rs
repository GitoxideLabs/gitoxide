use smallvec::SmallVec;

use crate::{
    parse,
    parse::{Event, EventRef, SectionData},
};

/// Events that precede the first section in a configuration file.
pub(crate) type FrontMatterEvents = SmallVec<[Event; 8]>;

/// A `git-config` file parser.
///
/// This parser exposes low-level syntactic events from a `git-config` file.
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
/// - Global properties before the first section are accepted for compatibility
///   with Git, even though they are uncommon in `.gitconfig` files.
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
/// and higher level wrappers around this parser should handle _semantic_ values.
/// This also means
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
/// Because this parser guarantees near-perfect reconstruction, there are many
/// non-significant events that occur in addition to the ones you may expect:
///
/// ```
/// # use gix_config::parse::{EventRef, Events};
/// # let events = Events::from_str("[core]\n  autocrlf = input")?;
/// assert_eq!(events.iter().collect::<Vec<_>>(), vec![
///     EventRef::SectionHeader {
///         name: "core".into(),
///         separator: None,
///         subsection_name: None,
///     },
///     EventRef::Newline("\n".into()),
///     EventRef::Whitespace("  ".into()),
///     EventRef::SectionValueName("autocrlf".into()),
///     EventRef::Whitespace(" ".into()),
///     EventRef::KeyValueSeparator,
///     EventRef::Whitespace(" ".into()),
///     EventRef::Value("input".into()),
/// ]);
/// # Ok::<_, gix_config::parse::Error>(())
/// ```
///
/// In particular, [`EventRef::SectionValueName`] and [`EventRef::Value`] are separated by two
/// [`EventRef::Whitespace`] events around an [`EventRef::KeyValueSeparator`]. If the config instead
/// had `autocrlf=input`, those whitespace events would not be present.
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
/// # use gix_config::parse::Events;
/// # let section_data = "[core]\n  autocrlf";
/// # let events = Events::from_str(section_data)?;
/// # assert_eq!(
/// #     events.iter().map(|event| event.to_string()).collect::<Vec<_>>(),
/// #     vec!["[core]", "\n", "  ", "autocrlf", ""]
/// # );
/// # Ok::<_, gix_config::parse::Error>(())
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
/// `false`. However, because this parser preserves the original event stream, we cannot process
/// partially quoted values, such as the `false` example. As a result, to
/// maintain consistency, the parser will just take all values as literals. The
/// relevant event stream emitted is thus emitted as:
///
/// ```
/// # use gix_config::parse::Events;
/// # let section_data = "[core]\nautocrlf=true\"\"\nfilemode=fa\"lse\"";
/// # let events = Events::from_str(section_data)?;
/// # assert_eq!(
/// #     events.iter().map(|event| event.to_string()).collect::<Vec<_>>(),
/// #     vec!["[core]", "\n", "autocrlf", "=", r#"true"""#, "\n", "filemode", "=", r#"fa"lse""#]
/// # );
/// # Ok::<_, gix_config::parse::Error>(())
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
/// # use gix_config::parse::Events;
/// # let section_data = "[some-section]\nfile=a\\\n    c";
/// # let events = Events::from_str(section_data)?;
/// # assert_eq!(
/// #     events.iter().map(|event| event.to_string()).collect::<Vec<_>>(),
/// #     vec!["[some-section]", "\n", "file", "=", "a\\", "\n", "    c"]
/// # );
/// # Ok::<_, gix_config::parse::Error>(())
/// ```
///
/// [`File`]: crate::File
/// [`.ini` file format]: https://en.wikipedia.org/wiki/INI_file
/// [`git`'s documentation]: https://git-scm.com/docs/git-config#_configuration_file
/// [`FromStr`]: std::str::FromStr
/// [`From<&'_ str>`]: std::convert::From
#[derive(Clone, Debug, Default)]
pub struct Events {
    pub(crate) backing: Vec<u8>,
    /// Events seen before the first section.
    pub(crate) frontmatter: FrontMatterEvents,
    /// All parsed sections.
    pub(crate) sections: Vec<SectionData>,
}

impl Events {
    /// Attempt to parse the provided bytes.
    ///
    /// Inputs larger than [`u32::MAX`] bytes are rejected because event spans use 32-bit offsets.
    ///
    /// Use `filter` to only include those events for which it returns true.
    pub fn from_bytes(input: &[u8], filter: Option<fn(EventRef<'_>) -> bool>) -> Result<Events, parse::Error> {
        let (mut events, error) = Self::parse_events(input, filter);
        if let Some(error) = error {
            return Err(error);
        }
        events.backing = input.to_vec();
        Ok(events)
    }

    /// Parse completed entries up to the first error, returning that error separately.
    ///
    /// The result contains all complete key/value pairs and section headers encountered
    /// before the error. An unfinished key and its value events are discarded together,
    /// including any line continuations. This permits validators to inspect earlier
    /// values even when later syntax is malformed. Includes are not resolved.
    ///
    /// As in [`Self::from_bytes()`], `filter` selects retained events. Inputs larger
    /// than [`u32::MAX`] produce no events and an error.
    pub fn from_bytes_partial(
        input: &[u8],
        filter: Option<fn(EventRef<'_>) -> bool>,
    ) -> (Events, Option<parse::Error>) {
        let (mut events, error) = Self::parse_events(input, filter);
        if input.len() <= u32::MAX as usize {
            events.backing = input.to_vec();
        }
        (events, error)
    }

    fn parse_events(input: &[u8], filter: Option<fn(EventRef<'_>) -> bool>) -> (Events, Option<parse::Error>) {
        let mut header = None;
        let mut events = Vec::with_capacity(256);
        let mut unfinished_key = None;
        let mut frontmatter = FrontMatterEvents::default();
        let mut sections = Vec::new();
        // The caller attaches backing bytes after deciding whether to retain partial results.
        let error = parse::from_bytes::from_bytes(input, &mut |e: Event| match e {
            Event::SectionHeader(next_header) => {
                match header.take() {
                    None => {
                        frontmatter = std::mem::take(&mut events).into_iter().collect();
                    }
                    Some(prev_header) => {
                        #[expect(
                            clippy::drain_collect,
                            reason = "Keep the scratch vector's allocation for parsing the next section."
                        )]
                        let section_events = events.drain(..).collect();
                        sections.push(parse::SectionData {
                            header: prev_header,
                            events: section_events,
                        });
                    }
                }
                header = Some(match Event::SectionHeader(next_header) {
                    Event::SectionHeader(h) => h,
                    _ => unreachable!("BUG: event type changed"),
                });
            }
            event => {
                match event {
                    Event::SectionValueName(_) => unfinished_key = Some(events.len()),
                    Event::Value(_) | Event::ValueDone(_) => unfinished_key = None,
                    _ => {}
                }
                if filter.is_none_or(|f| f(event.as_ref_in(input))) {
                    events.push(event);
                }
            }
        })
        .err();
        if error.is_some()
            && let Some(start) = unfinished_key
        {
            events.truncate(start);
        }

        match header {
            None => {
                frontmatter = events.into_iter().collect();
            }
            Some(prev_header) => {
                sections.push(parse::SectionData {
                    header: prev_header,
                    events: std::mem::take(&mut events),
                });
            }
        }
        (
            Events {
                backing: Vec::new(),
                frontmatter,
                sections,
            },
            error,
        )
    }

    /// Attempt to parse the provided `input` string.
    ///
    /// Prefer the [`from_bytes()`](Self::from_bytes()) method if UTF8 encoding
    /// isn't guaranteed.
    #[expect(
        clippy::should_implement_trait,
        reason = "the method has domain-specific semantics despite sharing a standard trait method name"
    )]
    pub fn from_str(input: &str) -> Result<Events, parse::Error> {
        Self::from_bytes(input.as_bytes(), None)
    }

    /// Return all contained events as borrowed views.
    pub fn iter(&self) -> impl Iterator<Item = EventRef<'_>> + '_ {
        self.frontmatter().chain(self.sections().flat_map(SectionRef::iter))
    }

    /// Return all events before the first section as borrowed views.
    pub fn frontmatter(&self) -> impl Iterator<Item = EventRef<'_>> + '_ {
        self.frontmatter.iter().map(move |event| event.as_ref_in(&self.backing))
    }

    /// Return all parsed sections as borrowed views.
    pub fn sections(&self) -> impl Iterator<Item = SectionRef<'_>> + '_ {
        self.sections.iter().map(move |section| SectionRef {
            header: &section.header,
            events: &section.events,
            backing: &self.backing,
        })
    }
}

impl TryFrom<&str> for Events {
    type Error = parse::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl TryFrom<&[u8]> for Events {
    type Error = parse::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Events::from_bytes(value, None)
    }
}

/// A borrowed view of a parsed section.
#[derive(Copy, Clone, Debug)]
pub struct SectionRef<'a> {
    header: &'a parse::section::HeaderData,
    events: &'a [Event],
    backing: &'a [u8],
}

impl<'a> SectionRef<'a> {
    /// Return the section header as an event view.
    pub fn header(&self) -> EventRef<'a> {
        EventRef::SectionHeader {
            name: self.header.name.as_bstr_in(self.backing),
            separator: self
                .header
                .separator
                .as_ref()
                .map(|separator| separator.as_bstr_in(self.backing)),
            subsection_name: self.header.subsection_name_original(self.backing),
        }
    }

    /// Return the events contained in this section body.
    pub fn body(self) -> impl Iterator<Item = EventRef<'a>> + 'a {
        self.events.iter().map(move |event| event.as_ref_in(self.backing))
    }

    /// Return the complete event stream for this section, including its header.
    pub fn iter(self) -> impl Iterator<Item = EventRef<'a>> + 'a {
        std::iter::once(self.header()).chain(self.body())
    }
}
