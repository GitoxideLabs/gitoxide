pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result = std::result::Result<(), Error>;

mod access;
mod baseline;
mod expand_path;
mod fuzzed;
mod parse;
