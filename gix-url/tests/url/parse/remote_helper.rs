use gix_url::Scheme;

use crate::parse::{assert_url, assert_url_roundtrip, url_alternate};

#[test]
fn address_may_contain_a_url() -> crate::Result {
    Ok(assert_url_roundtrip(
        "codecommit::eu-central-1://myaccount@my-repo",
        helper("codecommit", b"eu-central-1://myaccount@my-repo"),
    )?)
}

#[test]
fn address_is_not_interpreted() -> crate::Result {
    for (input, name, address) in [
        ("transport::address", "transport", &b"address"[..]),
        ("myhelper::/abs/path", "myhelper", b"/abs/path"),
        ("hg::https://example.com/repo", "hg", b"https://example.com/repo"),
        // This used to be rejected as a relative URL, but Git passes it to `git-remote-invalid`.
        (
            "invalid:://host.xz/path/to/repo.git/",
            "invalid",
            b"//host.xz/path/to/repo.git/",
        ),
    ] {
        assert_url_roundtrip(input, helper(name, address))?;
    }
    Ok(())
}

#[test]
fn ext_commands_are_normalized_and_distinguishable_from_other_helpers() -> crate::Result {
    assert_eq!(
        Scheme::from("ext"),
        Scheme::Ext,
        "the transport-policy name identifies the command-executing helper"
    );
    assert_eq!(
        Scheme::from("Ext"),
        Scheme::HelperUrl("Ext".into()),
        "transport names are case-sensitive"
    );
    assert_url_roundtrip(
        "ext::ssh -o Foo host %S",
        url_alternate(Scheme::Ext, None, None, None, b"ssh -o Foo host %S"),
    )?;
    let ext_url = assert_url(
        "ext://address/path%20arg",
        url_alternate(Scheme::Ext, None, None, None, b"ext://address/path%20arg"),
    )?;
    assert_eq!(
        ext_url.to_bstring(),
        "ext::ext://address/path%20arg",
        "normalization preserves the exact command Git passes to git-remote-ext"
    );
    assert_eq!(
        gix_url::parse(ext_url.to_bstring())?,
        ext_url,
        "the normalized spelling roundtrips"
    );
    Ok(())
}

#[test]
fn address_may_be_empty_or_contain_more_separators() -> crate::Result {
    assert_url_roundtrip("foo::", helper("foo", b""))?;
    assert_url_roundtrip("foo::a::b", helper("foo", b"a::b"))?;
    Ok(assert_url_roundtrip("a:::b", helper("a", b":b"))?)
}

#[test]
fn address_may_be_an_arbitrary_command_line() -> crate::Result {
    // Helpers can be as flexible as `git-remote-bash` running `eval "$2"`, so nothing about the
    // address may be assumed.
    Ok(assert_url_roundtrip(
        r#"bash::exec git-remote-https "$1" "$(/usr/local/bin/generate-git-https-url)""#,
        helper(
            "bash",
            br#"exec git-remote-https "$1" "$(/usr/local/bin/generate-git-https-url)""#,
        ),
    )?)
}

#[test]
fn a_single_letter_name_is_not_a_dos_drive_letter() -> crate::Result {
    // A DOS drive letter is followed by a single `:`, so these remain remote helpers on all platforms,
    // just like in Git.
    assert_url_roundtrip("c::foo", helper("c", b"foo"))?;
    Ok(assert_url_roundtrip(r"C::\foo", helper("C", br"\foo"))?)
}

#[test]
fn helper_names_shadow_built_in_schemes() -> crate::Result {
    for name in ["ssh", "file", "git", "http", "https"] {
        assert_url_roundtrip(&format!("{name}::address"), helper(name, b"address"))?;
    }
    Ok(())
}

#[test]
fn helper_names_are_case_sensitive() -> crate::Result {
    Ok(assert_url_roundtrip(
        "CodeCommit::address",
        helper("CodeCommit", b"address"),
    )?)
}

#[test]
fn helper_names_may_contain_special_characters_and_start_with_a_digit() -> crate::Result {
    for name in ["a1.2+3-4", "9foo", "foo.bar", "foo-bar", "foo+bar"] {
        assert_url_roundtrip(&format!("{name}::address"), helper(name, b"address"))?;
    }
    Ok(())
}

#[test]
fn addresses_may_contain_arbitrary_bytes() -> crate::Result {
    let url = gix_url::parse(bstr::BStr::new(b"foo::\xff\xfe"))?;
    assert_eq!(url.scheme, Scheme::Helper("foo".into()), "the name is always ASCII");
    assert_eq!(
        url.path,
        b"\xff\xfe".as_slice(),
        "addresses are passed to the helper as-is, so they need not be UTF-8"
    );
    assert_eq!(
        url.to_bstring(),
        b"foo::\xff\xfe".as_slice(),
        "serialization is lossless for non-UTF-8 addresses as well"
    );
    Ok(())
}

#[test]
fn transport_form_and_url_form_are_distinguishable() -> crate::Result {
    let helper_form = assert_url("codecommit::my-repo", helper("codecommit", b"my-repo"))?;
    let url_form = assert_url(
        "codecommit://my-repo",
        crate::parse::url(Scheme::HelperUrl("codecommit".into()), None, "my-repo", None, b""),
    )?;
    assert_ne!(
        helper_form, url_form,
        "both name the same helper program, but only one of them can round-trip to the other spelling"
    );
    Ok(())
}

#[test]
fn from_parts_rejects_invalid_helper_names() {
    assert!(
        gix_url::Url::from_parts(
            Scheme::Helper("foo_bar".into()),
            None,
            None,
            None,
            None,
            "address".into(),
            true
        )
        .is_err(),
        "an invalid helper name must not be reinterpreted as SCP syntax"
    );
}

#[test]
fn helper_form_is_always_retained() -> crate::Result {
    for alt in [false, true] {
        for input in ["9foo::address", "ssh::address", "foo::address", "ext::address"] {
            let url = gix_url::parse(input)?.with_request_alternate_form(alt);
            assert_eq!(
                url.to_bstring(),
                input,
                "URL form would change the address Git passes to the helper"
            );
            gix_url::parse(url.to_bstring())?;
        }
    }
    Ok(())
}

mod not_a_remote_helper {
    use gix_url::Scheme;

    use crate::parse::{assert_url, url_alternate};

    #[test]
    fn names_with_characters_git_does_not_accept() -> crate::Result {
        // `_` and `%` are not part of `[A-Za-z0-9][A-Za-z0-9+.-]*`, so Git falls back to SCP-like syntax.
        for (input, host, path) in [("foo_bar::baz", "foo_bar", &b":baz"[..]), ("f%o::bar", "f%o", b":bar")] {
            assert_url(input, url_alternate(Scheme::Ssh, None, host, None, path))?;
        }
        Ok(())
    }

    #[test]
    fn names_that_do_not_start_with_an_alphanumeric_character() -> crate::Result {
        assert_url(".foo::bar", url_alternate(Scheme::Ssh, None, ".foo", None, b":bar"))?;
        assert!(
            gix_url::parse("::bar").is_err(),
            "an empty helper name is a deviation and remains rejected"
        );
        Ok(())
    }

    #[test]
    fn a_single_colon_does_not_start_an_address() -> crate::Result {
        // Note that the name is longer than one character on purpose, as a single one would be a DOS
        // drive letter on Windows, which is decided after this and is unrelated to remote helpers.
        assert_url("ab:b::c", url_alternate(Scheme::Ssh, None, "ab", None, b"b::c"))?;
        assert_url("host:path", url_alternate(Scheme::Ssh, None, "host", None, b"path"))?;
        Ok(())
    }
}

fn helper(name: &str, address: &[u8]) -> gix_url::Url {
    url_alternate(Scheme::Helper(name.into()), None, None, None, address)
}
