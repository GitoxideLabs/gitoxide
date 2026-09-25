use gix_credentials::{program, protocol};
use gix_error::ErrorExt;

/// Run like this `echo url=https://example.com | cargo run --example custom-helper -- get`
pub fn main() -> gix_error::Result {
    gix_credentials::program::main(
        std::env::args_os().skip(1),
        std::io::stdin(),
        std::io::stdout(),
        protocol::ContextOptions::default(),
        |action, context| -> gix_error::Result<_> {
            match action {
                program::main::Action::Get => Ok(Some(protocol::Context {
                    username: Some("user".into()),
                    password: Some("pass".into()),
                    ..context
                })),
                program::main::Action::Erase => {
                    Err(gix_error::message("Refusing to delete credentials for demo purposes")
                        .raise()
                        .into())
                }
                program::main::Action::Store => Ok(None),
            }
        },
    )
}
