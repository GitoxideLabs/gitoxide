use crate::async_impl::h3_client::dns::resolve;
use crate::dns::DynResolver;
use crate::error::BoxError;
use bytes::Bytes;
use h3::client::SendRequest;
use h3_quinn::{Connection, OpenStreams};
use http::Uri;
use hyper_util::client::legacy::connect::dns::Name;
use quinn::crypto::rustls::QuicClientConfig;
use quinn::{ClientConfig, Endpoint, TransportConfig};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

type H3Connection = (
    h3::client::Connection<Connection, Bytes>,
    SendRequest<OpenStreams, Bytes>,
);

const HAPPY_EYEBALLS_DELAY: Duration = Duration::from_millis(250);

/// H3 Client Config
#[derive(Clone)]
pub(crate) struct H3ClientConfig {
    /// Set the maximum HTTP/3 header size this client is willing to accept.
    ///
    /// See [header size constraints] section of the specification for details.
    ///
    /// [header size constraints]: https://www.rfc-editor.org/rfc/rfc9114.html#name-header-size-constraints
    ///
    /// Please see docs in [`Builder`] in [`h3`].
    ///
    /// [`Builder`]: https://docs.rs/h3/latest/h3/client/struct.Builder.html#method.max_field_section_size
    pub(crate) max_field_section_size: Option<u64>,

    /// Enable whether to send HTTP/3 protocol grease on the connections.
    ///
    /// Just like in HTTP/2, HTTP/3 also uses the concept of "grease"
    ///
    /// to prevent potential interoperability issues in the future.
    /// In HTTP/3, the concept of grease is used to ensure that the protocol can evolve
    /// and accommodate future changes without breaking existing implementations.
    ///
    /// Please see docs in [`Builder`] in [`h3`].
    ///
    /// [`Builder`]: https://docs.rs/h3/latest/h3/client/struct.Builder.html#method.send_grease
    pub(crate) send_grease: Option<bool>,
}

impl Default for H3ClientConfig {
    fn default() -> Self {
        Self {
            max_field_section_size: None,
            send_grease: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct H3Connector {
    resolver: DynResolver,
    endpoint: Endpoint,
    client_config: H3ClientConfig,
    local_addr: Option<IpAddr>,
}

impl H3Connector {
    pub fn new(
        resolver: DynResolver,
        tls: rustls::ClientConfig,
        local_addr: Option<IpAddr>,
        transport_config: TransportConfig,
        client_config: H3ClientConfig,
    ) -> Result<H3Connector, BoxError> {
        let quic_client_config = Arc::new(QuicClientConfig::try_from(tls)?);
        let mut config = ClientConfig::new(quic_client_config);
        // FIXME: Replace this when there is a setter.
        config.transport_config(Arc::new(transport_config));

        // Pipe the local address through to the endpoint creation
        let socket_addr = match local_addr {
            Some(ip) => SocketAddr::new(ip, 0),
            None => "[::]:0".parse::<SocketAddr>().unwrap(),
        };

        let mut endpoint = Endpoint::client(socket_addr)?;
        endpoint.set_default_client_config(config);

        Ok(Self {
            resolver,
            endpoint,
            client_config,
            local_addr,
        })
    }

    pub async fn connect(&mut self, dest: Uri) -> Result<H3Connection, BoxError> {
        let host = dest
            .host()
            .ok_or("destination must have a host")?
            .trim_start_matches('[')
            .trim_end_matches(']');
        let port = dest.port_u16().unwrap_or(443);

        let addrs = if let Some(addr) = IpAddr::from_str(host).ok() {
            // If the host is already an IP address, skip resolving.
            vec![SocketAddr::new(addr, port)]
        } else {
            let addrs = resolve(&mut self.resolver, Name::from_str(host)?).await?;
            let addrs = addrs.map(|mut addr| {
                addr.set_port(port);
                addr
            });
            addrs.collect()
        };

        self.remote_connect(addrs, host).await
    }

    async fn remote_connect(
        &mut self,
        addrs: Vec<SocketAddr>,
        server_name: &str,
    ) -> Result<H3Connection, BoxError> {
        if addrs.is_empty() {
            return Err("no addresses to connect to".into());
        }

        let (mut ipv6_addrs, mut ipv4_addrs): (Vec<SocketAddr>, Vec<SocketAddr>) =
            addrs.into_iter().partition(|addr| addr.is_ipv6());

        if let Some(local_ip) = self.local_addr {
            if local_ip.is_ipv6() {
                ipv4_addrs.clear();
            } else {
                ipv6_addrs.clear();
            }
        }

        if ipv6_addrs.is_empty() {
            return Self::try_addresses_static(
                &self.endpoint,
                &ipv4_addrs,
                server_name,
                &self.client_config,
            )
            .await;
        }
        if ipv4_addrs.is_empty() {
            return Self::try_addresses_static(
                &self.endpoint,
                &ipv6_addrs,
                server_name,
                &self.client_config,
            )
            .await;
        }

        let endpoint = self.endpoint.clone();
        let client_config = self.client_config.clone();

        if self.local_addr.is_some() {
            return match Self::try_addresses_static(
                &endpoint,
                &ipv6_addrs,
                server_name,
                &client_config,
            )
            .await
            {
                Ok(conn) => Ok(conn),
                Err(_) => {
                    Self::try_addresses_static(&endpoint, &ipv4_addrs, server_name, &client_config)
                        .await
                }
            };
        }

        Self::try_addresses_happy_eyeballs(
            &endpoint,
            &ipv6_addrs,
            &ipv4_addrs,
            server_name,
            &client_config,
        )
        .await
    }

    async fn try_addresses_static(
        endpoint: &Endpoint,
        addrs: &[SocketAddr],
        server_name: &str,
        client_config: &H3ClientConfig,
    ) -> Result<H3Connection, BoxError> {
        let mut last_err: Option<BoxError> = None;

        for addr in addrs {
            match endpoint.connect(*addr, server_name) {
                Ok(connecting) => match connecting.await {
                    Ok(new_conn) => {
                        let quinn_conn = Connection::new(new_conn);
                        let mut h3_client_builder = h3::client::builder();
                        if let Some(max_field_section_size) = client_config.max_field_section_size {
                            h3_client_builder.max_field_section_size(max_field_section_size);
                        }
                        if let Some(send_grease) = client_config.send_grease {
                            h3_client_builder.send_grease(send_grease);
                        }
                        return Ok(h3_client_builder.build(quinn_conn).await?);
                    }
                    Err(e) => {
                        last_err = Some(Box::new(e) as BoxError);
                    }
                },
                Err(e) => {
                    last_err = Some(Box::new(e) as BoxError);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| "no addresses available".into()))
    }

    async fn try_addresses_happy_eyeballs(
        endpoint: &Endpoint,
        ipv6_addrs: &[SocketAddr],
        ipv4_addrs: &[SocketAddr],
        server_name: &str,
        client_config: &H3ClientConfig,
    ) -> Result<H3Connection, BoxError> {
        let ipv6_connect =
            Self::try_addresses_static(endpoint, ipv6_addrs, server_name, client_config);
        tokio::pin!(ipv6_connect);

        let delay = tokio::time::sleep(HAPPY_EYEBALLS_DELAY);
        tokio::pin!(delay);

        tokio::select! {
            result = &mut ipv6_connect => {
                return match result {
                    Ok(conn) => Ok(conn),
                    Err(_) => {
                        Self::try_addresses_static(endpoint, ipv4_addrs, server_name, client_config).await
                    }
                };
            }
            _ = &mut delay => {}
        }

        let ipv4_connect =
            Self::try_addresses_static(endpoint, ipv4_addrs, server_name, client_config);
        tokio::pin!(ipv4_connect);

        let wait_for_ipv6 = tokio::select! {
            result = &mut ipv6_connect => {
                match result {
                    Ok(conn) => return Ok(conn),
                    Err(_) => false,
                }
            }
            result = &mut ipv4_connect => {
                match result {
                    Ok(conn) => return Ok(conn),
                    Err(_) => true,
                }
            }
        };

        if wait_for_ipv6 {
            ipv6_connect.await
        } else {
            ipv4_connect.await
        }
    }
}
