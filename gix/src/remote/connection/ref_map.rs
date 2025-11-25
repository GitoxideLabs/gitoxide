use gix_features::progress::Progress;
#[cfg(feature = "async-network-client")]
use gix_transport::client::async_io::Transport;
#[cfg(feature = "blocking-network-client")]
use gix_transport::client::blocking_io::Transport;

use crate::{
    bstr::BString,
    remote::{fetch, Connection, Direction},
};

/// The error returned by [`Connection::ref_map()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    InitRefMap(#[from] gix_protocol::fetch::refmap::init::Error),
    #[error("Failed to configure the transport before connecting to {url:?}")]
    GatherTransportConfig {
        url: BString,
        source: crate::config::transport::Error,
    },
    #[error("Failed to configure the transport layer")]
    ConfigureTransport(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error(transparent)]
    Handshake(#[from] gix_protocol::handshake::Error),
    #[error(transparent)]
    Transport(#[from] gix_protocol::transport::client::Error),
    #[error(transparent)]
    ConfigureCredentials(#[from] crate::config::credential_helpers::Error),
}

impl gix_protocol::transport::IsSpuriousError for Error {
    fn is_spurious(&self) -> bool {
        match self {
            Error::Transport(err) => err.is_spurious(),
            Error::Handshake(err) => err.is_spurious(),
            _ => false,
        }
    }
}

/// For use in [`Connection::ref_map()`].
#[derive(Debug, Clone)]
pub struct Options {
    /// Use a two-component prefix derived from the ref-spec's source, like `refs/heads/`  to let the server pre-filter refs
    /// with great potential for savings in traffic and local CPU time. Defaults to `true`.
    pub prefix_from_spec_as_filter_on_remote: bool,
    /// Parameters in the form of `(name, optional value)` to add to the handshake.
    ///
    /// This is useful in case of custom servers.
    pub handshake_parameters: Vec<(String, Option<String>)>,
    /// A list of refspecs to use as implicit refspecs which won't be saved or otherwise be part of the remote in question.
    ///
    /// This is useful for handling `remote.<name>.tagOpt` for example.
    pub extra_refspecs: Vec<gix_refspec::RefSpec>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            prefix_from_spec_as_filter_on_remote: true,
            handshake_parameters: Vec::new(),
            extra_refspecs: Vec::new(),
        }
    }
}

impl<T> Connection<'_, '_, T>
where
    T: Transport,
{
    /// List all references on the remote that have been filtered through our remote's [`refspecs`][crate::Remote::refspecs()]
    /// for _fetching_.
    ///
    /// This comes in the form of all matching tips on the remote and the object they point to, along with
    /// the local tracking branch of these tips (if available).
    ///
    /// Note that this doesn't fetch the objects mentioned in the tips nor does it make any change to underlying repository.
    ///
    /// # Consumption
    ///
    /// Due to management of the transport, it's cleanest to only use it for a single interaction. Thus, it's consumed
    /// along with the connection.
    ///
    /// ### Configuration
    ///
    /// - `gitoxide.userAgent` is read to obtain the application user agent for git servers and for HTTP servers as well.
    #[allow(clippy::result_large_err)]
    #[gix_protocol::maybe_async::maybe_async]
    pub async fn ref_map(
        mut self,
        progress: impl Progress,
        options: Options,
    ) -> Result<(fetch::RefMap, gix_protocol::Handshake), Error> {
        let refmap = self.ref_map_by_ref(progress, options).await?;
        let handshake = self
            .handshake
            .expect("refmap always performs handshake and stores it if it succeeds");
        Ok((refmap, handshake))
    }

    #[allow(clippy::result_large_err)]
    #[gix_protocol::maybe_async::maybe_async]
    pub(crate) async fn ref_map_by_ref(
        &mut self,
        mut progress: impl Progress,
        Options {
            prefix_from_spec_as_filter_on_remote,
            handshake_parameters,
            mut extra_refspecs,
        }: Options,
    ) -> Result<fetch::RefMap, Error> {
        let _span = gix_trace::coarse!("remote::Connection::ref_map()");
        if let Some(tag_spec) = self.remote.fetch_tags.to_refspec().map(|spec| spec.to_owned()) {
            if !extra_refspecs.contains(&tag_spec) {
                extra_refspecs.push(tag_spec);
            }
        }
        let mut credentials_storage;
        let url = self.transport.inner.to_url();
        let authenticate = match self.authenticate.as_mut() {
            Some(f) => f,
            None => {
                let url = self.remote.url(Direction::Fetch).map_or_else(
                    || gix_url::parse(url.as_ref()).expect("valid URL to be provided by transport"),
                    ToOwned::to_owned,
                );
                credentials_storage = self.configured_credentials(url)?;
                &mut credentials_storage
            }
        };

        let repo = self.remote.repo;
        if self.transport_options.is_none() {
            self.transport_options = repo
                .transport_options(url.as_ref(), self.remote.name().map(crate::remote::Name::as_bstr))
                .map_err(|err| Error::GatherTransportConfig {
                    source: err,
                    url: url.into_owned(),
                })?;
        }
        if let Some(config) = self.transport_options.as_ref() {
            self.transport.inner.configure(&**config)?;
        }
        let mut handshake = gix_protocol::handshake(
            &mut self.transport.inner,
            gix_transport::Service::UploadPack,
            authenticate,
            handshake_parameters,
            &mut progress,
        )
        .await?;

        let context = fetch::refmap::init::Context {
            fetch_refspecs: self.remote.fetch_specs.clone(),
            extra_refspecs,
        };

        let fetch_refmap = handshake.fetch_or_extract_refmap(
            self.remote.repo.config.user_agent_tuple(),
            prefix_from_spec_as_filter_on_remote,
            context,
        )?;

        #[cfg(feature = "async-network-client")]
        let ref_map = fetch_refmap
            .fetch_async(progress, &mut self.transport.inner, self.trace)
            .await?;

        #[cfg(feature = "blocking-network-client")]
        let ref_map = fetch_refmap.fetch_blocking(progress, &mut self.transport.inner, self.trace)?;

        self.handshake = Some(handshake);
        Ok(ref_map)
    }
}
