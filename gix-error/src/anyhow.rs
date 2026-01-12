//! Conversions to `anyhow::Error` for interoperability.
//!
//! This module provides conversions from `Exn<T>` and `Error` types to `anyhow::Error`,
//! flattening the tree of error frames into a linear chain of sources.

use crate::{Error, Exn, Frame};
use std::fmt;

/// A wrapper error that provides a linear chain through the error tree.
///
/// This error is used to convert from `Exn` or `Error` to `anyhow::Error`,
/// presenting the first child as the source, which then presents its first child, and so on.
struct ChainedFrameError {
    message: String,
    debug: String,
    source: Option<Box<ChainedFrameError>>,
}

impl ChainedFrameError {
    /// Create a chain from a frame by walking the first-child path.
    fn from_frame(frame: &Frame) -> Self {
        let message = format!("{}", frame.as_error());
        let debug = format!("{:?}", frame.as_error());
        
        // Create the source chain by following the first child
        let source = frame.children().first().map(|child| {
            Box::new(ChainedFrameError::from_frame(child))
        });
        
        ChainedFrameError {
            message,
            debug,
            source,
        }
    }
}

impl fmt::Display for ChainedFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl fmt::Debug for ChainedFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.debug)
    }
}

impl std::error::Error for ChainedFrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|s| s.as_ref() as &(dyn std::error::Error + 'static))
    }
}

/// Extension trait to convert to `anyhow::Error`.
pub trait IntoAnyhow {
    /// Convert into an `anyhow::Error`, flattening the error tree into a source chain.
    fn into_anyhow(self) -> anyhow::Error;
}

/// Convert `Exn<E>` to `anyhow::Error`, flattening the error tree into a source chain.
impl<E> IntoAnyhow for Exn<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_anyhow(self) -> anyhow::Error {
        let chained = ChainedFrameError::from_frame(self.as_frame());
        anyhow::Error::new(chained)
    }
}

/// Convert `Error` to `anyhow::Error`, flattening the error tree into a source chain.
impl IntoAnyhow for Error {
    fn into_anyhow(self) -> anyhow::Error {
        let frame = self.into_frame();
        let chained = ChainedFrameError::from_frame(&frame);
        anyhow::Error::new(chained)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorExt, ResultExt};

    #[derive(Debug)]
    struct TestError1(&'static str);
    impl fmt::Display for TestError1 {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "TestError1: {}", self.0)
        }
    }
    impl std::error::Error for TestError1 {}

    #[derive(Debug)]
    struct TestError2(&'static str);
    impl fmt::Display for TestError2 {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "TestError2: {}", self.0)
        }
    }
    impl std::error::Error for TestError2 {}

    #[derive(Debug)]
    struct TestError3(&'static str);
    impl fmt::Display for TestError3 {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "TestError3: {}", self.0)
        }
    }
    impl std::error::Error for TestError3 {}

    #[test]
    fn test_exn_to_anyhow_simple() {
        let err: Exn<TestError1> = TestError1("base error").raise();
        let anyhow_err = err.into_anyhow();
        
        let error_chain = format!("{:?}", anyhow_err);
        assert!(error_chain.contains("TestError1: base error"));
    }

    #[test]
    fn test_exn_to_anyhow_chain() {
        let result: Result<(), TestError3> = Err(TestError3("lowest error"));
        let result = result.or_raise(|| TestError2("middle error"));
        let result = result.or_raise(|| TestError1("top error"));
        
        let err = result.unwrap_err();
        let anyhow_err = err.into_anyhow();
        
        // Verify the error chain is preserved
        let error_chain = format!("{:?}", anyhow_err);
        assert!(error_chain.contains("TestError1: top error"));
        
        // Check that sources are accessible - should have all three levels
        use std::error::Error;
        let mut source = anyhow_err.source();
        assert!(source.is_some(), "Should have first source");
        
        // Verify middle error is in the chain
        if let Some(err) = source {
            let s = format!("{}", err);
            assert!(s.contains("TestError2: middle error"), "Second level should be middle error, got: {}", s);
            source = err.source();
        }
        
        // Verify lowest error is in the chain
        assert!(source.is_some(), "Should have third source");
        if let Some(err) = source {
            let s = format!("{}", err);
            assert!(s.contains("TestError3: lowest error"), "Third level should be lowest error, got: {}", s);
        }
    }

    #[test]
    fn test_error_to_anyhow() {
        let exn: Exn<TestError1> = TestError1("base error").raise();
        let error: Error = exn.into();
        let anyhow_err = error.into_anyhow();
        
        let error_chain = format!("{:?}", anyhow_err);
        assert!(error_chain.contains("TestError1: base error"));
    }

    #[test]
    fn test_anyhow_error_chain_prints() {
        // Create a chain of errors
        let result: Result<(), TestError3> = Err(TestError3("IO failed"));
        let result = result.or_raise(|| TestError2("failed to read file"));
        let result = result.or_raise(|| TestError1("operation failed"));
        
        let err = result.unwrap_err();
        let anyhow_err = err.into_anyhow();
        
        // Print the error to verify it displays as a chain
        // This would normally print:
        // Error: TestError1: operation failed
        //
        // Caused by:
        //    0: TestError2: failed to read file
        //    1: TestError3: IO failed
        let display = format!("{}", anyhow_err);
        assert!(display.contains("TestError1: operation failed"));
        
        // Also test debug format
        let debug = format!("{:?}", anyhow_err);
        assert!(debug.contains("TestError1: operation failed"));
    }
}
