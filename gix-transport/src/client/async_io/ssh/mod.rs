use crate::{
    client::{SetServiceResponse, Transport, TransportWithoutIO},
    Protocol, Service,
};
use async_trait::async_trait;

mod client;
mod error;

pub use error::Error;

pub struct NativeSsh {
    url: gix_url::Url,
    desired_version: Protocol,
    trace: bool,

    identity: Option<gix_sec::identity::Account>,
    client: Option<client::Client>,
    connection: Option<crate::client::git::Connection<client::Session, client::Session>>,
}

impl TransportWithoutIO for NativeSsh {
    fn set_identity(&mut self, identity: gix_sec::identity::Account) -> Result<(), crate::client::Error> {
        self.identity = Some(identity);
        Ok(())
    }

    fn request(
        &mut self,
        write_mode: crate::client::WriteMode,
        on_into_read: crate::client::MessageKind,
        trace: bool,
    ) -> Result<super::RequestWriter<'_>, crate::client::Error> {
        if let Some(connection) = &mut self.connection {
            connection.request(write_mode, on_into_read, trace)
        } else {
            Err(crate::client::Error::MissingHandshake)
        }
    }

    fn to_url(&self) -> std::borrow::Cow<'_, bstr::BStr> {
        self.url.to_bstring().into()
    }

    fn connection_persists_across_multiple_requests(&self) -> bool {
        true
    }

    fn configure(
        &mut self,
        _config: &dyn std::any::Any,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        Ok(())
    }
}

#[async_trait(?Send)]
impl Transport for NativeSsh {
    async fn handshake<'a>(
        &mut self,
        service: Service,
        extra_parameters: &'a [(&'a str, Option<&'a str>)],
    ) -> Result<SetServiceResponse<'_>, crate::client::Error> {
        let host = self.url.host().expect("url has host");
        let port = self.url.port_or_default().expect("ssh has a default port");

        let auth_mode = match self.identity.as_ref() {
            Some(crate::client::Account {
                username,
                password,
                oauth_refresh_token: _,
            }) => client::AuthMode::UsernamePassword {
                username: username.clone(),
                password: password.clone(),
            },
            None => client::AuthMode::PublicKey {
                username: self
                    .url
                    .user()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default(),
            },
        };

        let mut client = client::Client::connect(host, port, auth_mode).await?;

        let session = client
            .open_session(
                format!("{} {}", service.as_str(), self.url.path),
                vec![(
                    "GIT_PROTOCOL".to_string(),
                    format!("version={}", self.desired_version as usize),
                )],
            )
            .await?;

        let connection = crate::client::git::Connection::new(
            session.clone(),
            session,
            self.desired_version,
            self.url.path.clone(),
            None::<(String, _)>,
            crate::client::git::ConnectMode::Process,
            self.trace,
        );

        self.client = Some(client);
        self.connection = Some(connection);

        self.connection
            .as_mut()
            .expect("connection to be there right after setting it")
            .handshake(service, extra_parameters)
            .await
    }
}

#[allow(clippy::unused_async)]
pub async fn connect(
    url: gix_url::Url,
    desired_version: Protocol,
    trace: bool,
) -> Result<NativeSsh, crate::client::connect::Error> {
    if url.scheme != gix_url::Scheme::Ssh {
        return Err(crate::client::connect::Error::UnsupportedScheme(url.scheme));
    }
    Ok(NativeSsh {
        url,
        desired_version,
        trace,
        identity: None,
        client: None,
        connection: None,
    })
}
