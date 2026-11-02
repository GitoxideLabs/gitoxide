#[cfg(any(feature = "http-client-curl", feature = "http-client-reqwest"))]
mod http;

#[test]
fn ssh_username_password_credentials_update_or_clear_username() -> gix_error::TestResult {
    use gix_transport::{
        Protocol,
        client::{Credentials, TransportWithoutIO, blocking_io::ssh},
    };

    let mut transport = ssh::connect(
        "ssh://original@example.com/repo".try_into()?,
        Protocol::V2,
        ssh::connect::Options {
            kind: Some(ssh::ProgramKind::Ssh),
            ..Default::default()
        },
        false,
    )?;
    for (username, expected_url) in [
        ("updated", "ssh://updated@example.com/repo"),
        ("", "ssh://example.com/repo"),
    ] {
        transport.set_credentials(Credentials::UsernamePassword {
            username: username.into(),
            password: "unused-password".into(),
            oauth_refresh_token: None,
        })?;
        assert_eq!(
            transport.to_url().as_ref(),
            expected_url,
            "SSH uses only the username and clears it when empty"
        );
    }
    Ok(())
}

#[test]
fn ssh_rejects_encoded_credentials_without_mutating_url() -> gix_error::TestResult {
    use gix_transport::{
        Protocol,
        client::{Credentials, Error, TransportWithoutIO, blocking_io::ssh},
    };

    for url in ["ssh://original@example.com/repo", "ssh://example.com/repo"] {
        let mut transport = ssh::connect(
            url.try_into()?,
            Protocol::V2,
            ssh::connect::Options {
                kind: Some(ssh::ProgramKind::Ssh),
                ..Default::default()
            },
            false,
        )?;
        let err = transport
            .set_credentials(Credentials::Encoded {
                authtype: "Bearer".into(),
                credential: "secret-token".into(),
                oauth_refresh_token: None,
            })
            .expect_err("SSH cannot use pre-encoded HTTP credentials");
        assert!(
            matches!(err, Error::AuthenticationUnsupported),
            "encoded credentials are explicitly unsupported by SSH"
        );
        assert_eq!(
            transport.to_url().as_ref(),
            url,
            "rejecting encoded credentials must not change the existing SSH URL"
        );
    }
    Ok(())
}
