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

use crate::{debug_string, new_tree_error, Error, ErrorWithSource};
use gix_error::Exn;
use gix_error::OptionExt;
use gix_error::ResultExt;
use gix_error::{message, ErrorExt};

#[test]
fn raise_chain() {
    let e1 = Error("E1").raise();
    let e2 = e1.raise(Error("E2"));
    let e3 = e2.raise(Error("E3"));
    let e4 = e3.raise(Error("E4"));
    let e5 = e4.raise(Error("E5"));
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
    insta::assert_snapshot!(debug_string(&e5), @r"
    E5, at gix-error/tests/error/exn.rs:27:17
    |
    └─ E4, at gix-error/tests/error/exn.rs:26:17
    |
    └─ E3, at gix-error/tests/error/exn.rs:25:17
    |
    └─ E2, at gix-error/tests/error/exn.rs:24:17
    |
    └─ E1, at gix-error/tests/error/exn.rs:23:26
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

    insta::assert_snapshot!(debug_string(&e), @r"
    E5, at gix-error/tests/error/exn.rs:27:17
    |
    └─ E4, at gix-error/tests/error/exn.rs:26:17
    |
    └─ E3, at gix-error/tests/error/exn.rs:25:17
    |
    └─ E2, at gix-error/tests/error/exn.rs:24:17
    |
    └─ E1, at gix-error/tests/error/exn.rs:23:26
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

    insta::assert_snapshot!(debug_string(&e), @r"
    E5, at gix-error/tests/error/exn.rs:27:17
    |
    └─ E4, at gix-error/tests/error/exn.rs:26:17
    |
    └─ E3, at gix-error/tests/error/exn.rs:25:17
    |
    └─ E2, at gix-error/tests/error/exn.rs:24:17
    |
    └─ E1, at gix-error/tests/error/exn.rs:23:26
    ");
}

#[test]
fn raise_iter() {
    let e = Error("Top").raise_iter(
        (1..5).map(|idx| message!("E{}", idx).raise_iter((0..idx).map(|sidx| message!("E{}-{}", idx, sidx)))),
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
    insta::assert_snapshot!(debug_string(&e), @r"
    Top, at gix-error/tests/error/exn.rs:105:26
    |
    └─ E1, at gix-error/tests/error/exn.rs:106:47
    |   |
    |   └─ E1-0, at gix-error/tests/error/exn.rs:106:47
    |
    └─ E2, at gix-error/tests/error/exn.rs:106:47
    |   |
    |   └─ E2-0, at gix-error/tests/error/exn.rs:106:47
    |   |
    |   └─ E2-1, at gix-error/tests/error/exn.rs:106:47
    |
    └─ E3, at gix-error/tests/error/exn.rs:106:47
    |   |
    |   └─ E3-0, at gix-error/tests/error/exn.rs:106:47
    |   |
    |   └─ E3-1, at gix-error/tests/error/exn.rs:106:47
    |   |
    |   └─ E3-2, at gix-error/tests/error/exn.rs:106:47
    |
    └─ E4, at gix-error/tests/error/exn.rs:106:47
        |
        └─ E4-0, at gix-error/tests/error/exn.rs:106:47
        |
        └─ E4-1, at gix-error/tests/error/exn.rs:106:47
        |
        └─ E4-2, at gix-error/tests/error/exn.rs:106:47
        |
        └─ E4-3, at gix-error/tests/error/exn.rs:106:47
    ");

    let e = e.chain_iter((1..3).map(|idx| message!("SE{}", idx)));
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

    insta::assert_snapshot!(format!("{:#}", e), @r#"Error("Top")"#);
    let _this_should_compile = Error("Top-untyped").raise_iter((1..5).map(|idx| message!("E{}", idx).erased()));

    assert_eq!(e.into_error().leaf().to_string(), "SE2", "we always get the last leaf");
}

#[test]
fn inverse_error_call_chain() {
    let e1 = Error("E1").raise();
    let e2 = e1.chain(Error("E2"));
    let e3 = e2.chain(Error("E3"));
    let e4 = e3.chain(Error("E4"));
    let e5 = e4.chain(Error("E5"));
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
    insta::assert_snapshot!(debug_string(&e5), @r"
    E1, at gix-error/tests/error/exn.rs:216:26
    |
    └─ E2, at gix-error/tests/error/exn.rs:217:17
    |
    └─ E3, at gix-error/tests/error/exn.rs:218:17
    |
    └─ E4, at gix-error/tests/error/exn.rs:219:17
    |
    └─ E5, at gix-error/tests/error/exn.rs:220:17
    ");
}

#[test]
fn error_tree() {
    let err = new_tree_error();
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
    insta::assert_snapshot!(debug_string(err), @r"
    E6, at gix-error/tests/error/main.rs:25:9
    |
    └─ E5, at gix-error/tests/error/main.rs:17:18
    |   |
    |   └─ E3, at gix-error/tests/error/main.rs:9:21
    |   |   |
    |   |   └─ E1, at gix-error/tests/error/main.rs:8:30
    |   |
    |   └─ E10, at gix-error/tests/error/main.rs:12:22
    |   |   |
    |   |   └─ E9, at gix-error/tests/error/main.rs:11:30
    |   |
    |   └─ E12, at gix-error/tests/error/main.rs:15:23
    |       |
    |       └─ E11, at gix-error/tests/error/main.rs:14:32
    |
    └─ E4, at gix-error/tests/error/main.rs:20:21
    |   |
    |   └─ E2, at gix-error/tests/error/main.rs:19:30
    |
    └─ E8, at gix-error/tests/error/main.rs:23:21
        |
        └─ E7, at gix-error/tests/error/main.rs:22:30
    ");
}

#[test]
fn result_ext() {
    let result: Result<(), Error> = Err(Error("An error"));
    let result = result.or_raise(|| Error("Another error"));
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @r"
    Another error, at gix-error/tests/error/exn.rs:303:25
    |
    └─ An error, at gix-error/tests/error/exn.rs:303:25
    ");
}

#[test]
fn option_ext() {
    let result: Option<()> = None;
    let result = result.ok_or_raise(|| Error("An error"));
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:314:25");
}

#[test]
fn from_error() {
    fn foo() -> Result<(), Exn<Error>> {
        Err(Error("An error"))?;
        Ok(())
    }

    let result = foo();
    insta::assert_snapshot!(debug_string(result.unwrap_err()),@"An error, at gix-error/tests/error/exn.rs:321:9");
}

#[test]
fn new_with_source() {
    let e = Exn::new(ErrorWithSource("top", Error("source")));
    insta::assert_debug_snapshot!(e,@r"
    top
    |
    └─ source
    ");
}

#[test]
fn bail() {
    fn foo() -> Result<(), Exn<Error>> {
        gix_error::bail!(Error("An error"));
    }

    let result = foo();
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:342:9");
}

#[test]
fn ensure_ok() {
    fn foo() -> Result<(), Exn<Error>> {
        gix_error::ensure!(true, Error("An error"));
        Ok(())
    }

    foo().unwrap();
}

#[test]
fn ensure_fail() {
    fn foo() -> Result<(), Exn<Error>> {
        gix_error::ensure!(false, Error("An error"));
        Ok(())
    }

    let result = foo();
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:362:9");
}

#[test]
fn result_ok() -> Result<(), Exn<Error>> {
    Ok(())
}
