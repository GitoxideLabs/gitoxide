use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};

/// An error that is further described in a message.
#[derive(Debug)]
pub struct Message(
    /// The error message.
    pub(crate) Cow<'static, str>,
);

/// Lifecycle
impl Message {
    /// Create a new instance that displays the given `message`.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Message(message.into())
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_ref())
    }
}

impl std::error::Error for Message {}

impl From<String> for Message {
    fn from(msg: String) -> Self {
        Message::new(msg)
    }
}

impl From<&'static str> for Message {
    fn from(msg: &'static str) -> Self {
        Message::new(msg)
    }
}

/// Return a new statically allocated `message`.
///
/// Use the [message!-macro](crate::message!) for convenient `format!`-like behavior.
pub fn message(message: &'static str) -> Message {
    Message::new(message)
}

/// Construct a [`Message`](crate::Message) from a string literal or format string.
/// Note that it always runs `format!()`, use the [`message()`](crate::message()) function for literals instead.
#[macro_export]
macro_rules! message {
    ($message_with_format_args:literal $(,)?) => {
        $crate::Message::new(format!($message_with_format_args))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Message::new(format!($fmt, $($arg)*))
    };
}
