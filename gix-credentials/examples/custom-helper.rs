use gix_credentials::{program, protocol};
use gix_error::ErrorExt;

/// Run like this `echo url=https://example.com | cargo run --example custom-helper -- get`
pub fn main() -> Result<(), gix_credentials::program::main::Error> {
    gix_credentials::program::main(
        std::env::args_os().skip(1),
        std::io::stdin(),
        std::io::stdout(),
        protocol::ContextOptions::default(),
        |action, context| -> Result<_, gix_error::Exn> {
            match action {
                program::main::Action::Get => Ok(Some(protocol::Context {
                    username: Some("user".into()),
                    password: Some("pass".into()),
                    ..context
                })),
                program::main::Action::Erase => {
                    Err(gix_error::message("Refusing to delete credentials for demo purposes").raise_erased())
                }
                program::main::Action::Store => Ok(None),
            }
        },
    )
}
