pub use crate::client::non_io_types::connect::{Error, Options};

#[cfg(feature = "async-std")]
pub(crate) mod function {
    use crate::client::{async_io::Transport, git::async_io::Connection, non_io_types::connect::Error};
    use gix_error::{ErrorExt, ResultExt, message};

    /// A general purpose connector connecting to a repository identified by the given `url`.
    ///
    /// This includes connections to
    /// [git daemons][crate::client::git::connect()] only at the moment.
    ///
    /// Use `options` to further control specifics of the transport resulting from the connection.
    pub async fn connect<Url, E>(url: Url, options: super::Options) -> Result<Box<dyn Transport + Send>, Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let mut url = url.try_into().or_raise(|| message("Could not parse URL"))?;
        Ok(match url.scheme {
            gix_url::Scheme::Git => {
                if url.user().is_some() {
                    return Err(message!(
                        "The url {:?} contains information that would not be used by the {} protocol",
                        url.to_bstring(),
                        url.scheme
                    )
                    .raise());
                }
                let path = std::mem::take(&mut url.path);
                Box::new(
                    Connection::new_tcp(
                        url.host().expect("host is present in url"),
                        url.port,
                        path,
                        options.version,
                        options.trace,
                    )
                    .await
                    .or_raise(|| message("connection failed"))?,
                )
            }
            scheme => return Err(message!("The '{scheme}' protocol is currently unsupported").raise()),
        })
    }
}

#[cfg(feature = "async-std")]
pub use function::connect;
