use bstr::ByteSlice;
use futures_lite::future::block_on;
use gix_transport::{
    Protocol, Service,
    server::async_io::{self, Connection},
};

/// Build a packetline data frame: 4-hex-digit length prefix + payload.
fn pkt_line(data: &[u8]) -> Vec<u8> {
    let len = data.len() + 4;
    let mut buf = format!("{len:04x}").into_bytes();
    buf.extend_from_slice(data);
    buf
}

/// A flush packet (0000).
fn pkt_flush() -> Vec<u8> {
    b"0000".to_vec()
}

fn build_connect_packet(message: &[u8]) -> Vec<u8> {
    let mut buf = pkt_line(message);
    buf.extend(pkt_flush());
    buf
}

mod accept {
    use super::*;

    #[test]
    fn upload_pack_v2() {
        block_on(async {
            let input = build_connect_packet(
                b"git-upload-pack /repo.git\0host=example.org\0\0version=2\0",
            );
            let output = futures_lite::io::Cursor::new(Vec::new());

            let (conn, request) = async_io::accept(input.as_slice(), output)
                .await
                .expect("accept succeeds");

            assert_eq!(request.service, Service::UploadPack);
            assert_eq!(request.repository_path, "/repo.git");
            assert_eq!(request.protocol, Protocol::V2);
            assert_eq!(request.virtual_host, Some(("example.org".to_owned(), None)));

            assert_eq!(conn.service, Service::UploadPack);
            assert_eq!(conn.repository_path, "/repo.git");
            assert_eq!(conn.protocol, Protocol::V2);
        });
    }

    #[test]
    fn receive_pack_v1_with_port() {
        block_on(async {
            let input = build_connect_packet(
                b"git-receive-pack /project.git\0host=git.example.com:9418\0",
            );
            let output = futures_lite::io::Cursor::new(Vec::new());

            let (conn, request) = async_io::accept(input.as_slice(), output)
                .await
                .expect("accept succeeds");

            assert_eq!(request.service, Service::ReceivePack);
            assert_eq!(conn.protocol, Protocol::V1);
            assert_eq!(
                request.virtual_host,
                Some(("git.example.com".to_owned(), Some(9418)))
            );
        });
    }

    #[test]
    fn subsequent_data_is_available_through_line_provider() {
        block_on(async {
            let mut input = pkt_line(
                b"git-upload-pack /repo.git\0host=h\0\0version=2\0",
            );
            input.extend(pkt_line(b"command=ls-refs\n"));
            input.extend(pkt_flush());

            let output = futures_lite::io::Cursor::new(Vec::new());
            let (mut conn, _) = async_io::accept(input.as_slice(), output)
                .await
                .expect("accept succeeds");

            conn.line_provider.reset();
            let next_line = conn.line_provider.read_line().await;
            assert!(next_line.is_some(), "subsequent data should be readable");
            let line = next_line.unwrap().expect("io ok").expect("decode ok");
            let text = line.as_text().expect("text line");
            assert_eq!(text.0.as_bstr(), "command=ls-refs".as_bytes().as_bstr());
        });
    }

    #[test]
    fn empty_input_returns_error() {
        block_on(async {
            let input: &[u8] = &[];
            let output = futures_lite::io::Cursor::new(Vec::new());
            let err = async_io::accept(input, output).await.unwrap_err();
            assert!(matches!(err, gix_transport::server::Error::MalformedMessage));
        });
    }

    #[test]
    fn flush_only_input_returns_error() {
        block_on(async {
            let input = pkt_flush();
            let output = futures_lite::io::Cursor::new(Vec::new());
            let err = async_io::accept(input.as_slice(), output).await.unwrap_err();
            assert!(matches!(err, gix_transport::server::Error::MalformedMessage));
        });
    }

    #[test]
    fn unknown_service_returns_error() {
        block_on(async {
            let input = build_connect_packet(b"git-frobnicate /repo.git\0");
            let output = futures_lite::io::Cursor::new(Vec::new());
            let err = async_io::accept(input.as_slice(), output).await.unwrap_err();
            assert!(matches!(err, gix_transport::server::Error::UnknownService { .. }));
        });
    }
}

mod connection_new {
    use super::*;

    #[test]
    fn creates_connection_with_given_parameters() {
        let input: &[u8] = b"";
        let output = futures_lite::io::Cursor::new(Vec::new());
        let conn = Connection::new(input, output, Service::UploadPack, "/repo.git", Protocol::V2);

        assert_eq!(conn.service, Service::UploadPack);
        assert_eq!(conn.repository_path, "/repo.git");
        assert_eq!(conn.protocol, Protocol::V2);
    }

    #[test]
    fn line_provider_reads_subsequent_data() {
        block_on(async {
            let mut input = pkt_line(b"want abc123\n");
            input.extend(pkt_flush());

            let output = futures_lite::io::Cursor::new(Vec::new());
            let mut conn =
                Connection::new(input.as_slice(), output, Service::UploadPack, "/repo.git", Protocol::V1);

            let next_line = conn.line_provider.read_line().await;
            assert!(next_line.is_some());
            let line = next_line.unwrap().expect("io ok").expect("decode ok");
            let text = line.as_text().expect("text line");
            assert_eq!(text.0.as_bstr(), "want abc123".as_bytes().as_bstr());
        });
    }

    #[test]
    fn debug_output_shows_metadata_not_buffers() {
        let input: &[u8] = b"";
        let output = futures_lite::io::Cursor::new(Vec::new());
        let conn =
            Connection::new(input, output, Service::ReceivePack, "/secret/repo.git", Protocol::V1);

        let debug = format!("{conn:?}");
        assert!(debug.contains("ReceivePack"), "should show service");
        assert!(debug.contains("/secret/repo.git"), "should show path");
        assert!(debug.contains("V1"), "should show protocol");
    }
}
