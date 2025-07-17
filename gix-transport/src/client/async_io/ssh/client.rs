use std::sync::Arc;

use russh::{
    client::Handle,
    client::{Config, Handler},
};

pub enum AuthMode {
    UsernamePassword { username: String, password: String },
}

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
        }
    }
}

struct ClientHandler;

impl Handler for ClientHandler {
    type Error = super::Error;
}
