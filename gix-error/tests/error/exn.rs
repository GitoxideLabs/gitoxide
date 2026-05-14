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

use crate::{ErrorWithSource, debug_string, fixup_paths, new_tree_error};
use gix_error::OptionExt;
use gix_error::ResultExt;
use gix_error::{ErrorExt, message};
use gix_error::{Exn, Message};

#[test]
fn raise_chain() {
    let e1 = message("E1").raise();
    let e2 = e1.raise(message("E2"));
    let e3 = e2.raise(message("E3"));
    let e4 = e3.raise(message("E4"));
    let e5 = e4.raise(message("E5"));
    insta::assert_debug_snapshot!(e5, @r"
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
    insta::assert_snapshot!(debug_string(&e5), @"
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
    insta::assert_debug_snapshot!(e, @r"
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
    insta::assert_snapshot!(format!("{e:#}"), @r#"
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
    insta::assert_snapshot!(format!("{e:}"), @"E5");

    insta::assert_snapshot!(debug_string(&e), @"
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
    insta::assert_debug_snapshot!(e, @r"
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

    insta::assert_snapshot!(format!("{e:#}"), @r#"
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
    insta::assert_debug_snapshot!(exn, @r"
    could not read config
    |
    └─ file not found
    ");

    // and_raise is equivalent to raise().raise() (compare with {:#?} to omit locations)
    let io_err2 = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let exn2 = io_err2.raise().raise(message("could not read config"));
    assert_eq!(format!("{exn:#?}"), format!("{exn2:#?}"));
}

#[test]
fn raise_all() {
    let e = message("Top").raise_all(
        (1..5).map(|idx| message!("E{}", idx).raise_all((0..idx).map(|sidx| message!("E{}-{}", idx, sidx)))),
    );
    insta::assert_debug_snapshot!(e, @r"
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
    insta::assert_snapshot!(debug_string(&e), @"
    Top, at gix-error/tests/error/exn.rs:138
    |
    └─ E1, at gix-error/tests/error/exn.rs:139
    |   |
    |   └─ E1-0, at gix-error/tests/error/exn.rs:139
    |
    └─ E2, at gix-error/tests/error/exn.rs:139
    |   |
    |   └─ E2-0, at gix-error/tests/error/exn.rs:139
    |   |
    |   └─ E2-1, at gix-error/tests/error/exn.rs:139
    |
    └─ E3, at gix-error/tests/error/exn.rs:139
    |   |
    |   └─ E3-0, at gix-error/tests/error/exn.rs:139
    |   |
    |   └─ E3-1, at gix-error/tests/error/exn.rs:139
    |   |
    |   └─ E3-2, at gix-error/tests/error/exn.rs:139
    |
    └─ E4, at gix-error/tests/error/exn.rs:139
        |
        └─ E4-0, at gix-error/tests/error/exn.rs:139
        |
        └─ E4-1, at gix-error/tests/error/exn.rs:139
        |
        └─ E4-2, at gix-error/tests/error/exn.rs:139
        |
        └─ E4-3, at gix-error/tests/error/exn.rs:139
    ");

    let e = e.chain_all((1..3).map(|idx| message!("SE{}", idx)));
    insta::assert_debug_snapshot!(e, @r"
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
    |   |
    |   └─ E4-0
    |   |
    |   └─ E4-1
    |   |
    |   └─ E4-2
    |   |
    |   └─ E4-3
    |
    └─ SE1
    |
    └─ SE2
    ");

    insta::assert_snapshot!(format!("{:#}", e), @r#"
    Message("Top")
    |
    └─ Message("E1")
    |   |
    |   └─ Message("E1-0")
    |
    └─ Message("E2")
    |   |
    |   └─ Message("E2-0")
    |   |
    |   └─ Message("E2-1")
    |
    └─ Message("E3")
    |   |
    |   └─ Message("E3-0")
    |   |
    |   └─ Message("E3-1")
    |   |
    |   └─ Message("E3-2")
    |
    └─ Message("E4")
    |   |
    |   └─ Message("E4-0")
    |   |
    |   └─ Message("E4-1")
    |   |
    |   └─ Message("E4-2")
    |   |
    |   └─ Message("E4-3")
    |
    └─ Message("SE1")
    |
    └─ Message("SE2")
    "#);
    let _this_should_compile = message("Top-untyped").raise_all((1..5).map(|idx| message!("E{}", idx).raise_erased()));

    assert_eq!(
        e.into_error().probable_cause().to_string(),
        "Top",
        "sometimes the cause is too ambiguous"
    );
}

#[test]
fn inverse_error_call_chain() {
    let e1 = message("E1").raise();
    let e2 = e1.chain(message("E2"));
    let e3 = e2.chain(message("E3"));
    let e4 = e3.chain(message("E4"));
    let e5 = e4.chain(message("E5"));
    insta::assert_debug_snapshot!(e5, @r"
    E1
    |
    └─ E2
    |
    └─ E3
    |
    └─ E4
    |
    └─ E5
    ");
    insta::assert_snapshot!(debug_string(&e5), @"
    E1, at gix-error/tests/error/exn.rs:287
    |
    └─ E2, at gix-error/tests/error/exn.rs:288
    |
    └─ E3, at gix-error/tests/error/exn.rs:289
    |
    └─ E4, at gix-error/tests/error/exn.rs:290
    |
    └─ E5, at gix-error/tests/error/exn.rs:291
    ");

    insta::assert_snapshot!(format!("{e5:#}"), @r#"
    Message("E1")
    |
    └─ Message("E2")
    |
    └─ Message("E3")
    |
    └─ Message("E4")
    |
    └─ Message("E5")
    "#);

    assert_eq!(e5.into_error().probable_cause().to_string(), "E5");
}

#[test]
fn error_tree() {
    let mut err = new_tree_error();
    insta::assert_debug_snapshot!(err, @r"
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
    insta::assert_snapshot!(debug_string(&err), @"
    E6, at gix-error/tests/error/main.rs:25
    |
    └─ E5, at gix-error/tests/error/main.rs:17
    |   |
    |   └─ E3, at gix-error/tests/error/main.rs:9
    |   |   |
    |   |   └─ E1, at gix-error/tests/error/main.rs:8
    |   |
    |   └─ E10, at gix-error/tests/error/main.rs:12
    |   |   |
    |   |   └─ E9, at gix-error/tests/error/main.rs:11
    |   |
    |   └─ E12, at gix-error/tests/error/main.rs:15
    |       |
    |       └─ E11, at gix-error/tests/error/main.rs:14
    |
    └─ E4, at gix-error/tests/error/main.rs:20
    |   |
    |   └─ E2, at gix-error/tests/error/main.rs:19
    |
    └─ E8, at gix-error/tests/error/main.rs:23
        |
        └─ E7, at gix-error/tests/error/main.rs:22
    ");
    insta::assert_debug_snapshot!(err.frame().iter_frames().map(ToString::to_string).collect::<Vec<_>>(), @r#"
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
    insta::assert_debug_snapshot!(new_e, @r"
    E-New
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
    insta::assert_snapshot!(err, @"E6");
}

#[test]
fn result_ext() {
    let result: Result<(), Message> = Err(message("An error"));
    let result = result.or_raise(|| message("Another error"));
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"
    Another error, at gix-error/tests/error/exn.rs:432
    |
    └─ An error, at gix-error/tests/error/exn.rs:432
    ");
}

#[test]
fn option_ext() {
    let result: Option<()> = None;
    let result = result.ok_or_raise(|| message("An error"));
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:443");
}

#[test]
fn from_message() {
    fn foo() -> Result<(), Exn<Message>> {
        Err(message("An error"))?;
        Ok(())
    }

    let result = foo();
    insta::assert_snapshot!(debug_string(result.unwrap_err()),@"An error, at gix-error/tests/error/exn.rs:450");
}

#[test]
fn new_with_source() {
    let e = Exn::new(ErrorWithSource("top", message("source")));
    insta::assert_debug_snapshot!(e,@r"
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
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:471");
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
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:491");
}

#[test]
fn result_ok() -> Result<(), Exn<Message>> {
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
    let e = message("E1").raise().erased();
    let _into_error_works = e.into_error();
}

#[cfg(feature = "anyhow")]
#[test]
fn raise_chain_anyhow() {
    let e1 = message("E1")
        .raise()
        .chain(Exn::raise_all([message("E1c1-1"), message("E1c1-2")], message("E1-2")))
        .chain(Exn::raise_all([message("E1c2-1"), message("E1c2-2")], message("E1-3")));
    let e2 = e1.raise(message("E2"));
    let root = e2.raise(Message::new("root"));

    // It's a linked list as linked up with the first child, but also has multiple children.
    insta::assert_snapshot!(format!("{root:#}"), @r#"
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

    insta::assert_snapshot!(remove_stackstrace(format!("{:?}", anyhow::Error::from(root))), @"
    root, at gix-error/tests/error/exn.rs:530

    Caused by:
        0: E2, at gix-error/tests/error/exn.rs:529
        1: E1, at gix-error/tests/error/exn.rs:526
        2: E1-2, at gix-error/tests/error/exn.rs:527
        3: E1-3, at gix-error/tests/error/exn.rs:528
        4: E1c1-1, at gix-error/tests/error/exn.rs:527
        5: E1c1-2, at gix-error/tests/error/exn.rs:527
        6: E1c2-1, at gix-error/tests/error/exn.rs:528
        7: E1c2-2, at gix-error/tests/error/exn.rs:528
    ");
}

#[cfg(feature = "anyhow")]
#[test]
fn inverse_error_call_chain_anyhow() {
    let e1 = message("E1").raise();
    let e2 = e1.chain(message("E2"));
    let e3 = e2.chain(message("E3"));
    let e4 = e3.chain(message("E4"));
    let e5 = e4.chain(message("E5"));
    insta::assert_debug_snapshot!(e5, @"
    E1
    |
    └─ E2
    |
    └─ E3
    |
    └─ E4
    |
    └─ E5
    ");

    insta::assert_snapshot!(remove_stackstrace(format!("{:?}", anyhow::Error::from(e5))), @"
    E1, at gix-error/tests/error/exn.rs:571

    Caused by:
        0: E2, at gix-error/tests/error/exn.rs:572
        1: E3, at gix-error/tests/error/exn.rs:573
        2: E4, at gix-error/tests/error/exn.rs:574
        3: E5, at gix-error/tests/error/exn.rs:575
    ");
}

fn remove_stackstrace(s: String) -> String {
    fixup_paths(s.find("Stack backtrace:").map_or(s.clone(), |pos| s[..pos].into()))
}

#[test]
fn into_chain() {
    let e1 = message("E1")
        .raise()
        .chain(Exn::raise_all([message("E1c1-1"), message("E1c1-2")], message("E1-2")))
        .chain(Exn::raise_all([message("E1c2-1"), message("E1c2-2")], message("E1-3")));
    let e2 = e1.raise(message("E2"));
    let root = e2.raise(Message::new("root"));

    insta::assert_snapshot!(format!("{root:#}"), @r#"
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
    insta::assert_debug_snapshot!(causes_display(&root, Style::Normal), @r#"
    [
        "root, at gix-error/tests/error/exn.rs:610",
        "E2, at gix-error/tests/error/exn.rs:609",
        "E1, at gix-error/tests/error/exn.rs:606",
        "E1-2, at gix-error/tests/error/exn.rs:607",
        "E1-3, at gix-error/tests/error/exn.rs:608",
        "E1c1-1, at gix-error/tests/error/exn.rs:607",
        "E1c1-2, at gix-error/tests/error/exn.rs:607",
        "E1c2-1, at gix-error/tests/error/exn.rs:608",
        "E1c2-2, at gix-error/tests/error/exn.rs:608",
    ]
    "#);

    // But these can also be turned off
    insta::assert_debug_snapshot!(causes_display(&root, Style::Alternate), @r#"
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
    insta::assert_snapshot!(remove_stackstrace(format!("{:?}", anyhow::Error::from(root))), @"
    root, at gix-error/tests/error/exn.rs:610

    Caused by:
        0: E2, at gix-error/tests/error/exn.rs:609
        1: E1, at gix-error/tests/error/exn.rs:606
        2: E1-2, at gix-error/tests/error/exn.rs:607
        3: E1-3, at gix-error/tests/error/exn.rs:608
        4: E1c1-1, at gix-error/tests/error/exn.rs:607
        5: E1c1-2, at gix-error/tests/error/exn.rs:607
        6: E1c2-1, at gix-error/tests/error/exn.rs:608
        7: E1c2-2, at gix-error/tests/error/exn.rs:608
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
