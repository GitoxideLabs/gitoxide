use std::path::PathBuf;

use gix_testtools::scripted_fixture_read_only;

pub(crate) fn repo_path(name: &str) -> PathBuf {
    let dir = scripted_fixture_read_only("make_remote_repos.sh").unwrap();
    dir.join(name)
}

pub(crate) fn repo(name: &str) -> gix::Repository {
    gix::open_opts(repo_path(name), gix::open::Options::isolated()).unwrap()
}

/// Spawn a git-daemon hosting all directories in or below `base_dir` if we are in async mode - currently only TCP is
/// available in async mode, and it's probably going to stay that way as we don't want to chose a particular runtime
/// in lower-level crates just yet.
/// Maybe this changes one day once we implement other protocols like spawning a process via `tokio` or `async-std`, or
/// provide async HTTP implementations as well.
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client-async-std"))]
pub(crate) fn spawn_git_daemon_if_async(
    _base_dir: impl AsRef<std::path::Path>,
) -> std::io::Result<Option<gix_testtools::GitDaemon>> {
    #[cfg(feature = "blocking-network-client")]
    {
        Ok(None)
    }
    #[cfg(feature = "async-network-client-async-std")]
    {
        gix_testtools::spawn_git_daemon(_base_dir).map(Some)
    }
}

/// Turn `remote` into a remote that interacts with the git `daemon`, all else being the same, by creating a new stand-in remote.
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client-async-std"))]
pub(crate) fn into_daemon_remote_if_async<'repo, 'a>(
    remote: gix::Remote<'repo>,
    _daemon: Option<&gix_testtools::GitDaemon>,
    _repo_name: impl Into<Option<&'a str>>,
) -> gix::Remote<'repo> {
    #[cfg(feature = "blocking-network-client")]
    {
        remote
    }
    #[cfg(feature = "async-network-client-async-std")]
    {
        let mut new_remote = remote
            .repo()
            .remote_at(format!(
                "{}/{}",
                _daemon.expect("daemon is available in async mode").url,
                _repo_name.into().unwrap_or_default()
            ))
            .expect("valid url to create remote at")
            .with_fetch_tags(remote.fetch_tags());
        for direction in [gix::remote::Direction::Fetch, gix::remote::Direction::Push] {
            new_remote
                .replace_refspecs(
                    remote.refspecs(direction).iter().map(|s| s.to_ref().to_bstring()),
                    direction,
                )
                .expect("input refspecs valid");
        }
        new_remote
    }
}

mod connect;
mod default_branch;
pub(crate) mod fetch;
mod ref_map;
mod save;
mod name {
    use std::borrow::Cow;

    use gix::{
        bstr::{BStr, BString, ByteSlice},
        remote::Name,
    };
    use gix_testtools::TestResult;

    macro_rules! assert_natural_equality {
        ($value:ident, $matching:literal, $different:literal) => {{
            let matching = $matching;
            let matching_string = matching.to_owned();
            let matching_bstr: &BStr = matching.as_bytes().as_bstr();
            let matching_bstring: BString = matching.as_bytes().into();

            assert_eq!($value, matching, "the name matches str");
            assert_eq!(matching, $value, "str comparison is symmetric");
            assert_eq!($value, matching_string, "the name matches String");
            assert_eq!(matching_string, $value, "String comparison is symmetric");
            assert_eq!($value, matching_bstr, "the name matches BStr");
            assert_eq!(matching_bstr, $value, "BStr comparison is symmetric");
            assert_eq!($value, matching_bstring, "the name matches BString");
            assert_eq!(matching_bstring, $value, "BString comparison is symmetric");

            let different = $different;
            let different_string = different.to_owned();
            let different_bstr: &BStr = different.as_bytes().as_bstr();
            let different_bstring: BString = different.as_bytes().into();

            assert_ne!($value, different, "the name differs from str");
            assert_ne!(different, $value, "str inequality is symmetric");
            assert_ne!($value, different_string, "the name differs from String");
            assert_ne!(different_string, $value, "String inequality is symmetric");
            assert_ne!($value, different_bstr, "the name differs from BStr");
            assert_ne!(different_bstr, $value, "BStr inequality is symmetric");
            assert_ne!($value, different_bstring, "the name differs from BString");
            assert_ne!(different_bstring, $value, "BString inequality is symmetric");
        }};
    }

    #[test]
    fn compares_with_text_and_byte_strings() -> TestResult {
        let symbol = Name::symbol("origin")?;
        assert_natural_equality!(symbol, "origin", "upstream");
        let symbol_ref = &symbol;
        assert_natural_equality!(symbol_ref, "origin", "upstream");
        Ok(())
    }

    #[test]
    fn constructors_preserve_slashes_and_non_utf8_bytes() -> TestResult {
        for bytes in [b"team/nested/origin".as_bstr(), b"team/\xff".as_bstr()] {
            let name = Name::symbol(bytes)?;
            assert_eq!(name.as_bstr(), bytes, "validation must not rewrite the name");
            assert_eq!(Name::try_from(bytes.to_owned())?, name);
            assert_eq!(Name::try_from(Cow::Borrowed(bytes))?, name);
            assert_eq!(Name::try_from(Cow::Owned(bytes.to_owned()))?, name);
            assert_eq!(
                BString::from(name).as_bstr(),
                bytes,
                "conversion preserves the original bytes"
            );
        }
        Ok(())
    }

    #[test]
    fn constructors_reject_invalid_tracking_names() {
        for bytes in [
            "",
            ".",
            "/origin",
            "team//origin",
            "bad name",
            "https://example.com/repo",
        ] {
            let bytes = bytes.as_bytes().as_bstr();
            let err = Name::symbol(bytes).expect_err("invalid tracking-ref components must be rejected");
            assert!(err.is_validation(), "{bytes:?}: {err}");
            assert!(Name::try_from(bytes.to_owned()).is_err(), "{bytes:?}");
            assert!(Name::try_from(Cow::Borrowed(bytes)).is_err(), "{bytes:?}");
            assert!(Name::try_from(Cow::Owned(bytes.to_owned())).is_err(), "{bytes:?}");
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deserialization_validates_names() -> TestResult {
        use serde::{
            Deserialize,
            de::value::{BytesDeserializer, Error},
        };

        let deserialize = |bytes| Name::deserialize(BytesDeserializer::<Error>::new(bytes));
        assert_eq!(deserialize(b"team/\xff")?.as_bstr(), b"team/\xff".as_bstr());
        for bytes in [b"".as_slice(), b".", b"team//origin", b"https://example.com/\xff"] {
            assert!(deserialize(bytes).is_err(), "{bytes:?} must not deserialize as a Name");
        }
        Ok(())
    }
}
