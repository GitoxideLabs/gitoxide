use crate::client::WriteMode;

/// The error used by the [Http] trait.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Could not initialize the http client")]
    InitHttpClient {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    #[error("{description}")]
    Detail { description: String },
    #[error("An IO error occurred while uploading the body of a POST request")]
    PostBody(#[from] std::io::Error),
}

impl crate::IsSpuriousError for Error {
    fn is_spurious(&self) -> bool {
        match self {
            Error::PostBody(err) => err.is_spurious(),
            #[cfg(any(feature = "http-client-reqwest", feature = "http-client-curl"))]
            Error::InitHttpClient { source } => {
                #[cfg(feature = "http-client-curl")]
                if let Some(err) = source.downcast_ref::<crate::client::http::curl::Error>() {
                    return err.is_spurious();
                }
                #[cfg(feature = "http-client-reqwest")]
                if let Some(err) = source.downcast_ref::<crate::client::http::reqwest::remote::Error>() {
                    return err.is_spurious();
                };
                false
            }
            _ => false,
        }
    }
}

/// The return value of [`Http::get()`].
pub struct GetResponse<H, B> {
    /// The response headers.
    pub headers: H,
    /// The response body.
    pub body: B,
}

/// The return value of [`Http::post()`].
pub struct PostResponse<H, B, PB> {
    /// The body to post to the server as part of the request.
    ///
    /// **Note**: Implementations should drop the handle to avoid deadlocks.
    pub post_body: PB,
    /// The headers of the post response.
    pub headers: H,
    /// The body of the post response.
    pub body: B,
}

/// Whether or not the post body is expected to fit into memory or not.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum PostBodyDataKind {
    /// We know how much data we are sending and think it will fit into memory. This allows to collect it into a buffer
    /// and send it with `Content-Length: <body-len>`.
    BoundedAndFitsIntoMemory,
    /// We don't know how much data we will send and assume it won't fit into memory. This enables streaming mode.
    Unbounded,
}

impl From<WriteMode> for PostBodyDataKind {
    fn from(m: WriteMode) -> Self {
        match m {
            WriteMode::Binary => PostBodyDataKind::Unbounded,
            WriteMode::OneLfTerminatedLinePerWriteCall => PostBodyDataKind::BoundedAndFitsIntoMemory,
        }
    }
}

impl<A, B, C> From<PostResponse<A, B, C>> for GetResponse<A, B> {
    fn from(v: PostResponse<A, B, C>) -> Self {
        GetResponse {
            headers: v.headers,
            body: v.body,
        }
    }
}

/// A trait to abstract the HTTP operations needed to power all git interactions: read via GET and write via POST.
/// Note that 401 must be turned into `std::io::Error(PermissionDenied)`, and other non-success http statuses must be transformed
/// into `std::io::Error(Other)`
#[allow(clippy::type_complexity)]
pub trait Http {
    /// A type providing headers line by line.
    type Headers: std::io::BufRead + Unpin;
    /// A type providing the response.
    type ResponseBody: std::io::BufRead;
    /// A type allowing to write the content to post.
    type PostBody: std::io::Write;

    /// Initiate a `GET` request to `url` provided the given `headers`, where `base_url` is so that `base_url + tail == url`.
    ///
    /// The `base_url` helps to validate redirects and to swap it with the effective base after a redirect.
    ///
    /// The `headers` are provided verbatim and include both the key as well as the value.
    fn get(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<GetResponse<Self::Headers, Self::ResponseBody>, Error>;

    /// Initiate a `POST` request to `url` providing with the given `headers`, where `base_url` is so that `base_url + tail == url`.
    ///
    /// The `base_url` helps to validate redirects and to swap it with the effective base after a redirect.
    ///
    /// The `headers` are provided verbatim and include both the key as well as the value.
    /// Note that the [`PostResponse`] contains the [`post_body`][PostResponse::post_body] field which implements [`std::io::Write`]
    /// and is expected to receive the body to post to the server. **It must be dropped** before reading the response
    /// to prevent deadlocks.
    fn post(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
        body: PostBodyDataKind,
    ) -> Result<PostResponse<Self::Headers, Self::ResponseBody, Self::PostBody>, Error>;

    /// Pass `config` which can deserialize in the implementation's configuration, as documented separately.
    ///
    /// The caller must know how that `config` data looks like for the intended implementation.
    fn configure(
        &mut self,
        config: &dyn std::any::Any,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
}
