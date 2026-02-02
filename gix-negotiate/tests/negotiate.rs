use gix_testtools::{size_ok, Result};

mod window_size {
    use gix_negotiate::window_size;

    #[test]
    fn initial_value_without_previous_window_size() {
        assert_eq!(window_size(false, None), 16);
        assert_eq!(window_size(true, None), 16);
    }

    #[test]
    fn transport_is_stateless() {
        let mut ws = window_size(true, None);
        for expected in [32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 18022, 19824] {
            ws = window_size(true, Some(ws));
            assert_eq!(ws, expected);
        }
    }

    #[test]
    fn transport_is_not_stateless() {
        let mut ws = window_size(false, None);
        for expected in [32, 64, 96] {
            ws = window_size(false, Some(ws));
            assert_eq!(ws, expected);
        }

        let mut ws = 4;
        for expected in [8, 16, 32, 64, 96] {
            ws = window_size(false, Some(ws));
            assert_eq!(ws, expected);
        }
    }
}

mod baseline;

#[test]
fn size_of_entry() {
    let actual = std::mem::size_of::<gix_revwalk::graph::Commit<gix_negotiate::Metadata>>();
    let sha1 = 56;
    let sha256_extra = 16;
    let expected = sha1 + sha256_extra;
    assert!(
        size_ok(actual, expected),
        "we may keep a lot of these, so let's not let them grow unnoticed: {actual} <~ {expected}"
    );
}
