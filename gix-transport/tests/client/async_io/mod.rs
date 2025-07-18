use gix_transport::Service;

#[cfg(feature = "russh")]
#[tokio::test]
async fn test_native_ssh_handshake() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = gix_transport::connect(
        gix_url::Url::try_from("ssh://git@github.com/GitoxideLabs/gitoxide.git").expect("url is valid"),
        gix_transport::connect::Options {
            version: gix_transport::Protocol::V2,
            trace: true,
        },
    )
    .await?;

    client.handshake(Service::UploadPack, &[]).await?;
    Ok(())
}
