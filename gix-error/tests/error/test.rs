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
    let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(std::io::Error::other("boxed"));
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
