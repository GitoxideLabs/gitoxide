use gix_error::{Error, message};
#[cfg(not(feature = "tree-error"))]
use gix_error::{ErrorExt, Exn, Message};
use std::error::Error as _;

#[cfg(not(feature = "tree-error"))]
#[test]
fn from_exn_error() {
    let err = Error::from(message("one").raise());
    assert_eq!(format!("{err:#}"), "one");
    insta::assert_snapshot!(debug_string(&err), @r#"Message("one")"#);
    insta::assert_debug_snapshot!(err, @r#"
    Message(
        "one",
    )
    "#);
    assert_eq!(err.source().map(debug_string), None);
}

#[cfg(not(feature = "tree-error"))]
#[test]
fn from_exn_error_tree() {
    let err = Error::from(new_tree_error().raise(message("topmost")));
    assert_eq!(format!("{err:#}").to_string(), "topmost");
    insta::assert_debug_snapshot!(err.sources().map(|err| fixup_paths(err.to_string())).collect::<Vec<_>>(), @r#"
    [
        "topmost, at gix-error/tests/auto_chain_error.rs:23",
        "E6, at gix-error/tests/auto_chain_error.rs:87",
        "E5, at gix-error/tests/auto_chain_error.rs:79",
        "E4, at gix-error/tests/auto_chain_error.rs:82",
        "E8, at gix-error/tests/auto_chain_error.rs:85",
        "E3, at gix-error/tests/auto_chain_error.rs:71",
        "E10, at gix-error/tests/auto_chain_error.rs:74",
        "E12, at gix-error/tests/auto_chain_error.rs:77",
        "E2, at gix-error/tests/auto_chain_error.rs:81",
        "E7, at gix-error/tests/auto_chain_error.rs:84",
        "E1, at gix-error/tests/auto_chain_error.rs:70",
        "E9, at gix-error/tests/auto_chain_error.rs:73",
        "E11, at gix-error/tests/auto_chain_error.rs:76",
    ]
    "#);
    assert_eq!(
        err.source().map(debug_string).as_deref(),
        Some(r#"Message("E6")"#),
        "The source is the first child"
    );
    assert_eq!(
        format!("{:#}", err.probable_cause()),
        "E6",
        "we get the top-most error that has most causes"
    );
}

#[test]
fn from_any_error() {
    let err = Error::from_error(message("one"));
    assert_eq!(format!("{err:#}"), "one");
    assert_eq!(debug_string(&err), r#"Message("one")"#);
    insta::assert_debug_snapshot!(err, @r#"
    Message(
        "one",
    )
    "#);
    assert_eq!(err.source().map(debug_string), None);
    assert_eq!(format!("{:#}", err.probable_cause()), "one");
}

#[cfg(not(feature = "tree-error"))]
pub fn new_tree_error() -> Exn<Message> {
    let e1 = message("E1").raise();
    let e3 = e1.raise(message("E3"));

    let e9 = message("E9").raise();
    let e10 = e9.raise(message("E10"));

    let e11 = message("E11").raise();
    let e12 = e11.raise(message("E12"));

    let e5 = Exn::raise_all([e3, e10, e12], message("E5"));

    let e2 = message("E2").raise();
    let e4 = e2.raise(message("E4"));

    let e7 = message("E7").raise();
    let e8 = e7.raise(message("E8"));

    Exn::raise_all([e5, e4, e8], message("E6"))
}

pub fn debug_string(input: impl std::fmt::Debug) -> String {
    fixup_paths(format!("{input:?}"))
}

fn fixup_paths(input: String) -> String {
    if cfg!(windows) { input.replace('\\', "/") } else { input }
}
