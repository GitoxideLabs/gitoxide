/// Run like this `echo url=https://example.com | cargo run --example git-credential-light -- fill`
pub fn main() -> Result<(), gix_error::Exn> {
    gix_credentials::program::main(
        std::env::args_os().skip(1),
        std::io::stdin(),
        std::io::stdout(),
        gix_credentials::protocol::ContextOptions::default(),
        |action, context| {
            use gix_credentials::program::main::Action::*;
            gix_credentials::helper::Cascade::default()
                .invoke(
                    match action {
                        Get => gix_credentials::helper::Action::Get(context),
                        Erase => gix_credentials::helper::Action::Erase(context.to_bstring()),
                        Store => gix_credentials::helper::Action::Store(context.to_bstring()),
                    },
                    gix_prompt::Options::default().apply_environment(true, true, true),
                )
                .map(|outcome| outcome.and_then(|outcome| (&outcome.next).try_into().ok()))
        },
    )
}
