use bstr::ByteSlice;
use gix_transport::{
    Protocol, Service,
    packetline::blocking_io::encode,
    server::blocking_io::{self, Connection},
};

fn build_connect_packet(message: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    encode::data_to_write(message, &mut buf).expect("encoding works");
    encode::flush_to_write(&mut buf).expect("flush works");
    buf
}

mod accept {
    use super::*;

    #[test]
    fn upload_pack_v2() {
        let input = build_connect_packet(b"git-upload-pack /repo.git\0host=example.org\0\0version=2\0");
        let output = Vec::new();

        let (conn, request) = blocking_io::accept(input.as_slice(), output).expect("accept succeeds");

        assert_eq!(request.service, Service::UploadPack);
        assert_eq!(request.repository_path, "/repo.git");
        assert_eq!(request.protocol, Protocol::V2);
        assert_eq!(request.virtual_host, Some(("example.org".to_owned(), None)));

        assert_eq!(conn.service, Service::UploadPack);
        assert_eq!(conn.repository_path, "/repo.git");
        assert_eq!(conn.protocol, Protocol::V2);
    }

    #[test]
    fn receive_pack_v1_with_port() {
        let input = build_connect_packet(b"git-receive-pack /project.git\0host=git.example.com:9418\0");
        let output = Vec::new();

        let (conn, request) = blocking_io::accept(input.as_slice(), output).expect("accept succeeds");

        assert_eq!(request.service, Service::ReceivePack);
        assert_eq!(conn.protocol, Protocol::V1);
        assert_eq!(request.virtual_host, Some(("git.example.com".to_owned(), Some(9418))));
    }

    #[test]
    fn subsequent_data_is_available_through_line_provider() {
        let mut input = Vec::new();
        encode::data_to_write(b"git-upload-pack /repo.git\0host=h\0\0version=2\0", &mut input)
            .expect("encode works");
        // Simulate a subsequent command the client sends after the connect message.
        encode::data_to_write(b"command=ls-refs\n", &mut input).expect("encode works");
        encode::flush_to_write(&mut input).expect("flush works");

        let output = Vec::new();
        let (mut conn, _request) = blocking_io::accept(input.as_slice(), output).expect("accept succeeds");

        // The line provider should be able to read the next packetline.
        conn.line_provider.reset();
        let next_line = conn.line_provider.read_line();
        assert!(next_line.is_some(), "subsequent data should be readable");
        let line = next_line.unwrap().expect("io ok").expect("decode ok");
        let text = line.as_text().expect("text line");
        assert_eq!(text.as_bstr(), "command=ls-refs".as_bytes().as_bstr());
    }

    #[test]
    fn empty_input_returns_error() {
        let input: &[u8] = &[];
        let output = Vec::new();
        let err = blocking_io::accept(input, output).unwrap_err();
        assert!(
            matches!(err, gix_transport::server::Error::MalformedMessage),
            "empty input should be malformed"
        );
    }

    #[test]
    fn flush_only_input_returns_error() {
        let mut input = Vec::new();
        encode::flush_to_write(&mut input).expect("flush works");

        let output = Vec::new();
        let err = blocking_io::accept(input.as_slice(), output).unwrap_err();
        assert!(
            matches!(err, gix_transport::server::Error::MalformedMessage),
            "a flush without data should be malformed"
        );
    }

    #[test]
    fn unknown_service_returns_error() {
        let input = build_connect_packet(b"git-frobnicate /repo.git\0");
        let output = Vec::new();
        let err = blocking_io::accept(input.as_slice(), output).unwrap_err();
        assert!(matches!(err, gix_transport::server::Error::UnknownService { .. }));
    }
}

mod connection_new {
    use super::*;

    #[test]
    fn creates_connection_with_given_parameters() {
        let input: &[u8] = b"";
        let output = Vec::new();
        let conn = Connection::new(input, output, Service::UploadPack, "/repo.git", Protocol::V2);

        assert_eq!(conn.service, Service::UploadPack);
        assert_eq!(conn.repository_path, "/repo.git");
        assert_eq!(conn.protocol, Protocol::V2);
    }

    #[test]
    fn line_provider_reads_subsequent_data() {
        let mut input = Vec::new();
        encode::data_to_write(b"want abc123\n", &mut input).expect("encode works");
        encode::flush_to_write(&mut input).expect("flush works");

        let output = Vec::new();
        let mut conn = Connection::new(input.as_slice(), output, Service::UploadPack, "/repo.git", Protocol::V1);

        let next_line = conn.line_provider.read_line();
        assert!(next_line.is_some());
        let line = next_line.unwrap().expect("io ok").expect("decode ok");
        let text = line.as_text().expect("text line");
        assert_eq!(text.as_bstr(), "want abc123".as_bytes().as_bstr());
    }

    #[test]
    fn debug_output_shows_metadata_not_buffers() {
        let input: &[u8] = b"";
        let output = Vec::new();
        let conn = Connection::new(input, output, Service::ReceivePack, "/secret/repo.git", Protocol::V1);

        let debug = format!("{conn:?}");
        assert!(debug.contains("ReceivePack"), "should show service");
        assert!(debug.contains("/secret/repo.git"), "should show path");
        assert!(debug.contains("V1"), "should show protocol");
    }
}
