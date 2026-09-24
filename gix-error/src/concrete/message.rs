use crate::Message;

/// Return a new statically allocated `message`.
///
/// Use the [message!-macro](crate::message!) for convenient `format!`-like behavior.
pub fn message(message: &'static str) -> Message {
    Message::new(message)
}
