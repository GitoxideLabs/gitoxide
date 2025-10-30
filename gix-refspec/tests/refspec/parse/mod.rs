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
                    match (res.as_ref().err(), err_code == 0) {
                        (
                            Some(
                                gix_refspec::parse::Error::NegativePartialName
                                | gix_refspec::parse::Error::NegativeGlobPattern,
                            ),
                            true,
                        ) => {} // we prefer failing fast, git let's it pass
                        // We now allow complex glob patterns in one-sided refspecs
                        (None, false) if is_one_sided_glob_pattern(spec, op) => {
                            // This is an intentional behavior change: we allow complex globs in one-sided refspecs
                        }
                        _ => {
                            eprintln!("{err_code} {res:?} {} {:?}", kind.as_bstr(), spec.as_bstr());
                            mismatch += 1;
                        }
                    }
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
    
    fn is_one_sided_glob_pattern(spec: &[u8], op: Operation) -> bool {
        use bstr::ByteSlice;
        matches!(op, Operation::Fetch) && spec.to_str().map(|s| s.contains('*') && !s.contains(':')).unwrap_or(false)
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
    use gix_refspec::{parse::Operation, Instruction, RefSpecRef};

    pub fn b(input: &str) -> &bstr::BStr {
        input.into()
    }

    pub fn try_parse(spec: &str, op: Operation) -> Result<RefSpecRef<'_>, gix_refspec::parse::Error> {
        gix_refspec::parse(spec.into(), op)
    }

    pub fn assert_parse<'a>(spec: &'a str, expected: Instruction<'_>) -> RefSpecRef<'a> {
        let spec = try_parse(spec, expected.operation()).expect("no error");
        assert_eq!(spec.instruction(), expected);
        spec
    }
}
pub use util::*;
