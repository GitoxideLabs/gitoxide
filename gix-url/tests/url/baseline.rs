use bstr::ByteSlice;

#[test]
fn parse_and_compare_baseline_urls() {
    let mut passed = 0;
    let mut failed = 0;
    let total = baseline::URLS.len();

    for (url, expected) in baseline::URLS.iter() {
        let result = std::panic::catch_unwind(|| {
            let actual = gix_url::parse(url).expect("url should parse successfully");
            assert_urls_equal(expected, &actual);

            let url_serialized_again = actual.to_bstring();
            let roundtrip = gix_url::parse(url_serialized_again.as_ref()).unwrap_or_else(|e| {
                panic!("roundtrip should work for original '{url}', serialized to '{url_serialized_again}': {e}")
            });
            assert_eq!(roundtrip, actual, "roundtrip failed for url: {url}");
        });

        match result {
            Ok(_) => passed += 1,
            Err(e) => {
                failed += 1;
                let msg = if let Some(&s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    format!("{e:?}")
                };
                println!("FAILED: {url}\n  {msg}");
            }
        }
    }

    println!("\nBaseline tests: {passed}/{total} passed, {failed} failed");

    if failed > 0 {
        panic!("{failed} baseline test(s) failed");
    }
}

fn assert_urls_equal(expected: &baseline::GitDiagUrl<'_>, actual: &gix_url::Url) {
    assert_eq!(
        actual.scheme,
        gix_url::Scheme::from(expected.protocol.to_str().unwrap()),
    );

    match expected.host {
        baseline::GitDiagHost::NonSsh { host_and_port } => match host_and_port {
            Some(expected_host_and_port) if !expected_host_and_port.is_empty() => {
                assert!(actual.host().is_some());

                let mut actual_host_and_port = String::new();
                if let Some(user) = actual.user() {
                    actual_host_and_port.push_str(user);
                    actual_host_and_port.push('@');
                }

                actual_host_and_port.push_str(actual.host().unwrap());

                if let Some(port) = actual.port {
                    actual_host_and_port.push(':');
                    actual_host_and_port.push_str(&port.to_string());
                }

                assert_eq!(actual_host_and_port, expected_host_and_port);
            }
            _ => {
                assert!(actual.host().is_none());
                assert!(actual.port.is_none());
            }
        },
        baseline::GitDiagHost::Ssh { user_and_host, port } => {
            match user_and_host {
                Some(expected_user_and_host) => {
                    assert!(actual.host().is_some());

                    let mut actual_user_and_host = String::new();
                    if let Some(user) = actual.user() {
                        actual_user_and_host.push_str(user);
                        actual_user_and_host.push('@');
                    }
                    actual_user_and_host.push_str(actual.host().unwrap());

                    assert_eq!(actual_user_and_host, expected_user_and_host);
                }
                None => {
                    assert!(actual.host().is_none());
                    assert!(actual.user().is_none());
                }
            }
            assert_eq!(actual.port.map(|p| p.to_string()), port.map(ToString::to_string));
        }
    }

    assert_eq!(actual.path, expected.path.unwrap_or_default());
}

#[allow(clippy::module_inception)]
mod baseline {
    use bstr::{BStr, BString, ByteSlice};
    use std::sync::LazyLock;

    pub enum Kind {
        Unix,
        Windows,
    }

    impl Kind {
        pub const fn new() -> Self {
            if cfg!(windows) {
                Kind::Windows
            } else {
                Kind::Unix
            }
        }

        pub fn extension(&self) -> &'static str {
            match self {
                Kind::Unix => "unix",
                Kind::Windows => "windows",
            }
        }
    }

    static BASELINE: LazyLock<BString> = LazyLock::new(|| {
        let base = gix_testtools::scripted_fixture_read_only("make_baseline.sh").unwrap();
        std::fs::read(base.join(format!("git-baseline.{}", Kind::new().extension())))
            .expect("fixture file exists")
            .into()
    });

    pub static URLS: LazyLock<Vec<(&'static BStr, GitDiagUrl<'static>)>> = LazyLock::new(|| {
        let mut out = Vec::new();

        let blocks = BASELINE
            .split(|c| c == &b';')
            .filter(|block| !block.is_empty())
            .map(ByteSlice::trim);

        for block in blocks {
            let (url, diag_url) = GitDiagUrl::parse(block.as_bstr());
            out.push((url, diag_url));
        }
        out
    });

    #[derive(Debug)]
    pub struct GitDiagUrl<'a> {
        pub protocol: &'a BStr,
        pub host: GitDiagHost<'a>,
        pub path: Option<&'a BStr>,
    }

    impl GitDiagUrl<'_> {
        /// Parses the given string into a [GitDiagUrl] according to the format
        /// specified in [Git's `connect.c`][git_src].
        ///
        /// [git_src]: https://github.com/git/git/blob/bcb6cae2966cc407ca1afc77413b3ef11103c175/connect.c#L1415
        fn parse(diag_url: &BStr) -> (&'_ BStr, GitDiagUrl<'_>) {
            fn null_is_none(input: &BStr) -> Option<&BStr> {
                if input == "NULL" || input == "NONE" {
                    None
                } else {
                    Some(input)
                }
            }
            let mut lines = diag_url.lines().map(ByteSlice::trim);
            let mut next_attr = |name: &str| {
                lines
                    .next()
                    .expect("well-known format")
                    .strip_prefix(format!("Diag: {name}=").as_bytes())
                    .expect("attribute is at the correct location")
                    .as_bstr()
            };

            let url = next_attr("url");
            let protocol = next_attr("protocol");

            let host = if protocol == "ssh" {
                let user_and_host = next_attr("userandhost");
                let port = next_attr("port");
                GitDiagHost::Ssh {
                    user_and_host: null_is_none(user_and_host),
                    port: null_is_none(port),
                }
            } else {
                let host_and_port = next_attr("hostandport");
                GitDiagHost::NonSsh {
                    host_and_port: null_is_none(host_and_port),
                }
            };

            let path = next_attr("path");
            assert!(lines.next().is_none(), "we consume everything");
            (
                url,
                GitDiagUrl {
                    protocol,
                    host,
                    path: null_is_none(path),
                },
            )
        }
    }

    #[derive(Debug)]
    pub enum GitDiagHost<'a> {
        NonSsh {
            host_and_port: Option<&'a BStr>,
        },
        Ssh {
            user_and_host: Option<&'a BStr>,
            port: Option<&'a BStr>,
        },
    }
}
