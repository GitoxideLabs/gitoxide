use crate::Message;
use bstr::BString;
use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};

/// An error occurred when parsing input
#[derive(Debug)]
pub struct ParseError {
    /// The error message.
    pub message: Cow<'static, str>,
    /// The input or portion of the input that failed to parse.
    pub input: Option<BString>,
}

/// Lifecycle
impl ParseError {
    /// Create a new error with `message` and `input`. Note that `input` isn't printed.
    pub fn new_with_input(message: impl Into<Cow<'static, str>>, input: impl Into<BString>) -> Self {
        ParseError {
            message: message.into(),
            input: Some(input.into()),
        }
    }

    /// Create a new instance that displays the given `message`.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        ParseError {
            message: message.into(),
            input: None,
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.input {
            None => f.write_str(self.message.as_ref()),
            Some(input) => {
                write!(f, "{}: {input}", self.message)
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl From<Message> for ParseError {
    fn from(Message(msg): Message) -> Self {
        ParseError::new(msg)
    }
}

impl From<String> for ParseError {
    fn from(msg: String) -> Self {
        ParseError::new(msg)
    }
}

impl From<&'static str> for ParseError {
    fn from(msg: &'static str) -> Self {
        ParseError::new(msg)
    }
}
