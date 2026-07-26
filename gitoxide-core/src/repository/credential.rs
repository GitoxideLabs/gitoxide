pub fn function(repo: Option<gix::Repository>, action: gix::credentials::program::main::Action) -> anyhow::Result<()> {
    use gix::credentials::program::main::Action::*;
    gix::credentials::program::main(
        Some(action.as_str().into()),
        std::io::stdin(),
        std::io::stdout(),
        gix::credentials::protocol::ContextOptions::default(),
        |action, context| -> Result<_, gix::Error> {
            let url = context
                .url
                .clone()
                .or_else(|| context.to_url())
                .ok_or(gix::Error::from_error(gix::credentials::protocol::Error::UrlMissing))?;

            let (mut cascade, _action, prompt_options) = match repo {
                Some(ref repo) => repo
                    .config_snapshot()
                    .credential_helpers(gix::url::parse(&url).map_err(gix::Error::from_error)?)
                    .map_err(gix::Error::from_error)?,
                None => {
                    let config = gix::config::File::from_globals().map_err(gix::Error::from_error)?;
                    let environment = gix::open::permissions::Environment::all();
                    gix::config::credential_helpers(
                        gix::url::parse(&url).map_err(gix::Error::from_error)?,
                        &config,
                        false,    /* lenient config */
                        |_| true, /* section filter */
                        environment,
                        false, /* use http path (override, uses configuration now)*/
                    )
                    .map_err(gix::Error::from_error)?
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
                .map_err(gix::Error::from_error)
        },
    )
    .map_err(Into::into)
}
