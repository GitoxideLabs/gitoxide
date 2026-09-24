use crate::ErrorWithSource;
use gix_error::{ErrorExt, ResultExt, TestError, message};

#[test]
fn debug_output_and_propagation_into_porcelain_errors() {
    fn test_failure(error: impl Into<TestError>) -> Result<(), TestError> {
        Err(error.into())
    }

    fn porcelain() -> Result<(), gix_error::Error> {
        test_failure(std::io::Error::other("porcelain"))?;
        Ok(())
    }

    let string = test_failure("message").unwrap_err();
    let plumbing = test_failure(message("plumbing").raise()).unwrap_err();
    let porcelain_input = test_failure(gix_error::Error::from(message("porcelain input").raise())).unwrap_err();
    let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(message("boxed"));
    let boxed = test_failure(boxed).unwrap_err();
    let porcelain = porcelain().unwrap_err();
    let output = format!(
        "message: {string:?}\nplumbing: {plumbing:?}\nporcelain input: {porcelain_input:?}\nboxed: {boxed:?}\nporcelain: {porcelain:?}"
    );

    insta::assert_snapshot!(output, "test failure Debug output", @r#"
    message: message, at gix-error/tests/error/test.rs:7
    plumbing: plumbing, at gix-error/tests/error/test.rs:16
    porcelain input: porcelain input, at gix-error/tests/error/test.rs:17
    boxed: boxed, at gix-error/tests/error/test.rs:7
    porcelain: Custom { kind: Other, error: "porcelain" }
    "#);
}

#[test]
fn debug_output_includes_the_complete_error_chain_and_call_sites() {
    fn failure() -> Result<(), TestError> {
        let result = Err::<(), _>(ErrorWithSource("leaf", message("native source")));
        result
            .or_raise(|| message("inner context"))
            .or_raise(|| message("outer context"))?;
        Ok(())
    }

    let output = format!("{:?}", failure().unwrap_err());
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_snapshot!(output, "test errors show the complete error tree and caller locations", @"
    outer context, at gix-error/tests/error/test.rs:40
    |
    └─ inner context, at gix-error/tests/error/test.rs:39
    |
    └─ leaf, at gix-error/tests/error/test.rs:39
    |
    └─ native source, at gix-error/tests/error/test.rs:39
    ");
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_snapshot!(output, "test errors show the complete flattened chain and caller locations", @"
    outer context, at gix-error/tests/error/test.rs:40

    Caused by:
        0: inner context, at gix-error/tests/error/test.rs:39
        1: leaf, at gix-error/tests/error/test.rs:39
        2: native source
    ");
}

#[test]
fn io_payload_reports_expand_each_payload_once_in_both_backends() {
    use std::io::{Error, ErrorKind};

    use super::exn::assert_io_payload_report;

    for nested in [false, true] {
        let payload = gix_error::validation("invalid input").with("input", b"ref\xff".as_slice());
        let io = if nested {
            let boundary = payload.raise().chain(message("payload child")).into_error();
            Error::new(ErrorKind::InvalidData, boundary.raise().into_error())
        } else {
            Error::new(ErrorKind::InvalidData, payload)
        };
        let mut exn = io.raise();
        if nested {
            exn = exn.chain(message("explicit sibling"));
        }
        let original_nodes = exn.iter_errors().count();
        let metadata = exn.metadata().cloned().collect::<Vec<_>>();
        let error = TestError::from(exn);
        let (expected, locations) = match (
            nested,
            cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))),
        ) {
            (false, false) => (
                r#"I/O error (InvalidData)
|
└─ invalid input, "input"="ref\xff""#,
                2,
            ),
            (false, true) => (
                r#"I/O error (InvalidData)

Caused by:
    0: invalid input, "input"="ref\xff""#,
                1,
            ),
            (true, false) => (
                r#"I/O error (InvalidData)
|
└─ invalid input, "input"="ref\xff"
|   |
|   └─ payload child
|
└─ explicit sibling"#,
                4,
            ),
            (true, true) => (
                r#"I/O error (InvalidData)

Caused by:
    0: explicit sibling
    1: invalid input, "input"="ref\xff"
    2: payload child"#,
                4,
            ),
        };
        for (report, locations) in [(format!("{error:?}"), locations), (format!("{error:#?}"), 0)] {
            assert_io_payload_report(
                &report,
                expected,
                locations,
                &[
                    ("I/O error (InvalidData)", 1),
                    ("invalid input", 1),
                    (r"ref\xff", 1),
                    ("payload child", usize::from(nested)),
                    ("explicit sibling", usize::from(nested)),
                ],
            );
        }
        let error = gix_error::Error::from(error);
        assert_eq!(
            error.iter_errors().count(),
            original_nodes,
            "omitting boundary labels does not remove boundary entries from traversal"
        );
        assert_eq!(
            error
                .downcast_any_ref::<Error>()
                .expect("TestError retains the I/O wrapper")
                .kind(),
            ErrorKind::InvalidData,
            "TestError reporting preserves the I/O kind"
        );
        assert!(error.is_validation(), "TestError retains the payload classification");
        assert_eq!(
            error.metadata().cloned().collect::<Vec<_>>(),
            metadata,
            "reporting and conversion preserve payload bytes without duplicating metadata"
        );
    }
}
