use gix_object::{Kind, TagRef, TagRefIter, bstr::ByteSlice};

use crate::fixture_name;

const PGP_BEGIN_NOT_AT_LINE_START: &[u8] = b"object ffa700b4aca13b80cb6b98a078e7c96804f8e0ec
type commit
tag pgp-marker-in-message

message text
not-a-signature -----BEGIN PGP SIGNATURE-----
body
-----END PGP SIGNATURE-----";
const PGP_BEGIN_NOT_AT_LINE_START_MESSAGE: &[u8] = b"message text
not-a-signature -----BEGIN PGP SIGNATURE-----
body
-----END PGP SIGNATURE-----";
const PGP_SIGNATURE_WITH_TRAILING_TEXT: &[u8] = b"object ffa700b4aca13b80cb6b98a078e7c96804f8e0ec
type commit
tag pgp-signature-with-trailing-text

message text
-----BEGIN PGP SIGNATURE-----
body
-----END PGP SIGNATURE-----
trailing text";
const PGP_SIGNATURE_WITH_TRAILING_TEXT_SIGNATURE: &[u8] = b"-----BEGIN PGP SIGNATURE-----
body
-----END PGP SIGNATURE-----
trailing text";
const PGP_SIGNATURE_WITHOUT_END_MARKER: &[u8] = b"object ffa700b4aca13b80cb6b98a078e7c96804f8e0ec
type commit
tag pgp-signature-without-end-marker

message text
-----BEGIN PGP SIGNATURE-----
body";
const PGP_SIGNATURE_WITHOUT_END_MARKER_SIGNATURE: &[u8] = b"-----BEGIN PGP SIGNATURE-----
body";
const PGP_SIGNATURE_AT_BODY_START: &[u8] = b"object ffa700b4aca13b80cb6b98a078e7c96804f8e0ec
type commit
tag pgp-signature-at-body-start

-----BEGIN PGP SIGNATURE-----
body";
const PGP_SIGNATURE_AT_BODY_START_SIGNATURE: &[u8] = b"-----BEGIN PGP SIGNATURE-----
body";

#[test]
fn sha256_with_all_fields_and_signature() -> crate::Result {
    let input = b"object abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789
type commit
tag v2.0.0-sha256
tagger Release Bot <release@example.com> 1710007200 +0530

Release v2.0.0

- ship sha256 object support
- include annotated tag signatures
-----BEGIN PGP SIGNATURE-----
sha256-tag-signature
-----END PGP SIGNATURE-----
";
    let tag = TagRef::from_bytes(input, gix_hash::Kind::Sha256)?;
    assert_eq!(
        tag.target,
        b"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".as_bstr()
    );
    assert_eq!(tag.target().kind(), gix_hash::Kind::Sha256);
    assert_eq!(tag.target_kind, Kind::Commit);
    assert_eq!(tag.name, b"v2.0.0-sha256".as_bstr());
    assert_eq!(tag.tagger()?.expect("tagger").name, b"Release Bot".as_bstr());
    assert_eq!(
        tag.message,
        b"Release v2.0.0

- ship sha256 object support
- include annotated tag signatures"
            .as_bstr()
    );
    assert_eq!(
        tag.pgp_signature,
        Some(
            b"-----BEGIN PGP SIGNATURE-----
sha256-tag-signature
-----END PGP SIGNATURE-----
"
            .as_bstr()
        )
    );

    let tokens = TagRefIter::from_bytes(input, gix_hash::Kind::Sha256).collect::<Result<Vec<_>, _>>()?;
    assert!(matches!(
        tokens.first(),
        Some(gix_object::tag::ref_iter::Token::Target { id }) if id.kind() == gix_hash::Kind::Sha256
    ));
    assert!(matches!(
        tokens.last(),
        Some(gix_object::tag::ref_iter::Token::Body {
            pgp_signature: Some(_),
            ..
        })
    ));
    Ok(())
}

mod method {
    use bstr::ByteSlice;
    use gix_object::TagRef;
    use pretty_assertions::assert_eq;

    use crate::{fixture_name, hex_to_id, signature};

    #[test]
    fn target() -> crate::Result {
        let fixture = fixture_name("tag", "signed.txt");
        let tag_ref = TagRef::from_bytes(&fixture, gix_hash::Kind::Sha1)?;
        assert_eq!(tag_ref.target(), hex_to_id("ffa700b4aca13b80cb6b98a078e7c96804f8e0ec"));
        assert_eq!(tag_ref.target, "ffa700b4aca13b80cb6b98a078e7c96804f8e0ec".as_bytes());

        let gix_object::Tag {
            target,
            target_kind,
            name,
            tagger,
            message,
            pgp_signature,
        } = tag_ref.into_owned()?;
        assert_eq!(target.to_string(), tag_ref.target);
        assert_eq!(target_kind, tag_ref.target_kind);
        assert_eq!(name, tag_ref.name);
        let expected_tagger = tag_ref.tagger()?.map(Into::into);
        assert_eq!(tagger, expected_tagger);
        assert_eq!(message, tag_ref.message);
        assert_eq!(pgp_signature.as_ref().map(|s| s.as_bstr()), tag_ref.pgp_signature);
        Ok(())
    }

    #[test]
    fn tagger_trims_signature() -> crate::Result {
        let fixture = fixture_name("tag", "tagger-with-whitespace.txt");
        let tag = TagRef::from_bytes(&fixture, gix_hash::Kind::Sha1)?;
        std::assert_eq!(tag.tagger()?, Some(signature("1592381636 +0800")));
        Ok(())
    }
}

mod iter {
    use gix_object::{Kind, TagRefIter, bstr::ByteSlice, tag::ref_iter::Token};

    use crate::{fixture_name, hex_to_id, signature};

    #[test]
    fn empty() -> crate::Result {
        let tag = fixture_name("tag", "empty.txt");
        let tag_iter = TagRefIter::from_bytes(&tag, gix_hash::Kind::Sha1);
        let target_id = hex_to_id("01dd4e2a978a9f5bd773dae6da7aa4a5ac1cdbbc");
        let tagger = Some(signature("1592381636 +0800"));
        assert_eq!(
            tag_iter.collect::<Result<Vec<_>, _>>()?,
            vec![
                Token::Target { id: target_id },
                Token::TargetKind(Kind::Commit),
                Token::Name(b"empty".as_bstr()),
                Token::Tagger(tagger),
                Token::Body {
                    message: b"\n".as_bstr(),
                    pgp_signature: None,
                }
            ]
        );
        assert_eq!(tag_iter.target_id()?, target_id);
        assert_eq!(tag_iter.tagger()?, tagger);
        Ok(())
    }

    #[test]
    fn no_tagger() -> crate::Result {
        assert_eq!(
            TagRefIter::from_bytes(&fixture_name("tag", "no-tagger.txt"), gix_hash::Kind::Sha1)
                .collect::<Result<Vec<_>, _>>()?,
            vec![
                Token::Target {
                    id: hex_to_id("c39ae07f393806ccf406ef966e9a15afc43cc36a")
                },
                Token::TargetKind(Kind::Tree),
                Token::Name(b"v2.6.11-tree".as_bstr()),
                Token::Tagger(None),
                Token::Body {
                    message: b"This is the 2.6.11 tree object.

NOTE! There's no commit for this, since it happened before I started with git.
Eventually we'll import some sort of history, and that should tie this tree
object up to a real commit. In the meantime, this acts as an anchor point for
doing diffs etc under git."
                        .as_bstr(),
                    pgp_signature: Some(
                        b"-----BEGIN PGP SIGNATURE-----
Version: GnuPG v1.2.4 (GNU/Linux)

iD8DBQBCeV/eF3YsRnbiHLsRAl+SAKCVp8lVXwpUhMEvy8N5jVBd16UCmACeOtP6
KLMHist5yj0sw1E4hDTyQa0=
=/bIK
-----END PGP SIGNATURE-----
"
                        .as_bstr()
                    )
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn whitespace() -> crate::Result {
        assert_eq!(
            TagRefIter::from_bytes(&fixture_name("tag", "whitespace.txt"), gix_hash::Kind::Sha1)
                .collect::<Result<Vec<_>, _>>()?,
            vec![
                Token::Target {
                    id: hex_to_id("01dd4e2a978a9f5bd773dae6da7aa4a5ac1cdbbc")
                },
                Token::TargetKind(Kind::Commit),
                Token::Name(b"whitespace".as_bstr()),
                Token::Tagger(Some(signature("1592382888 +0800"))),
                Token::Body {
                    message: b" \ttab\nnewline\n\nlast-with-trailer\n".as_bstr(),
                    pgp_signature: None
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn pgp_begin_marker_not_at_line_start_is_message() -> crate::Result {
        assert_eq!(
            TagRefIter::from_bytes(super::PGP_BEGIN_NOT_AT_LINE_START, gix_hash::Kind::Sha1)
                .collect::<Result<Vec<_>, _>>()?,
            vec![
                Token::Target {
                    id: hex_to_id("ffa700b4aca13b80cb6b98a078e7c96804f8e0ec")
                },
                Token::TargetKind(Kind::Commit),
                Token::Name(b"pgp-marker-in-message".as_bstr()),
                Token::Tagger(None),
                Token::Body {
                    message: super::PGP_BEGIN_NOT_AT_LINE_START_MESSAGE.as_bstr(),
                    pgp_signature: None
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn error_handling() -> crate::Result {
        let data = fixture_name("tag", "empty.txt");
        let iter = TagRefIter::from_bytes(&data[..data.len() / 3], gix_hash::Kind::Sha1);
        let tokens = iter.collect::<Vec<_>>();
        assert!(
            tokens.last().expect("at least the errored token").is_err(),
            "errors are propagated and none is returned from that point on"
        );
        Ok(())
    }
}

#[test]
fn invalid() {
    let fixture = fixture_name("tag", "whitespace.txt");
    let partial_tag = &fixture[..fixture.len() / 2];
    assert!(TagRef::from_bytes(partial_tag, gix_hash::Kind::Sha1).is_err());
    assert_eq!(
        TagRefIter::from_bytes(partial_tag, gix_hash::Kind::Sha1)
            .take_while(Result::is_ok)
            .count(),
        3,
        "we can decode some fields before failing"
    );
}

#[test]
fn uppercase_target_id() -> crate::Result {
    let input = b"object FFA700B4ACA13B80CB6B98A078E7C96804F8E0EC
type commit
tag uppercase-target

message";
    let tag = TagRef::from_bytes(input, gix_hash::Kind::Sha1)?;
    assert_eq!(tag.target, b"FFA700B4ACA13B80CB6B98A078E7C96804F8E0EC".as_bstr());
    assert_eq!(
        tag.target(),
        crate::hex_to_id("ffa700b4aca13b80cb6b98a078e7c96804f8e0ec")
    );
    Ok(())
}

#[test]
fn invalid_target_id_length() {
    let input = b"object 00000066666666666684666666666666666299297\ntype commit\ntag bad\n";

    assert!(TagRef::from_bytes(input, gix_hash::Kind::Sha1).is_err());
    assert!(
        TagRefIter::from_bytes(input, gix_hash::Kind::Sha1)
            .next()
            .expect("a decoding error is returned for the first token")
            .is_err()
    );
}

mod from_bytes {
    use gix_object::{Kind, TagRef, WriteTo, bstr::ByteSlice};

    use crate::{fixture_name, fixture_oid, object_fixture};

    #[test]
    fn signed() -> crate::Result {
        let fixture = tag_fixture("signed.txt")?;
        let target = fixture_oid_hex("ffa700b4aca13b80cb6b98a078e7c96804f8e0ec");
        assert_eq!(
            TagRef::from_bytes(&fixture, crate::fixture_hash_kind())?,
            TagRef {
                target: target.as_bytes().as_bstr(),
                name: b"1.0.0".as_bstr(),
                target_kind: Kind::Commit,
                message: b"for the signature".as_bstr(),
                pgp_signature: Some(
                    b"-----BEGIN PGP SIGNATURE-----
Comment: GPGTools - https://gpgtools.org

iQIzBAABCgAdFiEEw7xSvXbiwjusbsBqZl+Z+p2ZlmwFAlsapyYACgkQZl+Z+p2Z
lmy6Ug/+KzvzqiNpzz1bMVVAzp8NCbiEO3QGYPyeQc521lBwpaTrRYR+oHJY15r3
OdL5WDysTpjN8N5FNyfmvzkuPdTkK3JlYmO7VRjdA2xu/B6vIZLaOfAowFrhMvKo
8eoqwGcAP3rC5TuWEgzq2qhbjS4JXFLd4NLjWEFqT2Y2UKm+g8TeGOsa/0pF4Nq5
xeW4qCYR0WcQLFedbpkKHxag2GfaXKvzNNJdqYhVQssNa6BeSmsfDvlWYNe617wV
NvsR/zJT0wHb5SSH+h6QmwA7LQIQF//83Vc3aF7kv9D54r3ibXW5TjZ3WoeTUZO7
kefkzJ12EYDCFLPhHvXPog518nO8Ot46dX+okrF0/B4N3RFTvjKr7VAGTzv2D/Dg
DrD531S2F71b+JIRh641eeP7bjWFQi3tWLtrEOtjjsKPJfYRMKpYFnAO4UUJ6Rck
Z5fFXEUCO8d5WT56jzKDjmVoY01lA87O1YsP/J+zQAlc9v1k6jqeQ53LZNgTN+ue
5fJuSPT3T43pSOD1VQSr3aZ2Anc4Qu7K8uX9lkpxF9Sc0tDbeCosFLZMWNVp6m+e
cjHJZXWmV4CcRfmLsXzU8s2cR9A0DBvOxhPD1TlKC2JhBFXigjuL9U4Rbq9tdegB
2n8f2douw6624Tn/6Lm4a7AoxmU+CMiYagDxDL3RuZ8CAfh3bn0=
=aIns
-----END PGP SIGNATURE-----"
                        .as_bstr(),
                ),
                tagger: Some(b"Sebastian Thiel <byronimo@gmail.com> 1528473343 +0230".as_bstr()),
            }
        );
        Ok(())
    }

    #[test]
    fn empty() -> crate::Result {
        let fixture = tag_fixture("empty.txt")?;
        let target = fixture_oid_hex("01dd4e2a978a9f5bd773dae6da7aa4a5ac1cdbbc");
        let tag_ref = TagRef::from_bytes(&fixture, crate::fixture_hash_kind())?;
        assert_eq!(
            tag_ref,
            TagRef {
                target: target.as_bytes().as_bstr(),
                name: b"empty".as_bstr(),
                target_kind: Kind::Commit,
                message: b"\n".as_bstr(),
                tagger: Some(b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592381636 +0800".as_bstr()),
                pgp_signature: None
            }
        );
        assert_eq!(tag_ref.size(), fixture.len() as u64);
        Ok(())
    }

    #[test]
    fn empty_missing_nl() -> crate::Result {
        let fixture = tag_fixture("empty_missing_nl.txt")?;
        let target = fixture_oid_hex("01dd4e2a978a9f5bd773dae6da7aa4a5ac1cdbbc");
        let tag_ref = TagRef::from_bytes(&fixture, crate::fixture_hash_kind())?;
        assert_eq!(
            tag_ref,
            TagRef {
                target: target.as_bytes().as_bstr(),
                name: b"empty".as_bstr(),
                target_kind: Kind::Commit,
                message: b"".as_bstr(),
                tagger: Some(b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592381636 +0800".as_bstr()),
                pgp_signature: None
            }
        );
        assert_eq!(tag_ref.size(), fixture.len() as u64);
        Ok(())
    }

    #[test]
    fn with_newlines() -> crate::Result {
        let target = fixture_oid_hex("ebdf205038b66108c0331aa590388431427493b7");
        assert_eq!(
            TagRef::from_bytes(&tag_fixture("with-newlines.txt")?, crate::fixture_hash_kind())?,
            TagRef {
                target: target.as_bytes().as_bstr(),
                name: b"baz".as_bstr(),
                target_kind: Kind::Commit,
                message: b"hello\n\nworld".as_bstr(),
                tagger: Some(b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592311808 +0800".as_bstr()),
                pgp_signature: None
            }
        );
        Ok(())
    }

    #[test]
    fn no_tagger() -> crate::Result {
        let target = fixture_oid_hex("c39ae07f393806ccf406ef966e9a15afc43cc36a");
        assert_eq!(
            TagRef::from_bytes(&tag_fixture("no-tagger.txt")?, crate::fixture_hash_kind())?,
            TagRef {
                target: target.as_bytes().as_bstr(),
                name: b"v2.6.11-tree".as_bstr(),
                target_kind: Kind::Tree,
                message: b"This is the 2.6.11 tree object.

NOTE! There's no commit for this, since it happened before I started with git.
Eventually we'll import some sort of history, and that should tie this tree
object up to a real commit. In the meantime, this acts as an anchor point for
doing diffs etc under git."
                    .as_bstr(),
                tagger: None,
                pgp_signature: Some(
                    b"-----BEGIN PGP SIGNATURE-----
Version: GnuPG v1.2.4 (GNU/Linux)

iD8DBQBCeV/eF3YsRnbiHLsRAl+SAKCVp8lVXwpUhMEvy8N5jVBd16UCmACeOtP6
KLMHist5yj0sw1E4hDTyQa0=
=/bIK
-----END PGP SIGNATURE-----
"
                    .as_bstr()
                )
            }
        );
        Ok(())
    }

    #[test]
    fn pgp_begin_marker_not_at_line_start_is_message() -> crate::Result {
        let tag = TagRef::from_bytes(super::PGP_BEGIN_NOT_AT_LINE_START, gix_hash::Kind::Sha1)?;
        assert_eq!(tag.message, super::PGP_BEGIN_NOT_AT_LINE_START_MESSAGE.as_bstr());
        assert_eq!(tag.pgp_signature, None, "it doesn't parse this as PGP signature");
        assert_roundtrip(super::PGP_BEGIN_NOT_AT_LINE_START)?;
        Ok(())
    }

    #[test]
    fn trailing_text_after_pgp_end_marker_is_signature() -> crate::Result {
        let tag = TagRef::from_bytes(super::PGP_SIGNATURE_WITH_TRAILING_TEXT, gix_hash::Kind::Sha1)?;
        assert_eq!(tag.message, b"message text".as_bstr());
        assert_eq!(
            tag.pgp_signature,
            Some(super::PGP_SIGNATURE_WITH_TRAILING_TEXT_SIGNATURE.as_bstr())
        );
        assert_roundtrip(super::PGP_SIGNATURE_WITH_TRAILING_TEXT)?;
        Ok(())
    }

    #[test]
    fn pgp_begin_marker_without_end_marker_starts_signature() -> crate::Result {
        let tag = TagRef::from_bytes(super::PGP_SIGNATURE_WITHOUT_END_MARKER, gix_hash::Kind::Sha1)?;
        assert_eq!(tag.message, b"message text".as_bstr());
        assert_eq!(
            tag.pgp_signature,
            Some(super::PGP_SIGNATURE_WITHOUT_END_MARKER_SIGNATURE.as_bstr())
        );
        assert_roundtrip(super::PGP_SIGNATURE_WITHOUT_END_MARKER)?;
        Ok(())
    }

    #[test]
    fn pgp_begin_marker_at_body_start_is_signature() -> crate::Result {
        let tag = TagRef::from_bytes(super::PGP_SIGNATURE_AT_BODY_START, gix_hash::Kind::Sha1)?;
        assert_eq!(tag.message, b"".as_bstr());
        assert_eq!(
            tag.pgp_signature,
            Some(super::PGP_SIGNATURE_AT_BODY_START_SIGNATURE.as_bstr())
        );
        assert_roundtrip(super::PGP_SIGNATURE_AT_BODY_START)?;
        Ok(())
    }

    #[test]
    fn whitespace() -> crate::Result {
        let target = fixture_oid_hex("01dd4e2a978a9f5bd773dae6da7aa4a5ac1cdbbc");
        assert_eq!(
            TagRef::from_bytes(&tag_fixture("whitespace.txt")?, crate::fixture_hash_kind())?,
            TagRef {
                target: target.as_bytes().as_bstr(),
                name: b"whitespace".as_bstr(),
                target_kind: Kind::Commit,
                message: b" \ttab\nnewline\n\nlast-with-trailer\n".as_bstr(),
                tagger: Some(b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592382888 +0800".as_bstr()),
                pgp_signature: None
            }
        );
        Ok(())
    }

    #[test]
    fn tagger_without_timestamp() -> crate::Result {
        assert_eq!(
            TagRef::from_bytes(
                &fixture_name("tag", "tagger-without-timestamp.txt"),
                gix_hash::Kind::Sha1
            )?,
            TagRef {
                target: b"4fcd840c4935e4c7a5ea3552710a0f26b9178c24".as_bstr(),
                name: b"ChangeLog".as_bstr(),
                target_kind: Kind::Commit,
                message: b"".as_bstr(),
                tagger: Some(b"shemminger <shemminger>".as_bstr()),
                pgp_signature: None
            }
        );
        Ok(())
    }

    fn assert_roundtrip(input: &[u8]) -> crate::Result {
        let tag = TagRef::from_bytes(input, gix_hash::Kind::Sha1)?;
        let mut out = Vec::new();
        tag.write_to(&mut out)?;
        assert_eq!(out, input);
        Ok(())
    }

    fn tag_fixture(path: &str) -> crate::Result<Vec<u8>> {
        object_fixture(&format!("tag/{path}"))
    }

    fn fixture_oid_hex(hex: &str) -> String {
        fixture_oid(hex).to_hex().to_string()
    }
}
