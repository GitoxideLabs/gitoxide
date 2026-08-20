#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    UrlParse(#[from] gix::Error),
    #[error(transparent)]
    Configuration(#[from] gix::config::credential_helpers::Error),
    #[error(transparent)]
    Protocol(#[from] gix::credentials::protocol::Error),
    #[error(transparent)]
    ConfigLoad(#[from] gix::config::file::init::from_paths::Error),
}

pub fn function(repo: Option<gix::Repository>, action: gix::credentials::program::main::Action) -> anyhow::Result<()> {
    use gix::credentials::program::main::Action::*;
    gix::credentials::program::main(
        Some(action.as_str().into()),
        std::io::stdin(),
        std::io::stdout(),
        gix::credentials::protocol::ContextOptions::default(),
        |action, context| -> Result<_, Error> {
            let url = context
                .url
                .clone()
                .or_else(|| context.to_url())
                .ok_or(Error::Protocol(gix::credentials::protocol::Error::UrlMissing))?;

            let url = gix::url::parse(&url).map_err(gix::Exn::into_error)?;
            let (mut cascade, _action, prompt_options) = match repo {
                Some(ref repo) => repo.config_snapshot().credential_helpers(url)?,
                None => {
                    let config = gix::config::File::from_globals()?;
                    let environment = gix::open::permissions::Environment::all();
                    gix::config::credential_helpers(
                        url,
                        &config,
                        false,    /* lenient config */
                        |_| true, /* section filter */
                        environment,
                        false, /* use http path (override, uses configuration now)*/
                    )?
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
                .map_err(Into::into)
        },
    )
    .map_err(Into::into)
}
