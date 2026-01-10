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

use crate::{debug_string, new_tree_error, Error};
use gix_error::ErrorExt;
use gix_error::Exn;
use gix_error::OptionExt;
use gix_error::ResultExt;

#[test]
fn error_call_chain() {
    let e1 = Error("E1").raise();
    let e2 = e1.raise(Error("E2"));
    let e3 = e2.raise(Error("E3"));
    let e4 = e3.raise(Error("E4"));
    let e5 = e4.raise(Error("E5"));
    insta::assert_snapshot!(debug_string(e5), @r"
    E5, at gix-error/tests/error/exn.rs:27:17
    |
    |-> E4, at gix-error/tests/error/exn.rs:26:17
    |
    |-> E3, at gix-error/tests/error/exn.rs:25:17
    |
    |-> E2, at gix-error/tests/error/exn.rs:24:17
    |
    |-> E1, at gix-error/tests/error/exn.rs:23:26
    ");
}

#[test]
fn error_tree() {
    let err = new_tree_error();
    insta::assert_snapshot!(debug_string(err), @r"
    E6, at gix-error/tests/error/main.rs:25:9
    |
    |-> E5, at gix-error/tests/error/main.rs:17:18
    |   |
    |   |-> E3, at gix-error/tests/error/main.rs:9:21
    |   |   |
    |   |   |-> E1, at gix-error/tests/error/main.rs:8:30
    |   |
    |   |-> E10, at gix-error/tests/error/main.rs:12:22
    |   |   |
    |   |   |-> E9, at gix-error/tests/error/main.rs:11:30
    |   |
    |   |-> E12, at gix-error/tests/error/main.rs:15:23
    |       |
    |       |-> E11, at gix-error/tests/error/main.rs:14:32
    |
    |-> E4, at gix-error/tests/error/main.rs:20:21
    |   |
    |   |-> E2, at gix-error/tests/error/main.rs:19:30
    |
    |-> E8, at gix-error/tests/error/main.rs:23:21
        |
        |-> E7, at gix-error/tests/error/main.rs:22:30
    ");
}

#[test]
fn result_ext() {
    let result: Result<(), Error> = Err(Error("An error"));
    let result = result.or_raise(|| Error("Another error"));
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @r"
    Another error, at gix-error/tests/error/exn.rs:74:25
    |
    |-> An error, at gix-error/tests/error/exn.rs:74:25
    ");
}

#[test]
fn option_ext() {
    let result: Option<()> = None;
    let result = result.ok_or_raise(|| Error("An error"));
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:85:25");
}

#[test]
fn from_error() {
    fn foo() -> Result<(), Exn<Error>> {
        Err(Error("An error"))?;
        Ok(())
    }

    let result = foo();
    insta::assert_snapshot!(debug_string(result.unwrap_err()),@"An error, at gix-error/tests/error/exn.rs:92:9");
}

#[test]
fn bail() {
    fn foo() -> Result<(), Exn<Error>> {
        gix_error::bail!(Error("An error"));
    }

    let result = foo();
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:103:9");
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
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:123:9");
}

#[test]
fn result_ok() -> Result<(), Exn<Error>> {
    Ok(())
}
