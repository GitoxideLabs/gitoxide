use crate::Message;
use bstr::BString;
use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};

/// An error occurred when validating input.
///
/// This can happen explicitly, or as part of parsing, for example.
#[derive(Debug)]
pub struct ValidationError {
    /// The error message.
    pub message: Cow<'static, str>,
    /// The input or portion of the input that wasn't valid.
    pub input: Option<BString>,
}

/// Lifecycle
impl ValidationError {
    /// Create a new error with `message` and `input`. Note that `input` isn't printed.
    pub fn new_with_input(message: impl Into<Cow<'static, str>>, input: impl Into<BString>) -> Self {
        ValidationError {
            message: message.into(),
            input: Some(input.into()),
        }
    }

    /// Create a new instance that displays the given `message`.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        ValidationError {
            message: message.into(),
            input: None,
        }
    }
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.input {
            None => f.write_str(self.message.as_ref()),
            Some(input) => {
                write!(f, "{}: {input}", self.message)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

impl From<Message> for ValidationError {
    fn from(Message(msg): Message) -> Self {
        ValidationError::new(msg)
    }
}

impl From<String> for ValidationError {
    fn from(msg: String) -> Self {
        ValidationError::new(msg)
    }
}

impl From<&'static str> for ValidationError {
    fn from(msg: &'static str) -> Self {
        ValidationError::new(msg)
    }
}
