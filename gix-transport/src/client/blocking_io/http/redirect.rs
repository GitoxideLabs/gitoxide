/// The error provided when redirection went beyond what we deem acceptable.
#[derive(Debug, thiserror::Error)]
#[error(
    "Redirect url {redirect_url:?} could not be reconciled with original url {expected_url} as they don't share authority or the same suffix"
)]
pub struct Error {
    redirect_url: String,
    expected_url: String,
}

pub(crate) fn shares_authority_or_upgrades_scheme(redirect_url: &str, original_url: &str) -> bool {
    let Ok(redirect_url) = gix_url::parse(redirect_url.into()) else {
        return false;
    };
    let Ok(original_url) = gix_url::parse(original_url.into()) else {
        return false;
    };

    if redirect_url.host != original_url.host {
        return false;
    }

    if redirect_url.scheme == original_url.scheme {
        return redirect_url.port_or_default() == original_url.port_or_default();
    }

    if original_url.scheme == gix_url::Scheme::Http && redirect_url.scheme == gix_url::Scheme::Https {
        let original_port = original_url.port_or_default();
        let redirect_port = redirect_url.port_or_default();
        return original_port == redirect_port || matches!((original_port, redirect_port), (Some(80), Some(443)));
    }

    false
}

pub(crate) fn base_url(redirect_url: &str, base_url: &str, url: String) -> Result<String, Error> {
    let tail = url
        .strip_prefix(base_url)
        .expect("BUG: caller assures `base_url` is subset of `url`");
    if !shares_authority_or_upgrades_scheme(redirect_url, base_url) {
        return Err(Error {
            redirect_url: redirect_url.into(),
            expected_url: url,
        });
    }
    redirect_url
        .strip_suffix(tail)
        .ok_or_else(|| Error {
            redirect_url: redirect_url.into(),
            expected_url: url,
        })
        .map(ToOwned::to_owned)
}

pub(crate) fn swap_tails(effective_base_url: Option<&str>, base_url: &str, mut url: String) -> String {
    match effective_base_url {
        Some(effective_base) => {
            url.replace_range(..base_url.len(), effective_base);
            url
        }
        None => url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_complete() {
        assert_eq!(
            base_url(
                "https://original/b/info/refs?hi",
                "https://original/a",
                "https://original/a/info/refs?hi".into()
            )
            .unwrap(),
            "https://original/b"
        );
    }

    #[test]
    fn base_url_allows_same_host_scheme_upgrade() {
        assert_eq!(
            base_url(
                "https://original/b/info/refs?hi",
                "http://original/a",
                "http://original/a/info/refs?hi".into()
            )
            .unwrap(),
            "https://original/b"
        );
    }

    #[test]
    fn base_url_rejects_authority_changes() {
        assert!(
            base_url(
                "http://original/b/info/refs?hi",
                "https://original/a",
                "https://original/a/info/refs?hi".into()
            )
            .is_err(),
            "downgrading from https to http must be rejected"
        );
        assert!(
            base_url(
                "https://redirected.org/b/info/refs?hi",
                "https://original/a",
                "https://original/a/info/refs?hi".into()
            )
            .is_err(),
            "changing the host must be rejected"
        );
        assert!(
            base_url(
                "https://original:444/b/info/refs?hi",
                "https://original/a",
                "https://original/a/info/refs?hi".into()
            )
            .is_err(),
            "changing the port on https must be rejected"
        );
        assert!(
            base_url(
                "https://original:444/b/info/refs?hi",
                "http://original/a",
                "http://original/a/info/refs?hi".into()
            )
            .is_err(),
            "upgrading the scheme does not allow changing the port"
        );
    }

    #[test]
    fn shares_authority_or_upgrades_scheme_complete() {
        assert!(
            shares_authority_or_upgrades_scheme("https://original/b/info/refs?hi", "https://original/a"),
            "keeping the same https authority must be allowed"
        );
        assert!(
            shares_authority_or_upgrades_scheme("https://original/b/info/refs?hi", "http://original/a"),
            "upgrading from http to https on the same host must be allowed"
        );
        assert!(
            shares_authority_or_upgrades_scheme("https://original:8080/b/info/refs?hi", "http://original:8080/a"),
            "upgrading from http to https on the same host and port must be allowed"
        );
        assert!(
            !shares_authority_or_upgrades_scheme("http://original/b/info/refs?hi", "https://original/a"),
            "downgrading from https to http must be rejected"
        );
        assert!(
            !shares_authority_or_upgrades_scheme("https://redirected.org/b/info/refs?hi", "https://original/a"),
            "changing the host must be rejected"
        );
        assert!(
            !shares_authority_or_upgrades_scheme("https://original:444/b/info/refs?hi", "https://original/a"),
            "changing the port on https must be rejected"
        );
        assert!(
            !shares_authority_or_upgrades_scheme("https://original:444/b/info/refs?hi", "http://original/a"),
            "upgrading the scheme does not allow changing the port"
        );
    }

    #[test]
    fn swap_tails_complete() {
        assert_eq!(
            swap_tails(None, "not interesting", "used".into()),
            "used",
            "without effective base url, it passes url, no redirect happened yet"
        );
        assert_eq!(
            swap_tails(
                Some("https://redirected.org/b"),
                "https://original/a",
                "https://original/a/info/refs?something".into()
            ),
            "https://redirected.org/b/info/refs?something",
            "the tail stays the same if redirection happened"
        );
    }
}
