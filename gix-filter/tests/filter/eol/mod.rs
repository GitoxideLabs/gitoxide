mod stats {
    mod from_bytes {
        use gix_filter::eol;

        #[test]
        fn all() {
            let stats = eol::Stats::from_bytes(b"\n\r\nhi\rho\0\tanother line\nother\r\nmixed");
            assert_eq!(
                stats,
                eol::Stats {
                    null: 1,
                    lone_cr: 1,
                    lone_lf: 2,
                    crlf: 2,
                    printable: 27,
                    non_printable: 1,
                }
            );
            assert!(stats.is_binary());
        }

        #[test]
        fn trailing_dos_eof_marker_is_not_counted_as_non_printable() {
            let stats = eol::Stats::from_bytes(b"hello\r\n\x1a");
            assert_eq!(
                stats,
                eol::Stats {
                    null: 0,
                    lone_cr: 0,
                    lone_lf: 0,
                    crlf: 1,
                    printable: 5,
                    non_printable: 0,
                }
            );
            assert!(
                !stats.is_binary(),
                "text that ends with a DOS EOF (\x1a) marker is still text"
            );

            let stats = eol::Stats::from_bytes(b"hello\r\n\x1a\x1a");
            assert_eq!(stats.non_printable, 1, "only the very last byte is discounted");
            assert!(
                stats.is_binary(),
                "a marker anywhere else keeps counting as unprintable"
            );
        }
    }
}

pub(crate) mod convert_to_git;
mod convert_to_worktree;
