use std::panic::catch_unwind;

use bstr::ByteSlice;
use gix_refspec::parse::Operation;
use gix_testtools::scripted_fixture_read_only;

#[test]
fn baseline() {
    let dir = scripted_fixture_read_only("parse_baseline.sh").unwrap();
    let baseline = std::fs::read(dir.join("baseline.git")).unwrap();
    let mut lines = baseline.lines();
    let mut panics = 0;
    let mut mismatch = 0;
    let mut count = 0;
    while let Some(kind_spec) = lines.next() {
        count += 1;
        let (kind, spec) = kind_spec.split_at(kind_spec.find_byte(b' ').expect("space between kind and spec"));
        let spec = &spec[1..];
        let err_code: usize = lines
            .next()
            .expect("err code")
            .to_str()
            .unwrap()
            .parse()
            .expect("number");
        let op = match kind {
            b"fetch" => Operation::Fetch,
            b"push" => Operation::Push,
            _ => unreachable!("{} unexpected", kind.as_bstr()),
        };
        let res = catch_unwind(|| try_parse(spec.to_str().unwrap(), op));
        match &res {
            Ok(res) => match (res.is_ok(), err_code == 0) {
                (true, true) | (false, false) => {
                    if let Ok(spec) = res {
                        spec.instruction(); // should not panic
                    }
                }
                _ => {
                    eprintln!("{err_code} {res:?} {} {:?}", kind.as_bstr(), spec.as_bstr());
                    mismatch += 1;
                }
            },
            Err(_) => {
                panics += 1;
            }
        }
    }
    if panics != 0 || mismatch != 0 {
        panic!(
            "Out of {} baseline entries, got {} right, ({} mismatches and {} panics)",
            count,
            count - (mismatch + panics),
            mismatch,
            panics
        );
    }
}

#[test]
fn local_and_remote() -> crate::Result {
    let spec = gix_refspec::parse("remote:local".into(), Operation::Fetch)?;
    assert_eq!(spec.remote(), spec.source());
    assert_eq!(spec.local(), spec.destination());

    let spec = gix_refspec::parse("local:remote".into(), Operation::Push)?;
    assert_eq!(spec.local(), spec.source());
    assert_eq!(spec.remote(), spec.destination());
    Ok(())
}

mod fetch;
mod invalid;
mod push;

mod util {
    use gix_refspec::{Instruction, RefSpecRef, parse::Operation};

    pub fn b(input: &str) -> &bstr::BStr {
        input.into()
    }

    pub fn try_parse(spec: &str, op: Operation) -> Result<RefSpecRef<'_>, gix_refspec::parse::Error> {
        gix_refspec::parse(spec.into(), op)
    }

    pub fn assert_validation(spec: &str, op: Operation, message: &str) -> gix_refspec::parse::Error {
        let err = try_parse(spec, op).expect_err("refspec is invalid");
        assert_eq!(err.message, message);
        err
    }

    pub fn assert_reference_error(spec: &str, op: Operation) -> gix_refspec::parse::Error {
        let err = try_parse(spec, op).expect_err("refspec contains an invalid reference name");
        let source = err
            .downcast_any_ref::<gix_validate::reference::name::Error>()
            .expect("the original reference-name validation error is retained");
        assert_eq!(err.message, source.to_string());
        err
    }

    pub fn assert_unsupported_pattern(spec: &str, op: Operation) {
        let err = assert_validation(spec, op, "refspec patterns may only contain a single '*' character");
        let input = err.input.as_ref().expect("the unsupported pattern is retained");
        assert!(
            spec.as_bytes()
                .windows(input.len())
                .any(|candidate| candidate == input.as_slice())
        );
    }

    pub fn assert_parse<'a>(spec: &'a str, expected: Instruction<'_>) -> RefSpecRef<'a> {
        let spec = try_parse(spec, expected.operation()).expect("no error");
        assert_eq!(spec.instruction(), expected);
        spec
    }
}
pub use util::*;
