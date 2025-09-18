use bstr::{BString, ByteSlice};
use gix_url::{parse, Url};
#[test]
fn scp_like_keeps_username_literal_no_percent_encoding() {
    // scp-like keeps username literal in alternative form
    let cases = [
        ("john+doe@github.com:org/repo.git", "john+doe"),
        ("user.name@host:repo", "user.name"),
        ("u_ser-123@host:org/repo", "u_ser-123"),
    ];

    for (input, expected_user) in cases {
        let url: Url = parse(input.as_bytes().as_bstr()).expect("parse scp-like");
        assert_eq!(url.user(), Some(expected_user), "user() changed for {input}");
        let round = url.to_bstring();
        assert_eq!(round, BString::from(input), "roundtrip mismatch for {input}");
    }
}

#[test]
fn ssh_scheme_behavior_unchanged() {
    // ssh:// round-trips with canonical form percent encoding
    let input = "ssh://john+doe@github.com/org/repo.git";
    let expected_output = "ssh://john%2Bdoe@github.com/org/repo.git";
    let url = gix_url::parse(input.as_bytes().as_bstr()).expect("parse ssh://");
    assert_eq!(
        url.to_bstring().as_bstr(),
        expected_output.as_bytes().as_bstr(),
        "ssh:// canonical form should percent-encode special chars"
    );
}
