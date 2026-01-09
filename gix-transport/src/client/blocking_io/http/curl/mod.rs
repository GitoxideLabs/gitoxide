use std::{
    sync::mpsc::{Receiver, SyncSender},
    thread,
};

use gix_features::io;

use crate::client::blocking_io::http::{self, traits::PostBodyDataKind};

mod remote;

/// Options to configure the `curl` HTTP handler.
#[derive(Default)]
pub struct Options {
    /// If `true` and runtime configuration is possible for `curl` backends, certificates revocation will be checked.
    ///
    /// This only works on windows apparently. Ignored if `None`.
    pub schannel_check_revoke: Option<bool>,
}

/// The error returned by the 'remote' helper, a purely internal construct to perform http requests.
///
/// It can be used for downcasting errors, which are boxed to hide the actual implementation.
pub type Error = crate::Error;

pub(crate) fn curl_is_spurious(err: &curl::Error) -> bool {
    err.is_couldnt_connect()
        || err.is_couldnt_resolve_proxy()
        || err.is_couldnt_resolve_host()
        || err.is_operation_timedout()
        || err.is_recv_error()
        || err.is_send_error()
        || err.is_http2_error()
        || err.is_http2_stream_error()
        || err.is_ssl_connect_error()
        || err.is_partial_file()
}

/// A utility to abstract interactions with curl handles.
pub struct Curl {
    req: SyncSender<remote::Request>,
    res: Receiver<remote::Response>,
    handle: Option<thread::JoinHandle<Result<(), Error>>>,
    config: http::Options,
}

impl Curl {
    fn restore_thread_after_failure(&mut self) -> http::Error {
        let err_that_brought_thread_down = self
            .handle
            .take()
            .expect("thread handle present")
            .join()
            .expect("handler thread should never panic")
            .expect_err("something should have gone wrong with curl (we join on error only)");
        let (handle, req, res) = remote::new();
        self.handle = Some(handle);
        self.req = req;
        self.res = res;
        err_that_brought_thread_down.into()
    }

    fn make_request(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
        upload_body_kind: Option<PostBodyDataKind>,
    ) -> Result<http::PostResponse<io::pipe::Reader, io::pipe::Reader, io::pipe::Writer>, http::Error> {
        let mut list = curl::easy::List::new();
        for header in headers {
            list.append(header.as_ref())?;
        }
        if self
            .req
            .send(remote::Request {
                url: url.to_owned(),
                base_url: base_url.to_owned(),
                headers: list,
                upload_body_kind,
                config: self.config.clone(),
            })
            .is_err()
        {
            return Err(self.restore_thread_after_failure());
        }
        let remote::Response {
            headers,
            body,
            upload_body,
        } = match self.res.recv() {
            Ok(res) => res,
            Err(_) => return Err(self.restore_thread_after_failure()),
        };
        Ok(http::PostResponse {
            post_body: upload_body,
            headers,
            body,
        })
    }
}

impl Default for Curl {
    fn default() -> Self {
        let (handle, req, res) = remote::new();
        Curl {
            handle: Some(handle),
            req,
            res,
            config: http::Options::default(),
        }
    }
}

#[allow(clippy::type_complexity)]
impl http::Http for Curl {
    type Headers = io::pipe::Reader;
    type ResponseBody = io::pipe::Reader;
    type PostBody = io::pipe::Writer;

    fn get(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<http::GetResponse<Self::Headers, Self::ResponseBody>, http::Error> {
        self.make_request(url, base_url, headers, None).map(Into::into)
    }

    fn post(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
        body: PostBodyDataKind,
    ) -> Result<http::PostResponse<Self::Headers, Self::ResponseBody, Self::PostBody>, http::Error> {
        self.make_request(url, base_url, headers, Some(body))
    }

    fn configure(
        &mut self,
        config: &dyn std::any::Any,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        if let Some(config) = config.downcast_ref::<http::Options>() {
            self.config = config.clone();
        }
        Ok(())
    }
}
