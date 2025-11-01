pub use crate::client::non_io_types::connect::{Error, Options};

pub(crate) mod function {
    #[cfg(feature = "http-client-curl")]
    use crate::client::blocking_io::http::curl::Curl;
    #[cfg(all(feature = "http-client-reqwest", not(feature = "http-client-curl")))]
    use crate::client::blocking_io::http::reqwest::Remote as Reqwest;
    use crate::client::{blocking_io::Transport, non_io_types::connect::Error};

    /// A general purpose connector connecting to a repository identified by the given `url`.
    ///
    /// This includes connections to
    /// [local repositories](crate::client::blocking_io::file::connect()),
    /// [repositories over ssh](crate::client::blocking_io::ssh::connect()),
    /// [git daemons](crate::client::blocking_io::connect::connect()),
    /// and if compiled in connections to [git repositories over https](crate::client::blocking_io::http::connect()).
    ///
    /// Use `options` to further control specifics of the transport resulting from the connection.
    pub fn connect<Url, E>(url: Url, options: super::Options) -> Result<Box<dyn Transport + Send>, Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        gix_url::parse::Error: From<E>,
    {
        let mut url = url.try_into().map_err(gix_url::parse::Error::from)?;
        Ok(match url.scheme {
            gix_url::Scheme::Ext(_) => return Err(Error::UnsupportedScheme(url.scheme)),
            gix_url::Scheme::File => {
                if url.user().is_some() || url.password().is_some() || url.host().is_some() || url.port.is_some() {
                    return Err(Error::UnsupportedUrlTokens {
                        url: url.to_bstring(),
                        scheme: url.scheme,
                    });
                }
                Box::new(
                    crate::client::blocking_io::file::connect(url.path, options.version, options.trace)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?,
                )
            }
            gix_url::Scheme::Ssh => Box::new({
                crate::client::blocking_io::ssh::connect(url, options.version, options.ssh, options.trace)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
            }),
            gix_url::Scheme::Git => {
                if url.user().is_some() {
                    return Err(Error::UnsupportedUrlTokens {
                        url: url.to_bstring(),
                        scheme: url.scheme,
                    });
                }
                Box::new({
                    let path = std::mem::take(&mut url.path);
                    crate::client::git::blocking_io::connect(
                        url.host().expect("host is present in url"),
                        path,
                        options.version,
                        url.port,
                        options.trace,
                    )
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
                })
            }
            #[cfg(not(any(feature = "http-client-curl", feature = "http-client-reqwest")))]
            gix_url::Scheme::Https | gix_url::Scheme::Http => return Err(Error::CompiledWithoutHttp(url.scheme)),
            #[cfg(feature = "http-client-curl")]
            gix_url::Scheme::Https | gix_url::Scheme::Http => Box::new(
                crate::client::blocking_io::http::connect::<Curl>(url, options.version, options.trace),
            ),
            #[cfg(all(feature = "http-client-reqwest", not(feature = "http-client-curl")))]
            gix_url::Scheme::Https | gix_url::Scheme::Http => Box::new(crate::client::blocking_io::http::connect::<
                Reqwest,
            >(
                url, options.version, options.trace
            )),
        })
    }
}

pub use function::connect;
