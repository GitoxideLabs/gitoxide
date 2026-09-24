mod io {
    use std::io::{BufRead, ErrorKind, Read, Write};

    use gix_features::io;

    #[test]
    fn threaded_read_to_end() {
        let (mut writer, mut reader) = gix_features::io::pipe::unidirectional(0);

        let message = "Hello, world!";
        std::thread::spawn(move || {
            writer
                .write_all(message.as_bytes())
                .expect("writes to work if reader is present");
        });

        let mut received = String::new();
        reader.read_to_string(&mut received).unwrap();

        assert_eq!(&received, message);
    }

    #[test]
    fn lack_of_reader_fails_with_broken_pipe() {
        let (mut writer, _) = io::pipe::unidirectional(0);
        let err = writer.write_all(b"must fail").expect_err("the operation must fail");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "lack of reader fails with broken pipe", @"
        Custom {
            kind: BrokenPipe,
            error: SendError { .. },
        }
        ");
        assert_eq!(err.kind(), ErrorKind::BrokenPipe);
    }
    #[test]
    fn line_reading_one_by_one() {
        let (mut writer, mut reader) = io::pipe::unidirectional(2);
        writer.write_all(b"a\n").expect("success");
        writer.write_all(b"b\nc").expect("success");
        drop(writer);
        let mut buf = String::new();
        for expected in &["a\n", "b\n", "c"] {
            buf.clear();
            assert_eq!(reader.read_line(&mut buf).expect("success"), expected.len());
            assert_eq!(buf, *expected);
        }
    }

    #[test]
    fn line_reading() {
        let (mut writer, reader) = io::pipe::unidirectional(2);
        writer.write_all(b"a\n").expect("success");
        writer.write_all(b"b\nc\n").expect("success");
        drop(writer);
        assert_eq!(
            reader.lines().map_while(Result::ok).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn writer_can_inject_errors() {
        let (writer, mut reader) = io::pipe::unidirectional(1);
        writer
            .channel
            .send(Err(std::io::Error::other("the error")))
            .expect("send success");
        let mut buf = [0];
        insta::assert_debug_snapshot!(reader.read(&mut buf).expect_err("using Read trait, errors are propagated"), "using Read trait, errors are propagated", @r#"
        Custom {
            kind: Other,
            error: "the error",
        }
        "#);

        writer
            .channel
            .send(Err(std::io::Error::other("the error")))
            .expect("send success");
        insta::assert_debug_snapshot!(reader.fill_buf().expect_err("using BufRead trait, errors are propagated"), "using BufRead trait, errors are propagated", @r#"
        Custom {
            kind: Other,
            error: "the error",
        }
        "#);
    }

    #[test]
    fn continue_on_empty_writes() {
        let (mut writer, mut reader) = io::pipe::unidirectional(2);
        writer.write_all(&[]).expect("write successful and non-blocking");
        let input = b"hello";
        writer
            .write_all(input)
            .expect("second write works as well as there is capacity");
        let mut buf = vec![0u8; input.len()];
        assert_eq!(reader.read(&mut buf).expect("read succeeds"), input.len());
        assert_eq!(buf, &input[..]);
    }

    #[test]
    fn small_reads() {
        const BLOCK_SIZE: usize = 20;
        let block_count = 20;
        let (mut writer, mut reader) = io::pipe::unidirectional(4);
        std::thread::spawn(move || {
            for _ in 0..block_count {
                let data = &[0; BLOCK_SIZE];
                writer.write_all(data).unwrap();
            }
        });

        let mut small_read_buf = [0; BLOCK_SIZE / 2];
        let mut bytes_read = 0;
        while let Ok(size) = reader.read(&mut small_read_buf) {
            if size == 0 {
                break;
            }
            bytes_read += size;
        }
        assert_eq!(block_count * BLOCK_SIZE, bytes_read);
    }
}
