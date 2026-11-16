use super::*;

#[tokio::test(flavor = "current_thread")]
async fn fragmented_messages_limits_and_cancelled_reads() -> gix_error::TestResult {
    let (mut reader, mut writer) = tokio::io::duplex(8);
    let write = tokio::spawn(async move {
        for byte in b"0009token0008\0a\0b0000" {
            writer.write_all(&[*byte]).await?;
        }
        Ok::<_, io::Error>(())
    });
    assert_eq!(
        read_message(&mut reader, 9).await?,
        Some(b"token\0a\0b".to_vec()),
        "fragmented packets preserve binary payloads"
    );
    write.await??;
    for mut input in [b"000d".as_slice(), b"0001", b"zzzz", b"0005x"] {
        assert!(
            read_message(&mut input, 8).await.is_err(),
            "malformed, oversized or truncated messages fail before trusting a response"
        );
    }
    let (mut reader, _writer) = tokio::io::duplex(8);
    assert!(
        tokio::time::timeout(Duration::from_millis(10), read_message(&mut reader, 8))
            .await
            .is_err(),
        "a stalled read yields to the runtime timer"
    );
    Ok(())
}
