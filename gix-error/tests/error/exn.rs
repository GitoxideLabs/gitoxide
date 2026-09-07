// Copyright 2025 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{ErrorWithSource, fixup_paths, new_tree_error};
use gix_error::OptionExt;
use gix_error::ResultExt;
use gix_error::{ErrorExt, message};
use gix_error::{Exn, FrameExt, Message};

#[test]
fn raise_chain() {
    let e1 = message("E1").raise();
    let e2 = e1.raise(message("E2"));
    let e3 = e2.raise(message("E3"));
    let e4 = e3.raise(message("E4"));
    let e5 = e4.raise(message("E5"));
    insta::assert_debug_snapshot!(e5, "raised errors render newest context first", @r"
    E5
    |
    └─ E4
    |
    └─ E3
    |
    └─ E2
    |
    └─ E1
    ");
    insta::assert_compact_debug_snapshot!(&e5, "raised frames retain their caller locations", @"
    E5, at gix-error/tests/error/exn.rs:27
    |
    └─ E4, at gix-error/tests/error/exn.rs:26
    |
    └─ E3, at gix-error/tests/error/exn.rs:25
    |
    └─ E2, at gix-error/tests/error/exn.rs:24
    |
    └─ E1, at gix-error/tests/error/exn.rs:23
    ");

    let e = e5.erased();
    insta::assert_debug_snapshot!(e, "type erasure preserves the rendered error chain", @r"
    E5
    |
    └─ E4
    |
    └─ E3
    |
    └─ E2
    |
    └─ E1
    ");
    insta::assert_snapshot!(format!("{e:#}"), "alternate display exposes erased message types", @r#"
    Message("E5")
    |
    └─ Message("E4")
        |
        └─ Message("E3")
            |
            └─ Message("E2")
                |
                └─ Message("E1")
    "#);
    insta::assert_snapshot!(format!("{e:}"), "standard display shows only the top message", @"E5");

    insta::assert_compact_debug_snapshot!(&e, "type erasure preserves caller locations", @"
    E5, at gix-error/tests/error/exn.rs:27
    |
    └─ E4, at gix-error/tests/error/exn.rs:26
    |
    └─ E3, at gix-error/tests/error/exn.rs:25
    |
    └─ E2, at gix-error/tests/error/exn.rs:24
    |
    └─ E1, at gix-error/tests/error/exn.rs:23
    ");

    // Double-erase
    let e = e.erased();
    insta::assert_debug_snapshot!(e, "repeated erasure preserves the rendered chain", @r"
    E5
    |
    └─ E4
    |
    └─ E3
    |
    └─ E2
    |
    └─ E1
    ");

    insta::assert_snapshot!(format!("{e:#}"), "repeated erasure preserves alternate display", @r#"
    Message("E5")
    |
    └─ Message("E4")
        |
        └─ Message("E3")
            |
            └─ Message("E2")
                |
                └─ Message("E1")
    "#);
    assert_eq!(
        e.into_error().probable_cause().to_string(),
        "E1",
        "linear chains are just followed"
    );
}

#[test]
fn and_raise() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let exn = io_err.and_raise(message("could not read config"));
    insta::assert_debug_snapshot!(exn, "and_raise links context above its source", @r"
    could not read config
    |
    └─ file not found
    ");

    let io_err2 = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let exn2 = io_err2.raise().raise(message("could not read config"));
    assert_eq!(
        format!("{exn:#?}"),
        format!("{exn2:#?}"),
        "and_raise is equivalent to raise().raise() (compare with {{:#?}} to omit locations)"
    );
}

#[test]
fn raise_all() {
    let e = message("Top").raise_all(
        (1..5).map(|idx| message!("E{}", idx).raise_all((0..idx).map(|sidx| message!("E{}-{}", idx, sidx)))),
    );
    insta::assert_debug_snapshot!(e, "raise_all preserves nested child order", @r"
    Top
    |
    └─ E1
    |   |
    |   └─ E1-0
    |
    └─ E2
    |   |
    |   └─ E2-0
    |   |
    |   └─ E2-1
    |
    └─ E3
    |   |
    |   └─ E3-0
    |   |
    |   └─ E3-1
    |   |
    |   └─ E3-2
    |
    └─ E4
        |
        └─ E4-0
        |
        └─ E4-1
        |
        └─ E4-2
        |
        └─ E4-3
    ");
    insta::assert_compact_debug_snapshot!(&e, "raise_all retains every caller location", @"
    Top, at gix-error/tests/error/exn.rs:141
    |
    └─ E1, at gix-error/tests/error/exn.rs:142
    |   |
    |   └─ E1-0, at gix-error/tests/error/exn.rs:142
    |
    └─ E2, at gix-error/tests/error/exn.rs:142
    |   |
    |   └─ E2-0, at gix-error/tests/error/exn.rs:142
    |   |
    |   └─ E2-1, at gix-error/tests/error/exn.rs:142
    |
    └─ E3, at gix-error/tests/error/exn.rs:142
    |   |
    |   └─ E3-0, at gix-error/tests/error/exn.rs:142
    |   |
    |   └─ E3-1, at gix-error/tests/error/exn.rs:142
    |   |
    |   └─ E3-2, at gix-error/tests/error/exn.rs:142
    |
    └─ E4, at gix-error/tests/error/exn.rs:142
        |
        └─ E4-0, at gix-error/tests/error/exn.rs:142
        |
        └─ E4-1, at gix-error/tests/error/exn.rs:142
        |
        └─ E4-2, at gix-error/tests/error/exn.rs:142
        |
        └─ E4-3, at gix-error/tests/error/exn.rs:142
    ");

    let _this_should_compile = message("Top-untyped").raise_all((1..5).map(|idx| message!("E{}", idx).raise_erased()));

    assert_eq!(
        e.into_error().probable_cause().to_string(),
        "Top",
        "sometimes the cause is too ambiguous"
    );
}

#[test]
fn error_tree() {
    let err = new_tree_error();
    insta::assert_debug_snapshot!(err, "tree errors preserve their hierarchy", @r"
    E6
    |
    └─ E5
    |   |
    |   └─ E3
    |   |   |
    |   |   └─ E1
    |   |
    |   └─ E10
    |   |   |
    |   |   └─ E9
    |   |
    |   └─ E12
    |       |
    |       └─ E11
    |
    └─ E4
    |   |
    |   └─ E2
    |
    └─ E8
        |
        └─ E7
    ");
    insta::assert_compact_debug_snapshot!(&err, "tree errors retain caller locations", @"
    E6, at gix-error/tests/error/main.rs:26
    |
    └─ E5, at gix-error/tests/error/main.rs:18
    |   |
    |   └─ E3, at gix-error/tests/error/main.rs:10
    |   |   |
    |   |   └─ E1, at gix-error/tests/error/main.rs:9
    |   |
    |   └─ E10, at gix-error/tests/error/main.rs:13
    |   |   |
    |   |   └─ E9, at gix-error/tests/error/main.rs:12
    |   |
    |   └─ E12, at gix-error/tests/error/main.rs:16
    |       |
    |       └─ E11, at gix-error/tests/error/main.rs:15
    |
    └─ E4, at gix-error/tests/error/main.rs:21
    |   |
    |   └─ E2, at gix-error/tests/error/main.rs:20
    |
    └─ E8, at gix-error/tests/error/main.rs:24
        |
        └─ E7, at gix-error/tests/error/main.rs:23
    ");
    insta::assert_debug_snapshot!(err.frame().iter_frames().map(ToString::to_string).collect::<Vec<_>>(), "frame iteration is breadth-first", @r#"
    [
        "E6",
        "E5",
        "E4",
        "E8",
        "E3",
        "E10",
        "E12",
        "E2",
        "E7",
        "E1",
        "E9",
        "E11",
    ]
    "#);
}

#[test]
fn result_ext() {
    let result: Result<(), Message> = Err(message("An error"));
    let result = result.or_raise(|| message("Another error"));
    insta::assert_compact_debug_snapshot!(result.unwrap_err(), "or_raise records context and source at the call site", @"
    Another error, at gix-error/tests/error/exn.rs:290
    |
    └─ An error, at gix-error/tests/error/exn.rs:290
    ");
}

#[test]
fn option_ext() {
    let result: Option<()> = None;
    let result = result.ok_or_raise(|| message("An error"));
    insta::assert_compact_debug_snapshot!(result.unwrap_err(), "ok_or_raise records the failure call site", @"An error, at gix-error/tests/error/exn.rs:301");
}

#[test]
fn from_message() {
    fn foo() -> Result<(), Exn<Message>> {
        Err(message("An error"))?;
        Ok(())
    }

    let result = foo();
    insta::assert_compact_debug_snapshot!(result.unwrap_err(), "question-mark conversion records the propagation site", @"An error, at gix-error/tests/error/exn.rs:308");
}

#[test]
fn new_with_source() {
    let e = Exn::new(ErrorWithSource("top", message("source")));
    insta::assert_debug_snapshot!(e, "Exn::new retains the standard error source", @r"
    top
    |
    └─ source
    ");
}

#[test]
fn bail() {
    fn foo() -> Result<(), Exn<Message>> {
        gix_error::bail!(message("An error"));
    }

    let result = foo();
    insta::assert_compact_debug_snapshot!(result.unwrap_err(), "bail records the invocation site", @"An error, at gix-error/tests/error/exn.rs:329");
}

#[test]
fn ensure_ok() {
    fn foo() -> Result<(), Exn<Message>> {
        gix_error::ensure!(true, message("An error"));
        Ok(())
    }

    foo().unwrap();
}

#[test]
fn ensure_fail() {
    fn foo() -> Result<(), Exn<Message>> {
        gix_error::ensure!(false, message("An error"));
        Ok(())
    }

    let result = foo();
    insta::assert_compact_debug_snapshot!(result.unwrap_err(), "ensure failure records the invocation site", @"An error, at gix-error/tests/error/exn.rs:349");
}

#[test]
fn result_ok() -> Result<(), Exn<Message>> {
    Ok(())
}

#[test]
fn erased_into_message() {
    let e = message("E1").raise().erased();
    let _into_error_works = e.into_error();
}

#[cfg(feature = "anyhow")]
#[test]
fn raise_chain_anyhow() {
    let e1 = message("E1").raise_all([
        Exn::raise_all([message("E1c1-1"), message("E1c1-2")], message("E1-2")),
        Exn::raise_all([message("E1c2-1"), message("E1c2-2")], message("E1-3")),
    ]);
    let e2 = e1.raise(message("E2"));
    let root = e2.raise(Message::new("root"));

    // It's a linked list as linked up with the first child, but also has multiple children.
    insta::assert_snapshot!(format!("{root:#}"), "alternate display preserves mixed linear and branched chains", @r#"
    Message("root")
    |
    └─ Message("E2")
        |
        └─ Message("E1")
            |
            └─ Message("E1-2")
            |   |
            |   └─ Message("E1c1-1")
            |   |
            |   └─ Message("E1c1-2")
            |
            └─ Message("E1-3")
                |
                └─ Message("E1c2-1")
                |
                └─ Message("E1c2-2")
    "#);

    insta::assert_snapshot!(remove_stackstrace(format!("{:?}", anyhow::Error::from(root))), "anyhow traverses every error frame in order", @"
    root, at gix-error/tests/error/exn.rs:376

    Caused by:
        0: E2, at gix-error/tests/error/exn.rs:375
        1: E1, at gix-error/tests/error/exn.rs:371
        2: E1-2, at gix-error/tests/error/exn.rs:372
        3: E1-3, at gix-error/tests/error/exn.rs:373
        4: E1c1-1, at gix-error/tests/error/exn.rs:372
        5: E1c1-2, at gix-error/tests/error/exn.rs:372
        6: E1c2-1, at gix-error/tests/error/exn.rs:373
        7: E1c2-2, at gix-error/tests/error/exn.rs:373
    ");
}

fn remove_stackstrace(s: String) -> String {
    fixup_paths(s.find("Stack backtrace:").map_or(s.clone(), |pos| s[..pos].into()))
}

#[test]
fn into_chain() {
    let e1 = message("E1").raise_all([
        Exn::raise_all([message("E1c1-1"), message("E1c1-2")], message("E1-2")),
        Exn::raise_all([message("E1c2-1"), message("E1c2-2")], message("E1-3")),
    ]);
    let e2 = e1.raise(message("E2"));
    let root = e2.raise(Message::new("root"));

    insta::assert_snapshot!(format!("{root:#}"), "alternate display preserves the source tree before flattening", @r#"
    Message("root")
    |
    └─ Message("E2")
        |
        └─ Message("E1")
            |
            └─ Message("E1-2")
            |   |
            |   └─ Message("E1c1-1")
            |   |
            |   └─ Message("E1c1-2")
            |
            └─ Message("E1-3")
                |
                └─ Message("E1c2-1")
                |
                └─ Message("E1c2-2")
    "#);

    // It's a linked list as linked up with the first child, but also has multiple children.
    let root = root.into_chain();
    // By default, there is paths displayed, just like everywhere.
    insta::assert_debug_snapshot!(causes_display(&root, Style::Normal), "into_chain exposes locations for every source", @r#"
    [
        "root, at gix-error/tests/error/exn.rs:425",
        "E2, at gix-error/tests/error/exn.rs:424",
        "E1, at gix-error/tests/error/exn.rs:420",
        "E1-2, at gix-error/tests/error/exn.rs:421",
        "E1-3, at gix-error/tests/error/exn.rs:422",
        "E1c1-1, at gix-error/tests/error/exn.rs:421",
        "E1c1-2, at gix-error/tests/error/exn.rs:421",
        "E1c2-1, at gix-error/tests/error/exn.rs:422",
        "E1c2-2, at gix-error/tests/error/exn.rs:422",
    ]
    "#);

    // But these can also be turned off
    insta::assert_debug_snapshot!(causes_display(&root, Style::Alternate), "alternate source display omits locations", @r#"
    [
        "root",
        "E2",
        "E1",
        "E1-2",
        "E1-3",
        "E1c1-1",
        "E1c1-2",
        "E1c2-1",
        "E1c2-2",
    ]
    "#);

    // This should look similar.
    #[cfg(feature = "anyhow")]
    insta::assert_snapshot!(remove_stackstrace(format!("{:?}", anyhow::Error::from(root))), "into_chain matches anyhow source traversal", @"
    root, at gix-error/tests/error/exn.rs:425

    Caused by:
        0: E2, at gix-error/tests/error/exn.rs:424
        1: E1, at gix-error/tests/error/exn.rs:420
        2: E1-2, at gix-error/tests/error/exn.rs:421
        3: E1-3, at gix-error/tests/error/exn.rs:422
        4: E1c1-1, at gix-error/tests/error/exn.rs:421
        5: E1c1-2, at gix-error/tests/error/exn.rs:421
        6: E1c2-1, at gix-error/tests/error/exn.rs:422
        7: E1c2-2, at gix-error/tests/error/exn.rs:422
    ");
}

enum Style {
    Normal,
    Alternate,
}

fn causes_display(err: &(dyn std::error::Error + 'static), style: Style) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = Some(err);
    while let Some(err) = current {
        out.push(fixup_paths(match style {
            Style::Normal => err.to_string(),
            Style::Alternate => {
                format!("{err:#}")
            }
        }));
        current = err.source();
    }
    out
}

#[test]
fn erased_frames_still_expose_the_original_error() {
    let e = ErrorWithSource("E1", message("E1-source")).raise().erased();
    assert!(
        e.downcast_any_ref::<ErrorWithSource>().is_some(),
        "erased frames can still be downcast to the original error type"
    );
    let frame_error = e.iter().next().expect("there is one frame").error();
    assert!(
        frame_error.downcast_ref::<ErrorWithSource>().is_some(),
        "the frame yields the original error, not the erasure marker"
    );
    assert_eq!(
        frame_error
            .source()
            .expect("the source is reachable through the erasure")
            .to_string(),
        "E1-source",
        "std-style source chains continue through erased errors"
    );
}

/// Mirrors the pattern that broke in https://github.com/GitoxideLabs/gitoxide/issues/2694, where
/// a caller of `Error::iter_errors()` downcasts each error to react to a specific one.
#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn erased_errors_are_found_by_error_iteration() {
    let e: gix_error::Error = message("E1").raise_erased().into();
    assert!(
        e.iter_errors().any(|err| err.downcast_ref::<Message>().is_some()),
        "iter_errors() yields the original error type even after type-erasure"
    );
}

#[test]
fn native_sources_are_traversed_with_their_original_types() {
    let e = Exn::new(ErrorWithSource("top", ErrorWithSource("middle", message("bottom"))));
    assert!(
        e.frame().explicit_children().is_empty(),
        "native source snapshots are excluded from explicit frame traversal"
    );

    let middle = e
        .frame()
        .error()
        .source()
        .expect("the original error exposes its middle source");
    assert!(
        middle.is::<ErrorWithSource<Message>>(),
        "native traversal preserves the concrete middle-error type"
    );
    assert_eq!(
        middle
            .source()
            .expect("the middle error retains its original source")
            .to_string(),
        "bottom",
        "the retained native source chain reaches its leaf"
    );
    assert!(
        e.downcast_any_ref::<ErrorWithSource<Message>>().is_some(),
        "Exn downcasting lazily traverses concrete native source types"
    );
    assert!(
        e.downcast_any_ref::<Message>().is_some(),
        "Exn downcasting reaches the concrete native source leaf"
    );
}

#[test]
fn into_boxed_std_error() {
    let err: Box<dyn std::error::Error + Send + Sync> = message("failure").raise().into();
    let err = err
        .downcast_ref::<gix_error::Error>()
        .expect("conversion retains the gix error boundary type");
    assert_eq!(err.probable_cause().to_string(), "failure");
}

#[test]
fn erased_validation_error_remains_classified() {
    let err = gix_error::ValidationError::new("invalid").raise_erased().into_error();
    assert!(
        err.is_validation(),
        "the tree-backed Error classifies the original ValidationError exposed by Frame::error() after type erasure"
    );
}
