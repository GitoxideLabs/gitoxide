use std::{ops::DerefMut, sync::Arc, task::ready};

use russh::{
    client::{Config, Handle, Handler},
    MethodSet,
};

pub enum AuthMode {
    UsernamePassword { username: String, password: String },
    PublicKey { username: String },
}

#[derive(Clone)]
pub struct Client {
    handle: Arc<Handle<ClientHandler>>,
}

impl Client {
    pub(super) async fn connect(host: &str, port: u16, auth: AuthMode) -> Result<Self, super::Error> {
        let mut handle = russh::client::connect(Arc::new(Config::default()), (host, port), ClientHandler).await?;

        Self::authenticate(&mut handle, auth).await?;

        Ok(Client {
            handle: Arc::new(handle),
        })
    }

    async fn authenticate(handle: &mut Handle<ClientHandler>, auth: AuthMode) -> Result<(), super::Error> {
        match auth {
            AuthMode::UsernamePassword { username, password } => {
                match handle.authenticate_password(username, password).await? {
                    russh::client::AuthResult::Success => Ok(()),
                    russh::client::AuthResult::Failure {
                        remaining_methods,
                        partial_success: _,
                    } => Err(super::Error::AuthenticationFailed(remaining_methods)),
                }
            }
            AuthMode::PublicKey { username } => {
                let mut agent = russh::keys::agent::client::AgentClient::connect_env().await?;
                let rsa_hash = handle.best_supported_rsa_hash().await?.flatten();
                let mut methods = MethodSet::empty();
                for key in agent.request_identities().await? {
                    match handle
                        .authenticate_publickey_with(&username, key, rsa_hash, &mut agent)
                        .await?
                    {
                        russh::client::AuthResult::Success => return Ok(()),
                        russh::client::AuthResult::Failure {
                            remaining_methods,
                            partial_success: _,
                        } => methods = remaining_methods,
                    }
                }
                Err(super::Error::AuthenticationFailed(methods))
            }
        }
    }

    pub async fn open_session(
        &mut self,
        cmd: impl Into<String>,
        env: Vec<(String, String)>,
    ) -> Result<Session, super::Error> {
        let channel = self.handle.channel_open_session().await?;

        for (key, value) in env {
            channel.set_env(false, key, value).await?;
        }

        channel.exec(false, cmd.into().bytes().collect::<Vec<_>>()).await?;

        let stream = channel.into_stream();
        Ok(Session {
            stream: Arc::new(std::sync::Mutex::new(stream)),
        })
    }
}

#[derive(Clone)]
pub struct Session {
    stream: Arc<std::sync::Mutex<russh::ChannelStream<russh::client::Msg>>>,
}

impl Session {
    fn poll_fn<F, R>(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>, poll_fn: F) -> std::task::Poll<R>
    where
        F: FnOnce(
            std::pin::Pin<&mut russh::ChannelStream<russh::client::Msg>>,
            &mut std::task::Context<'_>,
        ) -> std::task::Poll<R>,
    {
        match self.stream.try_lock() {
            Ok(mut inner) => {
                let pinned = std::pin::Pin::new(inner.deref_mut());
                (poll_fn)(pinned, cx)
            }
            Err(_) => {
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
        }
    }
}

impl futures_io::AsyncRead for Session {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        slice: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.poll_fn(cx, |pinned, cx| {
            let mut buf = tokio::io::ReadBuf::new(slice);
            ready!(tokio::io::AsyncRead::poll_read(pinned, cx, &mut buf))?;
            std::task::Poll::Ready(Ok(buf.filled().len()))
        })
    }
}

impl futures_io::AsyncWrite for Session {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.poll_fn(cx, |pinned, cx| tokio::io::AsyncWrite::poll_write(pinned, cx, buf))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.poll_fn(cx, tokio::io::AsyncWrite::poll_flush)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.poll_fn(cx, tokio::io::AsyncWrite::poll_shutdown)
    }
}

struct ClientHandler;

impl Handler for ClientHandler {
    type Error = super::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: configurable
        Ok(true)
    }
}
