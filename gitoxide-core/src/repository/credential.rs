pub fn function(repo: Option<gix::Repository>, action: gix::credentials::program::main::Action) -> anyhow::Result<()> {
    use gix::credentials::program::main::Action::*;
    use gix::error::{OptionExt, ResultExt, ValidationError, message};
    gix::credentials::program::main(
        Some(action.as_str().into()),
        std::io::stdin(),
        std::io::stdout(),
        gix::credentials::protocol::ContextOptions::default(),
        |action, context| -> Result<_, gix::Exn> {
            let url = context
                .url
                .clone()
                .or_else(|| context.to_url())
                .ok_or_raise_erased(|| {
                    ValidationError::new("Either 'url' field or both 'protocol' and 'host' fields must be provided")
                })?;

            let url = gix::url::parse(&url).or_erased()?;
            let (mut cascade, _action, prompt_options) = match repo {
                Some(ref repo) => repo
                    .config_snapshot()
                    .credential_helpers(url)
                    .or_raise_erased(|| message("Could not configure credential helpers"))?,
                None => {
                    let config = gix::config::File::from_globals()
                        .or_raise_erased(|| message("Could not load global configuration"))?;
                    let environment = gix::open::permissions::Environment::all();
                    gix::config::credential_helpers(
                        url,
                        &config,
                        false,    /* lenient config */
                        |_| true, /* section filter */
                        environment,
                        false, /* use http path (override, uses configuration now)*/
                    )
                    .or_raise_erased(|| message("Could not configure credential helpers"))?
                }
            };
            cascade
                .invoke(
                    match action {
                        Get => gix::credentials::helper::Action::Get(context),
                        Erase => gix::credentials::helper::Action::Erase(context.to_bstring()),
                        Store => gix::credentials::helper::Action::Store(context.to_bstring()),
                    },
                    prompt_options,
                )
                .map(|outcome| outcome.and_then(|outcome| (&outcome.next).try_into().ok()))
        },
    )
    .map_err(gix::Exn::into_error)?;
    Ok(())
}
