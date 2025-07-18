pub use crate::client::non_io_types::connect::{Error, Options};

pub(crate) mod function {
    use crate::client::non_io_types::connect::Error;

    /// A general purpose connector connecting to a repository identified by the given `url`.
    ///
    /// This includes connections to
    /// [git daemons][crate::client::git::connect()] and `ssh`,
    ///
    /// Use `options` to further control specifics of the transport resulting from the connection.
    pub async fn connect<Url, E>(
        url: Url,
        options: super::Options,
    ) -> Result<Box<dyn crate::client::Transport + Send>, Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        gix_url::parse::Error: From<E>,
    {
        let url = url.try_into().map_err(gix_url::parse::Error::from)?;
        Ok(match url.scheme {
            #[cfg(feature = "async-std")]
            gix_url::Scheme::Git => {
                if url.user().is_some() {
                    return Err(Error::UnsupportedUrlTokens {
                        url: url.to_bstring(),
                        scheme: url.scheme,
                    });
                }

                Box::new(
                    crate::client::git::Connection::new_tcp(
                        url.host().expect("host is present in url"),
                        url.port,
                        url.path.clone(),
                        options.version,
                        options.trace,
                    )
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?,
                )
            }
            #[cfg(feature = "russh")]
            gix_url::Scheme::Ssh => {
                Box::new(crate::client::async_io::ssh::connect(url, options.version, options.trace).await?)
            }
            scheme => return Err(Error::UnsupportedScheme(scheme)),
        })
    }
}
