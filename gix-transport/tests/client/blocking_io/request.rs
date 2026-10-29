use std::io::{Read, Write};

use gix_transport::{
    client::{MessageKind, WriteMode, blocking_io::RequestWriter},
    packetline::{PacketLineRef, blocking_io::StreamingPeekableIter},
};

#[test]
fn sendable_request_writer_can_upload_while_reading_a_response() -> gix_testtools::TestResult {
    let mut incoming = StreamingPeekableIter::new(&b"0008done0000"[..], &[PacketLineRef::Flush], false);
    let mut outgoing = Vec::new();
    let request = RequestWriter::new_from_bufread_with_send(
        &mut outgoing,
        Box::new(incoming.as_read_without_sidebands()),
        WriteMode::Binary,
        MessageKind::Flush,
        false,
    );
    let (mut writer, mut reader) = request.into_parts_with_send()?;
    let (signal, wait_for_read) = std::sync::mpsc::sync_channel(0);
    std::thread::scope(|scope| -> gix_testtools::TestResult {
        let upload = scope.spawn(move || -> std::io::Result<()> {
            wait_for_read.recv().map_err(std::io::Error::other)?;
            writer.write_all(b"PACK")
        });
        let mut response = String::new();
        reader.read_to_string(&mut response)?;
        signal.send(())?;
        upload.join().expect("upload worker completes without panicking")?;
        assert_eq!(
            response, "done",
            "the response remains readable while the upload is pending"
        );
        Ok(())
    })?;
    drop(reader);
    assert_eq!(
        outgoing, b"PACK",
        "the split writer does not encode pack bytes as packet lines"
    );
    Ok(())
}

#[test]
fn non_send_request_writers_still_support_existing_transports() -> gix_testtools::TestResult {
    struct LocalWriter(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);
    impl Write for LocalWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let output = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut incoming = StreamingPeekableIter::new(std::io::empty(), &[PacketLineRef::Flush], false);
    let mut request = RequestWriter::new_from_bufread(
        LocalWriter(output.clone()),
        Box::new(incoming.as_read_without_sidebands()),
        WriteMode::Binary,
        MessageKind::Flush,
        false,
    );
    request.write_all(b"want")?;
    let _response = request.into_read()?;
    assert_eq!(
        &*output.borrow(),
        b"0008want0000",
        "non-Send transports retain packet framing and flush behavior"
    );
    drop(_response);
    let request = RequestWriter::new_from_bufread(
        LocalWriter(output),
        Box::new(incoming.as_read_without_sidebands()),
        WriteMode::Binary,
        MessageKind::Flush,
        false,
    );
    assert!(
        request.into_parts_with_send().is_err(),
        "duplex callers get an explicit error for non-Send writers"
    );
    Ok(())
}
