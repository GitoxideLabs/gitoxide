use gix_error::Result;
/// An implementation for HTTP requests via [`gix-reqwest`][crate::gix_reqwest].
pub struct Remote {
    /// A worker thread which performs the actual request.
    handle: Option<std::thread::JoinHandle<Result>>,
    /// A channel to send requests (work) to the worker thread.
    request: std::sync::mpsc::SyncSender<remote::Request>,
    /// A channel to receive the result of the prior request.
    response: std::sync::mpsc::Receiver<remote::Response>,
    /// A mechanism for configuring the remote.
    config: crate::client::blocking_io::http::Options,
    /// The effective base URL after an accepted redirect.
    redirected_base_url: std::sync::Arc<parking_lot::Mutex<Option<String>>>,
}

/// A function to configure a single request prior to sending it, support most complex configuration beyond what's possible with
/// basic `git` http configuration.
/// The request belongs to [`crate::gix_reqwest`], Gitoxide's patched reqwest fork.
pub type ConfigureRequestFn = dyn FnMut(&mut gix_reqwest::blocking::Request) -> Result + Send + Sync + 'static;

/// A function to configure the HTTP client before it is built, for example to provide a TLS configuration.
/// Use [`crate::gix_reqwest`] types when configuring the builder.
pub type ConfigureClientFn = dyn FnMut(gix_reqwest::blocking::ClientBuilder) -> Result<gix_reqwest::blocking::ClientBuilder>
    + Send
    + Sync
    + 'static;

/// Options to configure the HTTP handler backed by [`crate::gix_reqwest`].
#[derive(Default)]
pub struct Options {
    /// Configure the client on the first request, before building and reusing it for subsequent requests.
    ///
    /// The callback runs again after client execution fails, after a worker restart, after a redirect chain changes
    /// proxies, or when proxy routing or initial credentials change.
    /// Changes to this callback after initialization do not affect an existing client. The transport installs
    /// its proxy and redirect policies after this callback; use [`Self::configure_request`] for request-specific headers.
    /// Reqwest handles redirect headers and timeouts, including the client's Referer setting and total request timeout.
    /// After an authority change, subsequent requests suppress sensitive client defaults and retain explicit request headers.
    pub configure_client: Option<Box<ConfigureClientFn>>,
    /// A function to configure the request that is about to be made.
    pub configure_request: Option<Box<ConfigureRequestFn>>,
}

///
pub mod remote;
