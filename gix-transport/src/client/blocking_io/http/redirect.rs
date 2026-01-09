/// The error provided when redirection went beyond what we deem acceptable.
pub type Error = crate::Error;

pub(crate) fn base_url(redirect_url: &str, base_url: &str, url: String) -> Result<String, Error> {
    let tail = url
        .strip_prefix(base_url)
        .expect("BUG: caller assures `base_url` is subset of `url`");
    redirect_url.strip_suffix(tail).ok_or_else(|| crate::Error::Redirect {
        redirect_url: redirect_url.into(),
        expected_url: url,
    }).map(ToOwned::to_owned)
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
                "https://redirected.org/b/info/refs?hi",
                "https://original/a",
                "https://original/a/info/refs?hi".into()
            )
            .unwrap(),
            "https://redirected.org/b"
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
