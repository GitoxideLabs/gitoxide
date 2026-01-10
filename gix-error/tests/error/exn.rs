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

use gix_error::ErrorExt;
use gix_error::Exn;
use gix_error::OptionExt;
use gix_error::ResultExt;

#[derive(Debug)]
struct Error(&'static str);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

#[test]
fn error_call_chain() {
    let e1 = Error("E1").raise();
    let e2 = e1.raise(Error("E2"));
    let e3 = e2.raise(Error("E3"));
    let e4 = e3.raise(Error("E4"));
    let e5 = e4.raise(Error("E5"));
    insta::assert_snapshot!(debug_string(e5), @r"
    E5, at gix-error/tests/error/exn.rs:37:17
    |
    |-> E4, at gix-error/tests/error/exn.rs:36:17
    |
    |-> E3, at gix-error/tests/error/exn.rs:35:17
    |
    |-> E2, at gix-error/tests/error/exn.rs:34:17
    |
    |-> E1, at gix-error/tests/error/exn.rs:33:26
    ");
}

#[test]
fn error_tree() {
    let e1 = Error("E1").raise();
    let e3 = e1.raise(Error("E3"));

    let e9 = Error("E9").raise();
    let e10 = e9.raise(Error("E10"));

    let e11 = Error("E11").raise();
    let e12 = e11.raise(Error("E12"));

    let e5 = Exn::from_iter([e3, e10, e12], Error("E5"));

    let e2 = Error("E2").raise();
    let e4 = e2.raise(Error("E4"));

    let e7 = Error("E7").raise();
    let e8 = e7.raise(Error("E8"));

    let e6 = Exn::from_iter([e5, e4, e8], Error("E6"));
    insta::assert_snapshot!(debug_string(e6), @r"
    E6, at gix-error/tests/error/exn.rs:70:14
    |
    |-> E5, at gix-error/tests/error/exn.rs:62:14
    |   |
    |   |-> E3, at gix-error/tests/error/exn.rs:54:17
    |   |   |
    |   |   |-> E1, at gix-error/tests/error/exn.rs:53:26
    |   |
    |   |-> E10, at gix-error/tests/error/exn.rs:57:18
    |   |   |
    |   |   |-> E9, at gix-error/tests/error/exn.rs:56:26
    |   |
    |   |-> E12, at gix-error/tests/error/exn.rs:60:19
    |       |
    |       |-> E11, at gix-error/tests/error/exn.rs:59:28
    |
    |-> E4, at gix-error/tests/error/exn.rs:65:17
    |   |
    |   |-> E2, at gix-error/tests/error/exn.rs:64:26
    |
    |-> E8, at gix-error/tests/error/exn.rs:68:17
        |
        |-> E7, at gix-error/tests/error/exn.rs:67:26
    ");
}

#[test]
fn result_ext() {
    let result: Result<(), Error> = Err(Error("An error"));
    let result = result.or_raise(|| Error("Another error"));
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @r"
    Another error, at gix-error/tests/error/exn.rs:101:25
    |
    |-> An error, at gix-error/tests/error/exn.rs:101:25
    ");
}

#[test]
fn option_ext() {
    let result: Option<()> = None;
    let result = result.ok_or_raise(|| Error("An error"));
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:112:25");
}

#[test]
fn from_error() {
    fn foo() -> Result<(), Exn<Error>> {
        Err(Error("An error"))?;
        Ok(())
    }

    let result = foo();
    insta::assert_snapshot!(debug_string(result.unwrap_err()),@"An error, at gix-error/tests/error/exn.rs:119:9");
}

#[test]
fn bail() {
    fn foo() -> Result<(), Exn<Error>> {
        gix_error::bail!(Error("An error"));
    }

    let result = foo();
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:130:9");
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
    insta::assert_snapshot!(debug_string(result.unwrap_err()), @"An error, at gix-error/tests/error/exn.rs:150:9");
}

#[test]
fn result_ok() -> Result<(), Exn<Error>> {
    Ok(())
}

fn debug_string(input: impl std::fmt::Debug) -> String {
    if cfg!(windows) {
        let out = format!("{input:?}");
        out.replace('\\', "/")
    } else {
        format!("{input:?}")
    }
}
