use gix_features::{progress, progress::Progress};
use gix_transport::{Service, client};
use maybe_async::maybe_async;

use super::Error;
use crate::Handshake;
#[cfg(feature = "async-client")]
use crate::transport::client::async_io::{SetServiceResponse, Transport};
#[cfg(feature = "blocking-client")]
use crate::transport::client::blocking_io::{SetServiceResponse, Transport};
use crate::{credentials, handshake::refs};

/// Perform a handshake with the server on the other side of `transport`, with `authenticate` being used if authentication
/// turns out to be required. `extra_parameters` are the parameters `(name, optional value)` to add to the handshake,
/// each time it is performed in case authentication is required.
/// `progress` is used to inform about what's currently happening.
/// The `service` tells the server whether to be in 'send' or 'receive' mode.
#[allow(clippy::result_large_err)]
#[maybe_async]
pub async fn handshake<AuthFn, T>(
    mut transport: T,
    service: Service,
    mut authenticate: AuthFn,
    extra_parameters: Vec<(String, Option<String>)>,
    progress: &mut impl Progress,
) -> Result<Handshake, Error>
where
    AuthFn: FnMut(credentials::helper::Action) -> credentials::protocol::Result,
    T: Transport,
{
    let _span = gix_features::trace::detail!("gix_protocol::handshake()", service = ?service, extra_parameters = ?extra_parameters);
    let (server_protocol_version, refs, capabilities) = {
        progress.init(None, progress::steps());
        progress.set_name("handshake".into());
        progress.step();

        let extra_parameters: Vec<_> = extra_parameters
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_deref()))
            .collect();
        let supported_versions: Vec<_> = transport.supported_protocol_versions().into();

        let result = transport.handshake(service, &extra_parameters).await;
        let SetServiceResponse {
            actual_protocol,
            capabilities,
            refs,
        } = match result {
            Ok(v) => Ok(v),
            Err(client::Error::Io(ref err)) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                drop(result); // needed to workaround this: https://github.com/rust-lang/rust/issues/76149
                let url = transport.to_url().into_owned();
                progress.set_name("authentication".into());
                let credentials::protocol::Outcome { identity, next } =
                    authenticate(credentials::helper::Action::get_for_url(url.clone()))?
                        .ok_or(Error::EmptyCredentials)?;
                transport.set_identity(identity)?;
                progress.step();
                progress.set_name("handshake (authenticated)".into());
                match transport.handshake(service, &extra_parameters).await {
                    Ok(v) => {
                        authenticate(next.store())?;
                        Ok(v)
                    }
                    // Still no permission? Reject the credentials.
                    Err(client::Error::Io(err)) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                        authenticate(next.erase())?;
                        return Err(Error::InvalidCredentials { url, source: err });
                    }
                    // Otherwise, do nothing, as we don't know if it actually got to try the credentials.
                    // If they were previously stored, they remain. In the worst case, the user has to enter them again
                    // next time they try.
                    Err(err) => Err(err),
                }
            }
            Err(err) => Err(err),
        }?;

        if !supported_versions.is_empty() && !supported_versions.contains(&actual_protocol) {
            return Err(Error::TransportProtocolPolicyViolation {
                actual_version: actual_protocol,
            });
        }

        let parsed_refs = match refs {
            Some(mut refs) => {
                assert!(
                    matches!(
                        actual_protocol,
                        gix_transport::Protocol::V0 | gix_transport::Protocol::V1
                    ),
                    "Only V(0|1) auto-responds with refs"
                );
                Some(
                    refs::from_v1_refs_received_as_part_of_handshake_and_capabilities(&mut refs, capabilities.iter())
                        .await?,
                )
            }
            None => None,
        };
        (actual_protocol, parsed_refs, capabilities)
    }; // this scope is needed, see https://github.com/rust-lang/rust/issues/76149

    let (refs, v1_shallow_updates) = refs
        .map(|(refs, shallow)| (Some(refs), Some(shallow)))
        .unwrap_or_default();

    Ok(Handshake {
        server_protocol_version,
        refs,
        v1_shallow_updates,
        capabilities,
    })
}
