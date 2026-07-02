use gix_config::parse::{EventRef, Events, Section};

mod error;
mod from_bytes;
mod section;

#[test]
fn size_in_memory() {
    let actual = std::mem::size_of::<Section<'_>>();
    assert!(
        actual <= 40,
        "{actual} <= 40: This shouldn't change without us noticing"
    );
    let actual = std::mem::size_of::<Events>();
    assert!(
        actual <= 872,
        "{actual} <= 872: This shouldn't change without us noticing"
    );
}

#[test]
fn empty() {
    let events = Events::from_str("").unwrap();
    assert_eq!(events.iter().collect::<Vec<_>>(), vec![]);
}

#[test]
fn newlines_with_spaces() {
    let events = Events::from_str("\n   \n \n").unwrap();
    assert_eq!(
        events.iter().collect::<Vec<_>>(),
        vec![newline(), whitespace("   "), newline(), whitespace(" "), newline()]
    );
}

#[test]
fn consecutive_newlines() {
    let events = Events::from_str("\n\n\n\n\n").unwrap();
    assert_eq!(
        events.iter().collect::<Vec<_>>(),
        vec![newline_custom("\n\n\n\n\n")],
        "multiple newlines are merged into a single event"
    );
}

fn name(name: &'static str) -> EventRef<'static> {
    EventRef::SectionValueName(name.into())
}

fn value(value: &'static str) -> EventRef<'static> {
    EventRef::Value(value.into())
}

fn newline() -> EventRef<'static> {
    newline_custom("\n")
}

fn newline_custom(value: &'static str) -> EventRef<'static> {
    EventRef::Newline(value.into())
}

fn whitespace(value: &'static str) -> EventRef<'static> {
    EventRef::Whitespace(value.into())
}

fn separator() -> EventRef<'static> {
    EventRef::KeyValueSeparator
}
