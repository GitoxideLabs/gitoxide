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
use gix_error::{Error, ResultExt, validation};
use gix_error::{ErrorExt, bail, message};
use gix_error::{Exn, ExnMessageResult, Message};
use gix_error::{OptionExt, ensure};

#[test]
fn raise_chain() {
    let e1 = message("E1").raise_typed();
    let e2 = e1.raise(message("E2"));
    let e3 = e2.raise(message("E3"));
    let e4 = e3.raise(message("E4"));
    let e5 = e4.raise(message("E5"));
    insta::assert_debug_snapshot!(e5, "raised errors render newest context first", @"
    E5

    Caused by:
        0: E4
        1: E3
        2: E2
        3: E1
    ");
    insta::assert_compact_debug_snapshot!(&e5, "raised frames retain their caller locations", @"
    E5, at gix-error/tests/error/exn.rs:27

    Caused by:
        0: E4, at gix-error/tests/error/exn.rs:26
        1: E3, at gix-error/tests/error/exn.rs:25
        2: E2, at gix-error/tests/error/exn.rs:24
        3: E1, at gix-error/tests/error/exn.rs:23
    ");

    let e = e5.erased();
    insta::assert_debug_snapshot!(e, "type erasure preserves the rendered error chain", @"
    E5

    Caused by:
        0: E4
        1: E3
        2: E2
        3: E1
    ");
    insta::assert_snapshot!(format!("{e:#}"), "alternate display exposes erased message types", @r#"
    Message { message: "E5" }

    Caused by:
        0: Message { message: "E4" }
        1: Message { message: "E3" }
        2: Message { message: "E2" }
        3: Message { message: "E1" }
    "#);
    insta::assert_snapshot!(format!("{e:}"), "standard display shows only the top message", @"E5");

    insta::assert_compact_debug_snapshot!(&e, "type erasure preserves caller locations", @"
    E5, at gix-error/tests/error/exn.rs:27

    Caused by:
        0: E4, at gix-error/tests/error/exn.rs:26
        1: E3, at gix-error/tests/error/exn.rs:25
        2: E2, at gix-error/tests/error/exn.rs:24
        3: E1, at gix-error/tests/error/exn.rs:23
    ");

    // Double-erase
    let e = e.erased();
    insta::assert_debug_snapshot!(e, "repeated erasure preserves the rendered chain", @"
    E5

    Caused by:
        0: E4
        1: E3
        2: E2
        3: E1
    ");

    insta::assert_snapshot!(format!("{e:#}"), "repeated erasure preserves alternate display", @r#"
    Message { message: "E5" }

    Caused by:
        0: Message { message: "E4" }
        1: Message { message: "E3" }
        2: Message { message: "E2" }
        3: Message { message: "E1" }
    "#);
    insta::assert_debug_snapshot!(format_args!("{}", e.into_error().probable_cause()), "linear chains are just followed", @"E1");
}

#[test]
fn and_raise() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let exn = io_err.and_raise_typed(message("could not read config"));
    insta::assert_debug_snapshot!(exn, "and_raise retains context, the I/O error, and its payload", @"
    could not read config

    Caused by:
        0: I/O error (NotFound)
        1: file not found
    ");

    let io_err2 = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let exn2 = io_err2.raise_typed().raise(message("could not read config"));
    assert_eq!(
        format!("{exn:#?}"),
        format!("{exn2:#?}"),
        "and_raise is equivalent to raise().raise_typed() (compare with {{:#?}} to omit locations)"
    );
}

#[test]
fn raise_all() {
    let e = message("Top").raise_all(
        (1..5).map(|idx| message!("E{}", idx).raise_all((0..idx).map(|sidx| message!("E{}-{}", idx, sidx)))),
    );
    insta::assert_debug_snapshot!(e, "raise_all preserves branch heads and nested child order", @"
    Top

    Caused by:
        0: E1
        └─0: E1-0
        1: E2
        ├─0: E2-0
        └─1: E2-1
        2: E3
        ├─0: E3-0
        ├─1: E3-1
        └─2: E3-2
        3: E4
        ├─0: E4-0
        ├─1: E4-1
        ├─2: E4-2
        └─3: E4-3
    ");
    insta::assert_compact_debug_snapshot!(&e, "raise_all retains every caller location", @"
    Top, at gix-error/tests/error/exn.rs:125

    Caused by:
        0: E1, at gix-error/tests/error/exn.rs:126
        └─0: E1-0, at gix-error/tests/error/exn.rs:126
        1: E2, at gix-error/tests/error/exn.rs:126
        ├─0: E2-0, at gix-error/tests/error/exn.rs:126
        └─1: E2-1, at gix-error/tests/error/exn.rs:126
        2: E3, at gix-error/tests/error/exn.rs:126
        ├─0: E3-0, at gix-error/tests/error/exn.rs:126
        ├─1: E3-1, at gix-error/tests/error/exn.rs:126
        └─2: E3-2, at gix-error/tests/error/exn.rs:126
        3: E4, at gix-error/tests/error/exn.rs:126
        ├─0: E4-0, at gix-error/tests/error/exn.rs:126
        ├─1: E4-1, at gix-error/tests/error/exn.rs:126
        ├─2: E4-2, at gix-error/tests/error/exn.rs:126
        └─3: E4-3, at gix-error/tests/error/exn.rs:126
    ");

    let e = e.chain_all((1..3).map(|idx| message!("SE{}", idx)));
    insta::assert_debug_snapshot!(e, "chain_all appends siblings without promoting branch-head causes", @"
    Top

    Caused by:
        0: E1
        └─0: E1-0
        1: E2
        ├─0: E2-0
        └─1: E2-1
        2: E3
        ├─0: E3-0
        ├─1: E3-1
        └─2: E3-2
        3: E4
        ├─0: E4-0
        ├─1: E4-1
        ├─2: E4-2
        └─3: E4-3
        4: SE1
        5: SE2
    ");

    insta::assert_snapshot!(format!("{:#}", e), "alternate display preserves the full error tree", @r#"
    Message { message: "Top" }

    Caused by:
        0: Message { message: "E1" }
        └─0: Message { message: "E1-0" }
        1: Message { message: "E2" }
        ├─0: Message { message: "E2-0" }
        └─1: Message { message: "E2-1" }
        2: Message { message: "E3" }
        ├─0: Message { message: "E3-0" }
        ├─1: Message { message: "E3-1" }
        └─2: Message { message: "E3-2" }
        3: Message { message: "E4" }
        ├─0: Message { message: "E4-0" }
        ├─1: Message { message: "E4-1" }
        ├─2: Message { message: "E4-2" }
        └─3: Message { message: "E4-3" }
        4: Message { message: "SE1" }
        5: Message { message: "SE2" }
    "#);
    let _this_should_compile = message("Top-untyped").raise_all((1..5).map(|idx| message!("E{}", idx).raise_erased()));

    insta::assert_debug_snapshot!(format_args!("{}", e.into_error().probable_cause()), "sometimes the cause is too ambiguous", @"Top");
}

#[test]
fn inverse_error_call_chain() {
    let e1 = message("E1").raise_typed();
    let e2 = e1.chain(message("E2"));
    let e3 = e2.chain(message("E3"));
    let e4 = e3.chain(message("E4"));
    let e5 = e4.chain(message("E5"));
    insta::assert_debug_snapshot!(e5, "chain appends errors in call order", @"
    E1

    Caused by:
        0: E2
        1: E3
        2: E4
        3: E5
    ");
    insta::assert_compact_debug_snapshot!(&e5, "chain retains caller locations in call order", @"
    E1, at gix-error/tests/error/exn.rs:218

    Caused by:
        0: E2, at gix-error/tests/error/exn.rs:219
        1: E3, at gix-error/tests/error/exn.rs:220
        2: E4, at gix-error/tests/error/exn.rs:221
        3: E5, at gix-error/tests/error/exn.rs:222
    ");

    insta::assert_snapshot!(format!("{e5:#}"), "alternate display follows chained order", @r#"
    Message { message: "E1" }

    Caused by:
        0: Message { message: "E2" }
        1: Message { message: "E3" }
        2: Message { message: "E4" }
        3: Message { message: "E5" }
    "#);

    insta::assert_debug_snapshot!(format_args!("{}", e5.into_error().probable_cause()), "branch root", @"E1");
}

#[test]
fn error_tree() {
    let mut err = new_tree_error();
    insta::assert_debug_snapshot!(err, "tree errors preserve sibling branch heads and keep their causes beneath them", @"
    E6

    Caused by:
        0: E5
        ├─0: E3
        │ └─0: E1
        ├─1: E10
        │ └─0: E9
        └─2: E12
          └─0: E11
        1: E4
        └─0: E2
        2: E8
        └─0: E7
    ");
    insta::assert_compact_debug_snapshot!(&err, "tree errors retain caller locations", @"
    E6, at gix-error/tests/error/main.rs:26

    Caused by:
        0: E5, at gix-error/tests/error/main.rs:18
        ├─0: E3, at gix-error/tests/error/main.rs:10
        │ └─0: E1, at gix-error/tests/error/main.rs:9
        ├─1: E10, at gix-error/tests/error/main.rs:13
        │ └─0: E9, at gix-error/tests/error/main.rs:12
        └─2: E12, at gix-error/tests/error/main.rs:16
          └─0: E11, at gix-error/tests/error/main.rs:15
        1: E4, at gix-error/tests/error/main.rs:21
        └─0: E2, at gix-error/tests/error/main.rs:20
        2: E8, at gix-error/tests/error/main.rs:24
        └─0: E7, at gix-error/tests/error/main.rs:23
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

    let new_e = message("E-New").raise_all(err.drain_children());
    insta::assert_debug_snapshot!(new_e, "drained children retain branch heads and their subtrees", @"
    E-New

    Caused by:
        0: E5
        ├─0: E3
        │ └─0: E1
        ├─1: E10
        │ └─0: E9
        └─2: E12
          └─0: E11
        1: E4
        └─0: E2
        2: E8
        └─0: E7
    ");
    insta::assert_snapshot!(err, "draining children leaves the root frame", @"E6");
}

#[test]
fn result_ext() {
    let result: Result<(), Message> = Err(message("An error"));
    let result = result.or_raise_typed(|| message("Another error"));
    insta::assert_compact_debug_snapshot!(result.unwrap_err(), "or_raise records context and source at the call site", @"
    Another error, at gix-error/tests/error/exn.rs:330

    Caused by:
        0: An error, at gix-error/tests/error/exn.rs:330
    ");
}

#[test]
fn option_ext() {
    let result: Option<()> = None;
    let result = result.ok_or_raise_typed(|| message("An error"));
    insta::assert_compact_debug_snapshot!(result.unwrap_err(), "ok_or_raise records the failure call site", @"An error, at gix-error/tests/error/exn.rs:342");
}

#[test]
fn from_message() {
    fn foo() -> ExnMessageResult {
        Err(message("An error"))?;
        Ok(())
    }

    let result = foo();
    insta::assert_compact_debug_snapshot!(result.unwrap_err(), "question-mark conversion records the propagation site", @"An error, at gix-error/tests/error/exn.rs:349");
}

#[test]
fn new_with_source() {
    let e = Exn::new(ErrorWithSource("top", message("source")));
    insta::assert_debug_snapshot!(e, "Exn::new retains the standard error source", @"
    top

    Caused by:
        0: source
    ");
}

#[test]
fn bail() {
    fn foo() -> ExnMessageResult {
        bail!(message("An error"));
    }

    let result = foo();
    insta::assert_compact_debug_snapshot!(result.unwrap_err(), "bail records the invocation site", @"An error, at gix-error/tests/error/exn.rs:371");
}

#[test]
fn ensure_ok() {
    fn foo() -> ExnMessageResult {
        ensure!(true, message("An error"));
        Ok(())
    }

    foo().unwrap();
}

#[test]
fn ensure_fail() {
    fn foo() -> ExnMessageResult {
        ensure!(false, message("An error"));
        Ok(())
    }

    let result = foo();
    insta::assert_compact_debug_snapshot!(result.unwrap_err(), "ensure failure records the invocation site", @"An error, at gix-error/tests/error/exn.rs:391");
}

#[test]
fn result_ok() -> ExnMessageResult {
    Ok(())
}

#[test]
fn erased_into_inner() {
    let e = message("E1").raise_erased();
    let _into_inner_works = e.into_inner();
}

#[test]
fn erased_into_box() {
    let e = message("E1").raise_erased();
    let _into_box_works = e.into_box();
}

#[test]
fn erased_into_message() {
    let e = message("E1").raise_typed().erased();
    let _into_error_works = e.into_error();
}

#[test]
fn into_chain() {
    let e1 = message("E1")
        .raise_typed()
        .chain(Exn::raise_all([message("E1c1-1"), message("E1c1-2")], message("E1-2")))
        .chain(Exn::raise_all([message("E1c2-1"), message("E1c2-2")], message("E1-3")));
    let e2 = e1.raise(message("E2"));
    let root = e2.raise(Message::new("root"));

    insta::assert_snapshot!(format!("{root:#}"), "alternate display preserves the source tree before flattening", @r#"
    Message { message: "root" }

    Caused by:
        0: Message { message: "E2" }
        1: Message { message: "E1" }
        ├─0: Message { message: "E1-2" }
        │ ├─0: Message { message: "E1c1-1" }
        │ └─1: Message { message: "E1c1-2" }
        └─1: Message { message: "E1-3" }
          ├─0: Message { message: "E1c2-1" }
          └─1: Message { message: "E1c2-2" }
    "#);

    // It's a linked list as linked up with the first child, but also has multiple children.
    let root = root.into_chain();
    // By default, there is paths displayed, just like everywhere.
    insta::assert_debug_snapshot!(causes_display(&root, Style::Normal), "into_chain exposes locations for every source", @r#"
    [
        "root, at gix-error/tests/error/exn.rs:429",
        "E2, at gix-error/tests/error/exn.rs:428",
        "E1, at gix-error/tests/error/exn.rs:425",
        "E1-2, at gix-error/tests/error/exn.rs:426",
        "E1-3, at gix-error/tests/error/exn.rs:427",
        "E1c1-1, at gix-error/tests/error/exn.rs:426",
        "E1c1-2, at gix-error/tests/error/exn.rs:426",
        "E1c2-1, at gix-error/tests/error/exn.rs:427",
        "E1c2-2, at gix-error/tests/error/exn.rs:427",
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
    let e = ErrorWithSource("E1", message("E1-source")).raise_typed().erased();
    insta::assert_debug_snapshot!(e, "erased frames can still be downcast to the original error type", @"
    E1

    Caused by:
        0: E1-source
    ");
    assert!(
        e.downcast_any_ref::<ErrorWithSource>().is_some(),
        "erased frames can still be downcast to the original error type"
    );
    let frame_error = e.iter().next().expect("there is one frame").error();
    insta::assert_debug_snapshot!(frame_error, "the frame yields the original error, not the erasure marker", @r#"
    ErrorWithSource(
        "E1",
        Message {
            message: "E1-source",
        },
    )
    "#);
    assert!(
        frame_error.downcast_ref::<ErrorWithSource>().is_some(),
        "the frame yields the original error, not the erasure marker"
    );
    insta::assert_debug_snapshot!(format_args!("{}", frame_error
            .source()
            .expect("the source is reachable through the erasure")
            ), "std-style source chains continue through erased errors", @"E1-source");
}

/// Mirrors the pattern that broke in https://github.com/GitoxideLabs/gitoxide/issues/2694, where
/// a caller of `Error::iter_errors()` downcasts each error to react to a specific one.
#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn erased_errors_are_found_by_error_iteration() {
    let e: Error = message("E1").raise_erased().into();
    assert!(
        e.iter_errors().any(|err| err.downcast_ref::<Message>().is_some()),
        "iter_errors() yields the original error type even after type-erasure"
    );
}

#[test]
fn erased_into_inner_preserves_source_chain() {
    let e = ErrorWithSource("E1", message("E1-source")).raise_erased().into_inner();
    insta::assert_debug_snapshot!(format_args!("{}", std::error::Error::source(&e)
            .expect("the erased error forwards to the wrapped error's source")
            ), "type erasure remains transparent to std-style source traversal", @"E1-source");
}

#[test]
fn native_sources_are_retained_and_traversed_lazily() {
    let e = Exn::new(ErrorWithSource("top", ErrorWithSource("middle", message("bottom"))));
    assert!(
        e.frame().children().is_empty(),
        "native sources remain owned by their error instead of becoming mutable Frame children"
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
    insta::assert_debug_snapshot!(format_args!("{}", middle
            .source()
            .expect("the middle error retains its original source")
            ), "the retained native source chain reaches its leaf", @"bottom");
    insta::assert_debug_snapshot!(e, "Exn downcasting lazily traverses concrete native source types", @"
    top

    Caused by:
        0: middle
        1: bottom
    ");
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
fn inspection_visits_native_sources_on_demand() {
    #[derive(Debug)]
    struct CountedSource {
        source_calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        source: Message,
    }

    impl std::fmt::Display for CountedSource {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("counted")
        }
    }

    impl std::error::Error for CountedSource {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.source_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(&self.source)
        }
    }

    let source_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let e = Exn::new(CountedSource {
        source_calls: std::sync::Arc::clone(&source_calls),
        source: message("source"),
    });
    assert_eq!(
        source_calls.load(std::sync::atomic::Ordering::Relaxed),
        0,
        "constructing an Exn neither traverses nor snapshots native sources"
    );
    assert!(e.downcast_any_ref::<CountedSource>().is_some());
    assert_eq!(
        source_calls.load(std::sync::atomic::Ordering::Relaxed),
        0,
        "finding the outer error does not visit its sources"
    );

    assert!(
        e.downcast_any_ref::<Message>().is_some(),
        "the source becomes reachable when a source-aware operation requests it"
    );
    assert!(
        source_calls.load(std::sync::atomic::Ordering::Relaxed) > 0,
        "source-aware operations traverse native sources on demand"
    );
    insta::assert_debug_snapshot!(e, "inspection renders retained sources after measuring lazy traversal", @"
    counted

    Caused by:
        0: source
    ");

    let e = e.raise(gix_error::ClassificationMarker::with_source(
        gix_error::Class::Retryable,
        message("retry"),
    ));
    source_calls.store(0, std::sync::atomic::Ordering::Relaxed);
    assert!(e.can_retry());
    assert_eq!(
        source_calls.load(std::sync::atomic::Ordering::Relaxed),
        1,
        "retry policy checks remaining sources for cancellation"
    );
    insta::assert_debug_snapshot!(e, "inspection renders retained sources after measuring lazy traversal", @"
    retry
    counted

    Caused by:
        0: source
    ");
    let e = e.into_error();
    source_calls.store(0, std::sync::atomic::Ordering::Relaxed);
    assert!(e.can_retry());
    assert!(e.iter_errors_with_locations().next().is_some());
    assert_eq!(
        source_calls.load(std::sync::atomic::Ordering::Relaxed),
        if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
            4
        } else {
            1
        },
        "retry policy resolves sources to check for cancellation"
    );
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(e, "inspection renders retained sources after measuring lazy traversal", @"
        retry

        Caused by:
            0: counted
            1: source
        ");
    } else {
        insta::assert_debug_snapshot!(e, "inspection renders retained sources after measuring lazy traversal", @"
        retry
        counted

        Caused by:
            0: source
        ");
    }
}

#[test]
fn into_boxed_std_error() {
    let err: Box<dyn std::error::Error + Send + Sync> = message("failure").raise_typed().into();
    let err = err
        .downcast_ref::<Error>()
        .expect("conversion retains the gix error boundary type");
    insta::assert_debug_snapshot!(err, "into boxed std error", @"failure");
    insta::assert_debug_snapshot!(format_args!("{}", err.probable_cause()), "boxed errors preserve the selected cause", @"failure");
}

#[test]
fn erased_validation_error_remains_classified() {
    let err = validation("invalid").raise_erased().into_error();
    insta::assert_debug_snapshot!(err, "the tree-backed Error classifies the original Message exposed by Frame::error() after type erasure", @"invalid");
    assert!(
        err.is_validation(),
        "the tree-backed Error classifies the original Message exposed by Frame::error() after type erasure"
    );
}

#[test]
fn downcasts_cross_nested_error_boundaries_in_breadth_first_order() {
    use gix_error::{Error, Message, validation};

    fn check<E: std::error::Error + Send + Sync + 'static>(exn: Exn<E>) -> impl std::fmt::Debug {
        let report = gix_testtools::redact_debug_snapshot(&exn, &[]);
        let selected = exn
            .downcast_any_ref::<Message>()
            .expect("the message is reachable without consuming the exception")
            .to_string();
        assert_eq!(
            exn.into_error()
                .downcast_any_ref::<Message>()
                .expect("conversion preserves the message")
                .to_string(),
            selected,
            "conversion preserves the downcast result"
        );
        (report, selected)
    }

    let nested = Error::from_error(validation("nested")).raise_typed();
    insta::assert_debug_snapshot!(nested, "downcasting can still find the nested Error wrapper itself", @"nested");
    assert!(
        std::ptr::eq(
            nested.downcast_any_ref::<Error>().expect("the wrapper is reachable"),
            nested.error()
        ),
        "downcasting can still find the nested Error wrapper itself"
    );
    insta::assert_debug_snapshot!(check(nested), "breadth-first downcasting selects nested", @r#"
    (
        nested,
        "nested",
    )
    "#);
    insta::assert_debug_snapshot!(check(ErrorWithSource("native wrapper", Error::from_error(validation("native nested"))).raise_erased()), "breadth-first downcasting selects native nested", @r#"
    (
        native wrapper
        
        Caused by:
            0: native nested,
        "native nested",
    )
    "#);
    insta::assert_debug_snapshot!(check(std::io::Error::other("root")
            .raise_typed()
            .chain(Error::from_error(validation("nested")))
            .chain(validation("direct sibling"))), "breadth-first downcasting selects direct sibling", @r#"
    (
        I/O error (Other)
        
        Caused by:
            0: root
            1: nested
            2: direct sibling,
        "direct sibling",
    )
    "#);
    insta::assert_debug_snapshot!(check(Error::from_error(validation("nested root"))
            .raise_typed()
            .chain(validation("explicit child"))), "breadth-first downcasting selects nested root", @r#"
    (
        nested root
        
        Caused by:
            0: explicit child,
        "nested root",
    )
    "#);
    insta::assert_debug_snapshot!(check(std::io::Error::other("root")
            .raise_typed()
            .chain(Error::from_error(ErrorWithSource(
                "nested source",
                validation("deeper"),
            )))
            .chain(ErrorWithSource("sibling", validation("shallower")))), "breadth-first downcasting selects shallower", @r#"
    (
        I/O error (Other)
        
        Caused by:
            0: root
            1: nested source
            └─0: deeper
            2: sibling
            └─0: shallower,
        "shallower",
    )
    "#);
    insta::assert_debug_snapshot!(check(ErrorWithSource("root", Error::from_error(validation("native boundary")))
            .raise_typed()
            .chain(Error::from_error(validation("explicit boundary")))), "breadth-first downcasting selects native boundary", @r#"
    (
        root
        
        Caused by:
            0: native boundary
            1: explicit boundary,
        "native boundary",
    )
    "#);
    assert!(
        Error::from_error(std::io::Error::other("not a message"))
            .raise_typed()
            .downcast_any_ref::<Message>()
            .is_none(),
        "nested errors without the requested type do not produce a match"
    );
    insta::assert_debug_snapshot!(check(message("root").raise_typed().chain(validation("child"))), "breadth-first downcasting selects root", @r#"
    (
        root
        
        Caused by:
            0: child,
        "root",
    )
    "#);
}

#[test]
fn probable_cause_is_available_without_consuming_the_exception() {
    let mut diagnostics = Vec::new();
    use gix_error::{Error, Message, validation};

    // A native source may occupy the same address as its owner without being the same error.
    #[derive(Debug)]
    #[repr(transparent)]
    struct Wrapper<E>(E);

    impl<E> std::fmt::Display for Wrapper<E> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("wrapper")
        }
    }

    impl<E: std::error::Error + 'static> std::error::Error for Wrapper<E> {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    let leaf = message("leaf").raise_erased();
    insta::assert_debug_snapshot!(leaf, "a childless exception returns its stored error even after erasure", @"leaf");
    assert!(
        std::ptr::eq(leaf.probable_cause(), leaf.frame().error()),
        "a childless exception returns its stored error even after erasure"
    );

    for exn in [
        leaf,
        Wrapper(Wrapper(message("native leaf"))).raise_erased(),
        crate::new_tree_error().erased(),
        Error::from_error(Error::from_error(validation("nested cause")))
            .and_raise_typed(message("context"))
            .erased(),
    ] {
        let expected = exn.probable_cause().to_string();
        diagnostics.push((gix_testtools::redact_debug_snapshot(&exn, &[]), expected.clone()));
        assert_eq!(
            exn.into_error().probable_cause().to_string(),
            expected,
            "conversion preserves the probable cause"
        );
    }

    let exn = Error::from_error(validation("typed cause")).raise_typed();
    insta::assert_debug_snapshot!(exn, "a probable cause within a nested error retains its concrete type", @"typed cause");
    assert!(
        exn.probable_cause().is::<Message>(),
        "a probable cause within a nested error retains its concrete type"
    );
    insta::assert_debug_snapshot!(diagnostics, "probable cause is available without consuming the exception", @r#"
    [
        (
            leaf,
            "leaf",
        ),
        (
            wrapper
            
            Caused by:
                0: wrapper
                1: native leaf,
            "native leaf",
        ),
        (
            E6
            
            Caused by:
                0: E5
                ├─0: E3
                │ └─0: E1
                ├─1: E10
                │ └─0: E9
                └─2: E12
                  └─0: E11
                1: E4
                └─0: E2
                2: E8
                └─0: E7,
            "E6",
        ),
        (
            context
            
            Caused by:
                0: nested cause,
            "nested cause",
        ),
    ]
    "#);
}

#[test]
fn drained_children_are_valid_bare_exceptions() {
    let mut diagnostics = Vec::new();
    let child = || {
        ErrorWithSource("child", message("native source"))
            .raise_typed()
            .chain(validation("explicit cause"))
    };
    let mut parent = message("parent").raise_typed().chain(child()).chain(child().erased());
    let children = parent.drain_children().collect::<Vec<_>>();
    assert!(
        parent.frame().children().is_empty(),
        "draining removes the explicit children"
    );

    for child in children {
        insta::allow_duplicates! { insta::assert_debug_snapshot!(format_args!("{}", child.error()), "the bare exception has a valid Untyped root", @"child"); };
        insta::allow_duplicates! { insta::assert_debug_snapshot!(format_args!("{}", (*child)), "dereferencing a drained exception is valid", @"child"); };
        diagnostics.push(gix_testtools::redact_debug_snapshot(&child, &[]));
        assert!(
            child.downcast_any_ref::<ErrorWithSource>().is_some(),
            "draining retains the original concrete error"
        );
        assert!(child.is_validation(), "draining retains explicitly raised causes");
        insta::allow_duplicates! { insta::assert_debug_snapshot!(format_args!("{}", std::error::Error::source(child.error())
        .expect("the original native source is retained")
        ), "drained exceptions retain their native source diagnostic", @"native source"); };
        insta::allow_duplicates! { insta::assert_debug_snapshot!(format_args!("{}", child.into_inner()), "the erased root can be extracted safely", @"child"); };
    }

    let frame = gix_error::exn::Frame::from(message("direct conversion").raise_typed());
    insta::assert_debug_snapshot!(format_args!("{}", Exn::from(frame).into_box()), "direct Frame conversion also establishes the bare exception invariant", @"direct conversion");
    insta::assert_debug_snapshot!(diagnostics, "drained children are valid bare exceptions", @"
    [
        child
        
        Caused by:
            0: native source
            1: explicit cause,
        child
        
        Caused by:
            0: native source
            1: explicit cause,
    ]
    ");
}

#[test]
fn nested_error_formatting_prints_each_cause_once() {
    struct NativeSource(Error);

    impl std::fmt::Display for NativeSource {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("native-wrapper")
        }
    }

    impl std::fmt::Debug for NativeSource {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // Keep the source out of this error's own Debug output so the test measures our traversal.
            std::fmt::Display::fmt(self, f)
        }
    }

    impl std::error::Error for NativeSource {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    let nested = Error::from(message("inner-root").raise_typed().chain(message("inner-child")));
    let nested = Error::from(nested.raise_typed().chain(message("boundary-child")));
    let nested = Error::from(nested.raise_typed().chain(message("extra-child")));
    let err = message("outer-root")
        .raise_typed()
        .chain(NativeSource(nested))
        .chain(message("outer-sibling"));
    insta::assert_debug_snapshot!(format_args!("{}", err), "normal display shows the outermost error", @"outer-root");

    insta::assert_snapshot!(
        fixup_paths(format!("{err:?}")),
        "compact Debug expands nested error boundaries once and retains caller locations",
        @"
    outer-root, at gix-error/tests/error/exn.rs:999

    Caused by:
        0: native-wrapper, at gix-error/tests/error/exn.rs:1000
        ├─0: inner-root, at gix-error/tests/error/exn.rs:1000
        ├─1: extra-child, at gix-error/tests/error/exn.rs:997
        ├─2: boundary-child, at gix-error/tests/error/exn.rs:996
        └─3: inner-child, at gix-error/tests/error/exn.rs:995
        1: outer-sibling, at gix-error/tests/error/exn.rs:1001
    "
    );
    insta::assert_snapshot!(
        format!("{err:#?}"),
        "pretty Debug expands nested error boundaries once without caller locations",
        @"
    outer-root

    Caused by:
        0: native-wrapper
        ├─0: inner-root
        ├─1: extra-child
        ├─2: boundary-child
        └─3: inner-child
        1: outer-sibling
    "
    );
    insta::assert_snapshot!(
        format!("{err:#}"),
        "alternate display expands nested error boundaries once with concrete error types",
        @r#"
    Message { message: "outer-root" }

    Caused by:
        0: native-wrapper
        ├─0: Message { message: "inner-root" }
        ├─1: Message { message: "extra-child" }
        ├─2: Message { message: "boundary-child" }
        └─3: Message { message: "inner-child" }
        1: Message { message: "outer-sibling" }
    "#
    );

    for rendered in [format!("{err:?}"), format!("{err:#?}"), format!("{err:#}")] {
        for cause in [
            "outer-root",
            "native-wrapper",
            "inner-root",
            "inner-child",
            "boundary-child",
            "extra-child",
            "outer-sibling",
        ] {
            assert_eq!(
                rendered.matches(cause).count(),
                1,
                "each cause is rendered once, even across nested error boundaries: {rendered}"
            );
        }
    }
}

pub(super) fn assert_io_payload_report(report: &str, expected: &str, locations: usize, counts: &[(&str, usize)]) {
    assert_eq!(
        report.matches(", at ").count(),
        locations,
        "only compact Debug includes caller locations: {report}"
    );
    for (label, count) in counts {
        assert_eq!(
            report.matches(label).count(),
            *count,
            "each diagnostic is rendered independently, without deduplicating equal labels: {label}: {report}"
        );
    }
    let without_locations = report
        .lines()
        .map(|line| line.split_once(", at ").map_or(line, |(label, _)| label))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        without_locations, expected,
        "labels and report structure are exact apart from caller locations"
    );
}

#[test]
fn io_payload_direct_validation_retains_metadata_and_types() {
    use gix_error::{Class, MetadataValue, validation};
    use std::io::{Error, ErrorKind};

    let err = Error::new(
        ErrorKind::InvalidData,
        validation("invalid input").with_input(b"ref\xff".as_slice()),
    )
    .raise_typed();
    let expected_values = err
        .error()
        .get_ref()
        .and_then(|payload| payload.downcast_ref::<Message>())
        .expect("I/O owns the original validation message")
        .values
        .clone();
    assert_eq!(
        expected_values["input"],
        MetadataValue::from(b"ref\xff".as_slice()),
        "metadata retains non-UTF-8 bytes"
    );
    let classes = err.classify().map(|item| item.class()).collect::<Vec<_>>();
    assert_eq!(
        classes,
        [Class::Validation],
        "the payload supplies the classification through its unclassified I/O wrapper"
    );
    assert!(err.is_validation(), "the payload class is visible before conversion");
    assert_eq!(
        err.metadata().collect::<Vec<_>>(),
        [&expected_values],
        "metadata is visited once before conversion"
    );
    assert_eq!(
        err.downcast_any_ref::<Error>()
            .expect("I/O remains downcastable")
            .kind(),
        ErrorKind::InvalidData,
        "rendering must not replace the I/O error"
    );

    let human = r#"I/O error (InvalidData)

Caused by:
    0: invalid input, "input"="ref\xff""#;
    let typed = r#"I/O error (InvalidData)

Caused by:
    0: Message { message: "invalid input", class: Validation, values: {"input": Bytes("ref\xff")} }"#;
    let compact = format!("{err:?}");
    let location = err.frame().location();
    let at = format!(", at {}:{}", location.file(), location.line());
    assert_eq!(
        compact,
        format!(
            r#"I/O error (InvalidData){at}

Caused by:
    0: invalid input, "input"="ref\xff"{at}"#
        ),
        "a native payload inherits its owning frame's rendering location"
    );
    for (report, expected, locations) in [
        (compact, human, 2),
        (format!("{err:#?}"), human, 0),
        (format!("{err:#}"), typed, 0),
    ] {
        assert_io_payload_report(
            &report,
            expected,
            locations,
            &[("I/O error (InvalidData)", 1), ("invalid input", 1), (r"ref\xff", 1)],
        );
    }

    let err = err.into_error();
    let io = err
        .downcast_any_ref::<Error>()
        .expect("conversion retains the I/O error");
    assert_eq!(io.kind(), ErrorKind::InvalidData, "conversion preserves the I/O kind");
    let payload = io
        .get_ref()
        .and_then(|payload| payload.downcast_ref::<Message>())
        .expect("conversion preserves the concrete I/O payload");
    assert!(
        std::ptr::eq(
            payload,
            err.downcast_any_ref::<Message>()
                .expect("the payload remains downcastable")
        ),
        "traversal exposes the actual payload, not a replacement message"
    );
    assert!(err.is_validation(), "the payload class is visible after conversion");
    assert_eq!(
        err.classify().map(|item| item.class()).collect::<Vec<_>>(),
        classes,
        "rendering and conversion preserve classifications"
    );
    assert_eq!(
        err.metadata().collect::<Vec<_>>(),
        [&expected_values],
        "rendering and conversion preserve metadata without duplication"
    );
}

#[test]
fn io_payload_nested_wrappers_preserve_branches_and_equal_messages() {
    use std::io::{Error, ErrorKind};

    let branching = message("batch")
        .raise_typed()
        .chain(message("left").raise_typed().chain(message("same")))
        .chain(message("right").raise_typed().chain(message("same")))
        .into_error();
    let err = Error::new(ErrorKind::PermissionDenied, Error::other(branching))
        .raise_typed()
        .chain(message("explicit sibling"));
    let human = "I/O error (PermissionDenied)

Caused by:
    0: I/O error (Other)
    ├─0: batch
    ├─1: left
    ├─2: right
    ├─3: same
    └─4: same
    1: explicit sibling";
    let typed = format!(
        "I/O error (PermissionDenied)

Caused by:
    0: I/O error (Other)
    ├─0: {:?}
    ├─1: {:?}
    ├─2: {:?}
    ├─3: {:?}
    └─4: {:?}
    1: {:?}",
        message("batch"),
        message("left"),
        message("right"),
        message("same"),
        message("same"),
        message("explicit sibling")
    );
    let original_nodes = err.iter_errors().count();
    assert_eq!(
        err.frame().children().len(),
        1,
        "the explicit sibling is the only owned child frame"
    );
    for (report, expected, locations) in [
        (format!("{err:?}"), human, 8),
        (format!("{err:#?}"), human, 0),
        (format!("{err:#}"), typed.as_str(), 0),
    ] {
        assert_io_payload_report(
            &report,
            expected,
            locations,
            &[
                ("I/O error (PermissionDenied)", 1),
                ("I/O error (Other)", 1),
                ("batch", 1),
                ("left", 1),
                ("right", 1),
                ("same", 2),
                ("explicit sibling", 1),
            ],
        );
    }
    assert_eq!(
        err.iter_errors().count(),
        original_nodes,
        "rendering does not modify the graph"
    );
    let err = err.into_error();
    assert_eq!(
        err.iter_errors().count(),
        original_nodes,
        "conversion preserves boundary entries and their contents in traversal"
    );
    let io = err
        .downcast_any_ref::<Error>()
        .expect("the outer I/O wrapper remains accessible");
    assert_eq!(
        io.kind(),
        ErrorKind::PermissionDenied,
        "the outer wrapper keeps its kind"
    );
    let inner = io
        .get_ref()
        .and_then(|payload| payload.downcast_ref::<Error>())
        .expect("the inner I/O wrapper remains the native payload");
    assert_eq!(inner.kind(), ErrorKind::Other, "the inner wrapper keeps its own kind");
    assert!(
        inner
            .get_ref()
            .is_some_and(<dyn std::error::Error + Send + Sync>::is::<gix_error::Error>),
        "the branching error remains the inner I/O payload"
    );
    assert_eq!(
        err.iter_errors()
            .filter_map(|error| error.downcast_ref::<Message>())
            .filter(|message| message.to_string() == "same")
            .count(),
        2,
        "equal messages on separate branches remain separate errors"
    );
    assert!(
        err.iter_errors().any(|error| error
            .downcast_ref::<Message>()
            .is_some_and(|message| message.to_string() == "explicit sibling")),
        "the explicit sibling remains in the graph"
    );
}

#[test]
fn io_payload_report_does_not_leak_alternate_into_display_labels() {
    struct Sensitive(&'static str, Option<Message>);

    impl std::fmt::Display for Sensitive {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if f.alternate() {
                write!(f, "LEAKED {}", self.0)
            } else {
                f.write_str(self.0)
            }
        }
    }

    impl std::fmt::Debug for Sensitive {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Sensitive({:?})", self.0)
        }
    }

    impl std::error::Error for Sensitive {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.1.as_ref().map(|source| source as _)
        }
    }

    let nested = Sensitive("inner", None)
        .raise_typed()
        .chain(Sensitive("child", Some(message("native leaf"))))
        .into_error();
    let err = Sensitive("outer", None).raise_typed().chain(nested);
    let human = "outer

Caused by:
    0: inner
    1: child
    2: native leaf";
    let typed = r#"Sensitive("outer")

Caused by:
    0: Sensitive("inner")
    1: Sensitive("child")
    2: Message { message: "native leaf" }"#;
    let counts = [
        ("outer", 1),
        ("inner", 1),
        ("child", 1),
        ("native leaf", 1),
        ("LEAKED", 0),
    ];
    for (report, expected, locations) in [
        (format!("{err:?}"), human, 4),
        (format!("{err:#?}"), human, 0),
        (format!("{err:#}"), typed, 0),
    ] {
        assert_io_payload_report(&report, expected, locations, &counts);
    }
    let err = err.into_error();
    let child = err
        .iter_errors()
        .filter_map(|error| error.downcast_ref::<Sensitive>())
        .find(|error| error.0 == "child")
        .expect("the nested concrete error remains downcastable");
    assert!(
        std::error::Error::source(child).is_some_and(<dyn std::error::Error>::is::<Message>),
        "rendering preserves the concrete native source"
    );
    assert_eq!(
        format!("{child:#}"),
        "LEAKED child",
        "the custom error's own alternate Display behavior is unchanged"
    );

    let test_error = gix_error::TestError::from(err);
    let (expected, locations) = if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        (
            "outer

Caused by:
    0: inner
    1: child
    2: native leaf",
            3,
        )
    } else {
        (human, 4)
    };
    for (report, locations) in [(format!("{test_error:?}"), locations), (format!("{test_error:#?}"), 0)] {
        assert_io_payload_report(&report, expected, locations, &counts);
    }
}

#[test]
fn io_payload_free_errors_keep_their_own_labels() {
    use std::io::{Error, ErrorKind};

    for io in [
        Error::from(ErrorKind::NotFound),
        Error::from(ErrorKind::PermissionDenied),
        Error::from_raw_os_error(2),
    ] {
        assert!(io.get_ref().is_none(), "the control case has no custom payload");
        let human = io.to_string();
        let typed = format!("{io:?}");
        let err = io.raise_typed();
        for (report, expected, locations) in [
            (format!("{err:?}"), human.as_str(), 1),
            (format!("{err:#?}"), human.as_str(), 0),
            (format!("{err:#}"), typed.as_str(), 0),
        ] {
            assert_io_payload_report(&report, expected, locations, &[(expected, 1), ("I/O error (", 0)]);
        }
    }
}

#[test]
fn bail_formats_messages() {
    let unknown = "other";
    for (result, expected) in [
        ((|| -> ExnMessageResult { bail!("An error") })(), "An error"),
        ((|| -> ExnMessageResult { bail!("{{{unknown}}}",) })(), "{other}"),
        (
            (|| -> ExnMessageResult { bail!("{}: exit code {}", unknown, 128) })(),
            "other: exit code 128",
        ),
        (
            (|| -> ExnMessageResult { bail!("{unknown}: exit code {code:#x}", code = 128,) })(),
            "other: exit code 0x80",
        ),
        (
            (|| -> ExnMessageResult { bail!(concat!("unknown ", "{}"), unknown) })(),
            "unknown other",
        ),
    ] {
        let err = result.expect_err("bail returns a formatted message as an exception");
        assert_eq!(err.error().to_string(), expected, "format syntax follows message!()");
    }
}

#[test]
fn bail_formats_public_errors_at_the_call_site() {
    use gix_error::Result;

    let unknown = "other";
    for explicit_arguments in [false, true] {
        let mut invocation_line = 0;
        let err = (|| -> Result {
            if explicit_arguments {
                invocation_line = line!() + 1;
                bail!("Executable '{}' cannot be launched.", unknown);
            }
            invocation_line = line!() + 1;
            bail!("Executable '{unknown}' cannot be launched.");
        })()
        .expect_err("both format forms convert to a public error");
        assert_eq!(
            err.downcast_any_ref::<Message>()
                .expect("format syntax retains Message as the concrete error type")
                .to_string(),
            message!("Executable '{unknown}' cannot be launched.").to_string(),
            "formatting produces the same diagnostic as an explicit message!() call"
        );
        let location = err
            .iter_errors_with_locations()
            .next()
            .expect("the message is the root error")
            .location()
            .expect("bail records its invocation site");
        assert_eq!(location.file(), file!(), "the caller is outside the macro definition");
        assert_eq!(
            location.line(),
            invocation_line,
            "formatting preserves the bail call site"
        );
    }
}

#[test]
fn bail_preserves_error_expressions() {
    use gix_error::ExnResult;

    let mut evaluations = 0;
    let native = (|| -> ExnResult<(), std::io::Error> {
        bail!({
            evaluations += 1;
            std::io::Error::from(std::io::ErrorKind::NotFound)
        });
    })()
    .expect_err("an error expression retains its concrete exception type");
    assert_eq!(evaluations, 1, "the error expression is evaluated exactly once");
    assert_eq!(
        native.error().kind(),
        std::io::ErrorKind::NotFound,
        "concrete errors are not converted to formatted messages"
    );

    let original = native.raise(message("could not read configuration"));
    let frame = std::ptr::from_ref(original.frame());
    let err =
        (|| -> ExnMessageResult { bail!(original,) })().expect_err("existing exceptions also accept a trailing comma");
    assert!(
        std::ptr::eq(err.frame(), frame),
        "an existing exception retains its frame, source, and original caller location"
    );
}

#[test]
fn bail_formats_classified_messages() {
    let unknown = "other";
    for (result, expected) in [
        ((|| -> ExnMessageResult { bail!("An error".not_found()) })(), "An error"),
        (
            (|| -> ExnMessageResult { bail!("{{{unknown}}}".not_found(),) })(),
            "{other}",
        ),
        (
            (|| -> ExnMessageResult { bail!("{}: exit code {}".not_found(), unknown, 128,) })(),
            "other: exit code 128",
        ),
        (
            (|| -> ExnMessageResult { bail!("{unknown}: exit code {code:#x}".not_found(), code = 128,) })(),
            "other: exit code 0x80",
        ),
        (
            (|| -> ExnMessageResult { bail!(r#"unknown "{unknown}""#.not_found()) })(),
            r#"unknown "other""#,
        ),
    ] {
        let err = result.expect_err("builder shorthand returns a formatted message as an exception");
        assert_eq!(err.error().message, expected, "builders preserve format syntax");
        assert!(err.is_not_found(), "the builder classifies the formatted message");
        assert!(err.error().values.is_empty(), "classification does not invent metadata");
    }
}

#[test]
fn bail_chains_message_builders() {
    use gix_error::{Class, MetadataValue, ResourceExhaustionKind};

    let path = std::path::Path::new("refs");
    let offset = 42_u64;
    for result in [
        (|| -> ExnMessageResult {
            bail!(
                "missing {offset}"
                    .not_found()
                    .with("path", path)
                    .with("offset", offset)
                    .with_program("editor")
            );
        })(),
        (|| -> ExnMessageResult {
            bail!(
                "missing {}"
                    .with("offset", 0_u64)
                    .with("path", path,)
                    .not_found()
                    .with("offset", offset)
                    .with_program("editor"),
                offset,
            );
        })(),
        (|| -> ExnMessageResult {
            bail!(
                "missing {value}"
                    .with_program("editor")
                    .with("path", path)
                    .with("offset", offset)
                    .with_class(Class::NotFound),
                value = offset,
            );
        })(),
    ] {
        let err = result.expect_err("builder chains return a typed message exception");
        let message = err.error();
        assert_eq!(message.message, "missing 42", "formatting precedes metadata builders");
        assert!(
            err.is_not_found(),
            "classification works before or after metadata builders"
        );
        assert_eq!(message.values.len(), 3, "builders share one metadata dictionary");
        assert_eq!(
            message.values["path"],
            MetadataValue::Path(path.into()),
            "native paths are retained"
        );
        assert_eq!(
            message.values["offset"],
            MetadataValue::U64(offset),
            "later values replace earlier values"
        );
        assert_eq!(
            message.values["program"],
            MetadataValue::Path("editor".into()),
            "schema builders work too"
        );
        assert_eq!(err.metadata().count(), 1, "builder chains do not create extra contexts");
        assert!(
            std::error::Error::source(message).is_none(),
            "builders do not add synthetic causes"
        );
    }

    let err = (|| -> ExnMessageResult {
        bail!(
            "allocation at {offset}"
                .validation()
                .resource_exhaustion(ResourceExhaustionKind::AllocationLimit)
        );
    })()
    .expect_err("argument-taking class builders work in a chain");
    assert_eq!(
        err.error().class,
        Some(Class::ResourceExhaustion(ResourceExhaustionKind::AllocationLimit)),
        "the last class builder replaces previous classifications"
    );

    let err = (|| -> ExnMessageResult {
        bail!("missing {path:?}".with("path", path));
    })()
    .expect_err("metadata builders do not require a classification builder");
    assert_eq!(
        err.error().message,
        "missing \"refs\"",
        "metadata-only chains retain formatting"
    );
    assert_eq!(err.error().class, None, "metadata alone leaves the error unclassified");
    assert_eq!(
        err.error().values["path"],
        MetadataValue::Path(path.into()),
        "metadata-only chains retain values"
    );
}

#[test]
fn bail_preserves_parenthesized_literal_error_expressions() {
    use gix_error::ExnResult;

    let err = (|| -> ExnResult<(), std::num::ParseIntError> {
        bail!(("bad".parse::<u32>().expect_err("invalid number")));
    })()
    .expect_err("parentheses keep method calls on the literal instead of a formatted message");
    assert_eq!(
        err.error().kind(),
        &std::num::IntErrorKind::InvalidDigit,
        "the concrete error type is retained"
    );
}

#[test]
fn bail_builder_arguments_are_evaluated_once_in_order() {
    use gix_error::{MetadataValue, Result};

    let mut evaluations = Vec::new();
    let err = (|| -> Result {
        bail!(
            "missing {}"
                .not_found()
                .with(
                    {
                        evaluations.push("key");
                        "path"
                    },
                    {
                        evaluations.push("value");
                        std::path::Path::new("refs")
                    },
                )
                .with_program({
                    evaluations.push("program");
                    "editor"
                }),
            {
                evaluations.push("format");
                "refs"
            },
        );
    })()
    .expect_err("builder shorthand converts to a public error");
    assert_eq!(
        evaluations,
        ["format", "key", "value", "program"],
        "format and builder arguments run once in method-call order"
    );
    let message = err
        .downcast_any_ref::<Message>()
        .expect("the formatted message remains concrete");
    assert_eq!(
        message.message, "missing refs",
        "format arguments are evaluated before builders"
    );
    assert_eq!(
        message.values["path"],
        MetadataValue::Path("refs".into()),
        "builder arguments populate metadata"
    );
    assert!(err.is_not_found(), "public errors retain the builder classification");
}

#[test]
fn bail_builder_shorthand_records_the_call_site() {
    use gix_error::Result;

    let unknown = "other";
    for form in 0..3 {
        let mut invocation_line = 0;
        let err = (|| -> Result {
            match form {
                0 => {
                    invocation_line = line!() + 1;
                    bail!("unknown {unknown}".not_found().with_input(unknown));
                }
                1 => {
                    invocation_line = line!() + 1;
                    bail!("unknown {}".not_found().with_input(unknown), unknown);
                }
                _ => {
                    invocation_line = line!() + 1;
                    bail!("unknown {unknown}".with_input(unknown).not_found_error());
                }
            }
        })()
        .expect_err("all builder forms convert to public errors");
        assert!(
            err.is_not_found(),
            "both message and error builders retain classification"
        );
        let message = err
            .downcast_any_ref::<Message>()
            .expect("the builder preserves the message type");
        assert_eq!(message.message, "unknown other", "all forms retain formatting");
        assert_eq!(err.metadata().count(), 1, "raising builders do not duplicate metadata");
        for source in err.iter_errors_with_locations() {
            let location = source
                .location()
                .expect("bail and raising builders record their invocation site");
            assert_eq!(location.file(), file!(), "the location is outside the macro definition");
            assert_eq!(
                location.line(),
                invocation_line,
                "builder shorthand preserves the invocation line"
            );
        }
    }
}

#[test]
fn ensure_formats_messages_and_builder_chains() {
    use gix_error::{MetadataValue, ensure};

    let unknown = "other";
    for (result, expected) in [
        (
            (|| -> ExnMessageResult {
                ensure!(false, "An error");
                Ok(())
            })(),
            "An error",
        ),
        (
            (|| -> ExnMessageResult {
                ensure!(false, "{{{unknown}}}",);
                Ok(())
            })(),
            "{other}",
        ),
        (
            (|| -> ExnMessageResult {
                ensure!(false, "{}: exit code {}", unknown, 128,);
                Ok(())
            })(),
            "other: exit code 128",
        ),
        (
            (|| -> ExnMessageResult {
                ensure!(false, "{unknown}: exit code {code:#x}", code = 128,);
                Ok(())
            })(),
            "other: exit code 0x80",
        ),
        (
            (|| -> ExnMessageResult {
                ensure!(false, concat!("unknown ", "{}"), unknown);
                Ok(())
            })(),
            "unknown other",
        ),
    ] {
        let err = result.expect_err("a failed condition returns a formatted message");
        assert_eq!(err.error().message, expected, "ensure forwards format syntax to bail");
        assert_eq!(err.error().class, None, "formatting alone leaves errors unclassified");
    }

    for result in [
        (|| -> ExnMessageResult {
            ensure!(
                false,
                "unknown {unknown}"
                    .not_found()
                    .with_input(unknown)
                    .with_program("editor")
            );
            Ok(())
        })(),
        (|| -> ExnMessageResult {
            ensure!(
                false,
                "unknown {}".with_input(unknown,).not_found().with_program("editor"),
                unknown,
            );
            Ok(())
        })(),
        (|| -> ExnMessageResult {
            ensure!(
                false,
                "unknown {name}".not_found().with_input(unknown).with_program("editor"),
                name = unknown,
            );
            Ok(())
        })(),
    ] {
        let err = result.expect_err("a failed condition applies message builders");
        assert_eq!(err.error().message, "unknown other", "builder chains retain formatting");
        assert!(err.is_not_found(), "ensure retains the builder classification");
        assert_eq!(
            err.error().values["input"],
            MetadataValue::from(unknown),
            "with_input retains metadata"
        );
        assert_eq!(
            err.error().values["program"],
            MetadataValue::Path("editor".into()),
            "schema builders work in chains"
        );
        assert_eq!(err.metadata().count(), 1, "builder chains use one diagnostic context");
    }

    let err = (|| -> ExnMessageResult {
        ensure!(false, "unknown {unknown}".with_input(unknown));
        Ok(())
    })()
    .expect_err("metadata builders do not require classification");
    assert_eq!(
        err.error().class,
        None,
        "metadata-only builders leave errors unclassified"
    );
    assert_eq!(
        err.error().values["input"],
        MetadataValue::from(unknown),
        "metadata-only chains retain values"
    );
}

#[test]
fn ensure_evaluates_error_arguments_only_on_failure() -> gix_error::TestResult {
    use gix_error::{Result, ensure};

    for condition in [true, false] {
        let mut evaluations = Vec::new();
        let result = (|| -> Result {
            ensure!(
                {
                    evaluations.push("condition");
                    condition
                },
                "unknown {}"
                    .not_found()
                    .with(
                        {
                            evaluations.push("key");
                            "input"
                        },
                        {
                            evaluations.push("value");
                            "other"
                        },
                    )
                    .with_program({
                        evaluations.push("program");
                        "editor"
                    }),
                {
                    evaluations.push("format");
                    "other"
                },
            );
            Ok(())
        })();
        if condition {
            result?;
            assert_eq!(
                evaluations,
                ["condition"],
                "successful conditions do not evaluate format or builder arguments"
            );
        } else {
            let err = result.expect_err("a failed condition returns an error");
            assert!(err.is_not_found(), "the failure applies the builder classification");
            assert_eq!(
                evaluations,
                ["condition", "format", "key", "value", "program"],
                "condition and error arguments run once in order"
            );
        }
    }
    Ok(())
}

#[test]
fn ensure_builder_shorthand_records_the_call_site() {
    use gix_error::{Result, ensure};

    let unknown = "other";
    for form in 0..3 {
        let mut invocation_line = 0;
        let err = (|| -> Result {
            match form {
                0 => {
                    invocation_line = line!() + 1;
                    ensure!(false, "unknown {unknown}".not_found().with_input(unknown));
                }
                1 => {
                    invocation_line = line!() + 1;
                    ensure!(false, "unknown {}".not_found().with_input(unknown), unknown);
                }
                _ => {
                    invocation_line = line!() + 1;
                    ensure!(false, "unknown {unknown}".with_input(unknown).not_found_error());
                }
            }
            Ok(())
        })()
        .expect_err("failed conditions return public errors");
        assert!(err.is_not_found(), "message and raising builders retain classification");
        assert_eq!(err.metadata().count(), 1, "raising builders do not duplicate metadata");
        for source in err.iter_errors_with_locations() {
            let location = source.location().expect("ensure records the failure site");
            assert_eq!(location.file(), file!(), "the location is outside the macro definition");
            assert_eq!(
                location.line(),
                invocation_line,
                "ensure retains its invocation line through bail"
            );
        }
    }
}

#[test]
fn ensure_preserves_error_expressions() {
    use gix_error::{ExnResult, ensure};

    let err = (|| -> ExnResult<(), std::num::ParseIntError> {
        ensure!(false, ("bad".parse::<u32>().expect_err("invalid number")),);
        Ok(())
    })()
    .expect_err("parenthesized literal expressions keep their concrete error type");
    assert_eq!(
        err.error().kind(),
        &std::num::IntErrorKind::InvalidDigit,
        "ordinary expressions are not formatted as messages"
    );

    let original = err.raise(message("invalid configuration"));
    let frame = std::ptr::from_ref(original.frame());
    let err = (|| -> ExnMessageResult {
        ensure!(false, original,);
        Ok(())
    })()
    .expect_err("existing exceptions remain supported");
    assert!(
        std::ptr::eq(err.frame(), frame),
        "existing exceptions retain their frame and original location"
    );
}

#[test]
fn numbered_reports_flatten_chains_and_retain_branch_hierarchy() {
    use gix_error::ClassificationMarker;

    let username = ErrorWithSource(
        "Username '-Fconfigfile' could be mistaken for a command-line argument",
        ClassificationMarker::VALIDATION,
    );
    let chain = username
        .raise_typed()
        .raise(message("Failed to prepare SSH invocation"))
        .raise(message("Transport handshake failed"));
    insta::assert_debug_snapshot!(chain, "a native classification marker does not add a numbered cause", @"
    Transport handshake failed

    Caused by:
        0: Failed to prepare SSH invocation
        1: Username '-Fconfigfile' could be mistaken for a command-line argument
    ");

    let branch = message("Failed to prepare SSH invocation")
        .raise_typed()
        .chain(ClassificationMarker::VALIDATION)
        .chain(ErrorWithSource("another cause", message("source")))
        .chain(message("yet another cause"));
    let tree = message("Transport handshake failed")
        .raise_typed()
        .chain(ClassificationMarker::VALIDATION)
        .chain(branch)
        .chain(message("another failure"));
    insta::assert_debug_snapshot!(tree, "filtered markers leave branch heads in place with their causes beneath them", @"
    Transport handshake failed

    Caused by:
        0: Failed to prepare SSH invocation
        ├─0: another cause
        │ └─0: source
        └─1: yet another cause
        1: another failure
    ");
    assert!(tree.is_validation(), "hidden markers still classify the error tree");

    let tree = tree.raise(message("Clone failed"));
    insta::assert_debug_snapshot!(tree, "a main-chain prefix retains branch heads at every fork", @"
    Clone failed

    Caused by:
        0: Transport handshake failed
        ├─0: Failed to prepare SSH invocation
        │ ├─0: another cause
        │ │ └─0: source
        │ └─1: yet another cause
        └─1: another failure
    ");
}

#[test]
fn numbered_reports_continue_local_indices_and_guides_through_nested_chains() {
    let first = (0..11).rev().fold(
        message("first fork").raise_all([message("first left"), message("first right")]),
        |chain, index| chain.raise(message!("chain {index}")),
    );
    let last = message("last fork")
        .raise_all([message("last left"), message("last right")])
        .raise(message("last middle"))
        .raise(message("last chain"));
    let group = message("group").raise_all([first, last]);
    let tail = message("tail leaf").raise_typed().raise(message("tail chain"));
    let report = message("outer fork").raise_all([group, tail]).raise(message("report"));
    insta::assert_debug_snapshot!(report, "branch heads retain sibling indices; eleven causes below them reach index 10 and nest terminal forks, with ancestor bars only for enclosing siblings", @"
    report

    Caused by:
        0: outer fork
        ├─0: group
        │ ├─0: chain 0
        │ │ ├─0: chain 1
        │ │ ├─1: chain 2
        │ │ ├─2: chain 3
        │ │ ├─3: chain 4
        │ │ ├─4: chain 5
        │ │ ├─5: chain 6
        │ │ ├─6: chain 7
        │ │ ├─7: chain 8
        │ │ ├─8: chain 9
        │ │ ├─9: chain 10
        │ │ └─10: first fork
        │ │   ├─0: first left
        │ │   └─1: first right
        │ └─1: last chain
        │   ├─0: last middle
        │   └─1: last fork
        │     ├─0: last left
        │     └─1: last right
        └─1: tail chain
          └─0: tail leaf
    ");

    let boundary = message("boundary")
        .raise_all([message("left"), message("right")])
        .into_error();
    let sibling = message("sibling").raise_typed().chain(message("leaf"));
    let group = message("group").raise_all([boundary.raise_typed().erased(), sibling.erased()]);
    let report = message("report")
        .raise_typed()
        .chain(group)
        .chain(message("outer sibling"));
    insta::assert_debug_snapshot!(report, "already flattened nested-boundary sources start a new level beneath their branch head, leaving subsequent siblings in place", @"
    report

    Caused by:
        0: group
        ├─0: boundary
        │ ├─0: left
        │ └─1: right
        └─1: sibling
          └─0: leaf
        1: outer sibling
    ");
}
