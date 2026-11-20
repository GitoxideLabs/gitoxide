use url::Url;

use crate::Body;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResponseUrl(pub Url);

/// Extension trait for http::response::Builder objects
///
/// Allows the user to add a `Url` to the http::Response
pub trait ResponseBuilderExt {
    /// A builder method for the `http::response::Builder` type that allows the user to add a `Url`
    /// to the `http::Response`
    fn url(self, url: Url) -> Self;
}

/// Extension trait for http::Response objects
///
/// Provides methods to extract URL information from HTTP responses
pub trait ResponseExt {
    /// Returns a reference to the `Url` associated with this response, if available.
    fn url(&self) -> Option<&Url>;
}

impl ResponseBuilderExt for http::response::Builder {
    fn url(self, url: Url) -> Self {
        self.extension(ResponseUrl(url))
    }
}

impl ResponseExt for http::Response<Body> {
    fn url(&self) -> Option<&Url> {
        self.extensions().get::<ResponseUrl>().map(|r| &r.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{ResponseBuilderExt, ResponseUrl};
    use http::response::Builder;
    use url::Url;

    #[test]
    fn test_response_builder_ext() {
        let url = Url::parse("http://example.com").unwrap();
        let response = Builder::new()
            .status(200)
            .url(url.clone())
            .body(())
            .unwrap();

        assert_eq!(
            response.extensions().get::<ResponseUrl>(),
            Some(&ResponseUrl(url))
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_response_ext() {
        use super::ResponseExt;
        let url = Url::parse("http://example.com").unwrap();
        let response = http::Response::builder()
            .status(200)
            .extension(ResponseUrl(url.clone()))
            .body(crate::Body::empty())
            .unwrap();

        assert_eq!(response.url(), Some(&url));
    }
}
