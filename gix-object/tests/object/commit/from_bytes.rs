use gix_actor::SignatureRef;
use gix_object::{
    CommitRef, CommitRefIter, WriteTo, bstr::ByteSlice, commit::message::body::TrailerRef, commit::ref_iter::Token,
};
use smallvec::SmallVec;

use crate::{
    commit::{LONG_MESSAGE, MERGE_TAG, SIGNATURE},
    fixture_name, fixture_oid, hex_to_id, linus_signature,
};

#[test]
fn invalid_timestsamp() {
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "invalid-timestamp.txt"), gix_hash::Kind::Sha1)
            .expect("auto-correct invalid timestamp by discarding it (time is still valid UTC)"),
        CommitRef {
            tree: b"7989dfb2ec2f41914611a22fb30bbc2b3849df9a".as_bstr(),
            parents: [b"8845ae683e2688bc619baade49510c17e978518f".as_bstr()].into(),
            author: b"Name <name@example.com> 1312735823 +051800".as_bstr(),
            committer: b"Name <name@example.com> 1312735823 +051800".as_bstr(),
            encoding: None,
            message: b"edit changelog to mention about x_sendfile_header default change".as_bstr(),
            extra_headers: vec![]
        },
        "the offset of the actor is null, leaving the UTC time"
    );
}

#[test]
fn sha256_with_all_fields_and_signature() -> crate::Result {
    let input = b"tree 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
parent 1111111111111111111111111111111111111111111111111111111111111111
parent 2222222222222222222222222222222222222222222222222222222222222222
author Ada Lovelace <ada@example.com> 1710000000 +0000
committer Grace Hopper <grace@example.com> 1710003600 -0230
encoding ISO-8859-1
gpgsig -----BEGIN SSH SIGNATURE-----
 U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgZXhhbXBsZS1zaGEyNTY=
 -----END SSH SIGNATURE-----
mergetag object 3333333333333333333333333333333333333333333333333333333333333333
 type commit
 tag nested-sha256
 tagger Release Bot <release@example.com> 1710007200 +0530
\x20
nested release notes
 -----BEGIN PGP SIGNATURE-----
 nested-signature
 -----END PGP SIGNATURE-----

sha256 subject

sha256 body
";
    let commit = CommitRef::from_bytes(input, gix_hash::Kind::Sha256)?;
    assert_eq!(
        commit.tree,
        b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".as_bstr()
    );
    assert_eq!(commit.parents.len(), 2);
    assert_eq!(commit.encoding, Some(b"ISO-8859-1".as_bstr()));
    assert_eq!(commit.author()?.name, b"Ada Lovelace".as_bstr());
    assert_eq!(commit.committer()?.email, b"grace@example.com".as_bstr());
    assert_eq!(
        commit.extra_headers().pgp_signature(),
        Some(
            b"-----BEGIN SSH SIGNATURE-----
U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgZXhhbXBsZS1zaGEyNTY=
-----END SSH SIGNATURE-----
"
            .as_bstr()
        )
    );
    assert_eq!(commit.extra_headers().mergetags().count(), 1);
    assert_eq!(commit.message, b"sha256 subject\n\nsha256 body\n".as_bstr());

    let tokens = CommitRefIter::from_bytes(input, gix_hash::Kind::Sha256).collect::<Result<Vec<_>, _>>()?;
    assert!(matches!(tokens[0], Token::Tree { ref id } if id.kind() == gix_hash::Kind::Sha256));
    assert_eq!(
        tokens
            .iter()
            .filter(|token| matches!(token, Token::Parent { .. }))
            .count(),
        2
    );
    assert_eq!(
        tokens.last(),
        Some(&Token::Message(b"sha256 subject\n\nsha256 body\n".as_bstr()))
    );
    Ok(())
}

#[test]
fn uppercase_tree_id() -> crate::Result {
    let input = b"tree 7989DFB2EC2F41914611A22FB30BBC2B3849DF9A
author Name <name@example.com> 1312735823 +0518
committer Name <name@example.com> 1312735823 +0518

message";
    let commit = CommitRef::from_bytes(input, gix_hash::Kind::Sha1)?;
    assert_eq!(commit.tree, b"7989DFB2EC2F41914611A22FB30BBC2B3849DF9A".as_bstr());
    assert_eq!(commit.tree(), hex_to_id("7989dfb2ec2f41914611a22fb30bbc2b3849df9a"));
    Ok(())
}

#[test]
fn invalid_email_of_committer() -> crate::Result {
    let actor = gix_actor::SignatureRef {
        name: b"Gregor Hartmann".as_bstr(),
        email: b"gh <Gregor Hartmann<gh@openoffice.org".as_bstr(),
        time: "1282910542 +0200",
    };

    let mut buf = vec![];
    let backing = fixture_name("commit", "invalid-actor.txt");
    let commit = CommitRef::from_bytes(&backing, gix_hash::Kind::Sha1).expect("ignore strangely formed actor format");
    assert_eq!(
        commit,
        CommitRef {
            tree: b"220738fd4199e95a2b244465168366a73ebdf271".as_bstr(),
            parents: [b"209fbe2d632761b30b7b17422914e11b93692833".as_bstr()].into(),
            author: b"Gregor Hartmann<gh <Gregor Hartmann<gh@openoffice.org>> 1282910542 +0200".as_bstr(),
            committer: b"Gregor Hartmann<gh <Gregor Hartmann<gh@openoffice.org>> 1282910542 +0200".as_bstr(),
            encoding: None,
            message: b"build breakers".as_bstr(),
            extra_headers: vec![]
        }
    );
    assert_eq!(commit.author()?, actor);
    assert_eq!(commit.committer()?, actor);

    commit.write_to(&mut buf).expect("we can write invalid actors back");
    assert_eq!(
        CommitRef::from_bytes(&buf, gix_hash::Kind::Sha1).expect("this is the same commit and it can be parsed"),
        commit,
        "round-tripping works"
    );

    Ok(())
}

#[test]
fn unsigned() -> crate::Result {
    let tree = fixture_oid_hex("1b2dfb4ac5e42080b682fc676e9738c94ce6d54d");
    assert_eq!(
        CommitRef::from_bytes(&commit_fixture("unsigned.txt")?, crate::fixture_hash_kind())?,
        CommitRef {
            tree: tree.as_bytes().as_bstr(),
            parents: SmallVec::default(),
            author: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592437401 +0800".as_bstr(),
            committer: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592437401 +0800".as_bstr(),
            encoding: None,
            message: b"without sig".as_bstr(),
            extra_headers: vec![]
        }
    );
    Ok(())
}

#[test]
fn whitespace() -> crate::Result {
    let tree = fixture_oid_hex("9bed6275068a0575243ba8409253e61af81ab2ff");
    let parent = fixture_oid_hex("26b4df046d1776c123ac69d918f5aec247b58cc6");
    assert_eq!(
        CommitRef::from_bytes(&commit_fixture("whitespace.txt")?, crate::fixture_hash_kind())?,
        CommitRef {
            tree: tree.as_bytes().as_bstr(),
            parents: SmallVec::from(vec![parent.as_bytes().as_bstr()]),
            author: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592448450 +0800".as_bstr(),
            committer: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592448450 +0800".as_bstr(),
            encoding: None,
            message: b" nl".as_bstr(), // this one had a \n trailing it, but git seems to trim that
            extra_headers: vec![]
        }
    );
    Ok(())
}

#[test]
fn signed_singleline() -> crate::Result {
    let tree = fixture_oid_hex("00fc39317701176e326974ce44f5bd545a32ec0b");
    let parent = fixture_oid_hex("09d8d3a12e161a7f6afb522dbe8900a9c09bce06");
    assert_eq!(
        CommitRef::from_bytes(&commit_fixture("signed-singleline.txt")?, crate::fixture_hash_kind())?,
        CommitRef {
            tree: tree.as_bytes().as_bstr(),
            parents: SmallVec::from(vec![parent.as_bytes().as_bstr()]),
            author: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592391367 +0800".as_bstr(),
            committer: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592391367 +0800".as_bstr(),
            encoding: None,
            message: b"update tasks\n".as_bstr(),
            extra_headers: vec![(b"gpgsig".as_bstr(), b"magic:signature".as_bstr().into())]
        }
    );
    Ok(())
}

#[test]
fn mergetag() -> crate::Result {
    let fixture = commit_fixture("mergetag.txt")?;
    let tree = fixture_oid_hex("1c61918031bf2c7fab9e17dde3c52a6a9884fcb5");
    let parent_a = fixture_oid_hex("44ebe016df3aad96e3be8f95ec52397728dd7701");
    let parent_b = fixture_oid_hex("8d485da0ddee79d0e6713405694253d401e41b93");
    let expected = CommitRef {
        tree: tree.as_bytes().as_bstr(),
        parents: SmallVec::from(vec![parent_a.as_bytes().as_bstr(), parent_b.as_bytes().as_bstr()]),
        author: b"Linus Torvalds <torvalds@linux-foundation.org> 1591996221 -0700".as_bstr(),
        committer: b"Linus Torvalds <torvalds@linux-foundation.org> 1591996221 -0700".as_bstr(),
        encoding: None,
        message: LONG_MESSAGE.as_bytes().as_bstr(),
        extra_headers: vec![(
            b"mergetag".as_bstr(),
            std::borrow::Cow::Owned(MERGE_TAG.as_bytes().into()),
        )],
    };
    let commit = CommitRef::from_bytes(&fixture, crate::fixture_hash_kind())?;
    assert_eq!(commit, expected);
    assert_eq!(commit.extra_headers().find_all("mergetag").count(), 1);
    assert_eq!(commit.extra_headers().mergetags().count(), 1);
    assert_eq!(commit.author()?, linus_signature("1591996221 -0700"));
    assert_eq!(commit.committer()?, linus_signature("1591996221 -0700"));
    Ok(())
}

#[test]
fn signed() -> crate::Result {
    let tree = fixture_oid_hex("00fc39317701176e326974ce44f5bd545a32ec0b");
    let parent = fixture_oid_hex("09d8d3a12e161a7f6afb522dbe8900a9c09bce06");
    assert_eq!(
        CommitRef::from_bytes(&commit_fixture("signed.txt")?, crate::fixture_hash_kind())?,
        CommitRef {
            tree: tree.as_bytes().as_bstr(),
            parents: SmallVec::from(vec![parent.as_bytes().as_bstr()]),
            author: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592391367 +0800".as_bstr(),
            committer: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592391367 +0800".as_bstr(),
            encoding: None,
            message: b"update tasks\n".as_bstr(),
            extra_headers: vec![(b"gpgsig".as_bstr(), b"-----BEGIN PGP SIGNATURE-----\n\niQEzBAABCAAdFiEEdjYp/sh4j8NRKLX27gKdHl60AwAFAl7p9tgACgkQ7gKdHl60\nAwBpegf+KQciv9AOIN7+yPmowecGxBnSfpKWTDzFxnyGR8dq63SpWT8WEKG5mf3a\nG6iUqpsDWaMHlzihaMKRvgRpZxFRbjnNPFBj6F4RRqfE+5R7k6DRSLUV5PqnsdSH\nuccfIDWi1imhsm7AaP5trwl1t+83U2JhHqPcPVFLMODYwWeO6NLR/JCzGSTQRa8t\nRgaVMKI19O/fge5OT5Ua8D47VKEhsJX0LfmkP5RfZQ8JJvNd40TupqKRdlv0sAzP\nya7NXkSHXCavHNR6kA+KpWxn900UoGK8/IDlwU6MeOkpPVawb3NFMqnc7KJDaC2p\nSMzpuEG8LTrCx2YSpHNLqHyzvQ1CZA==\n=5ITV\n-----END PGP SIGNATURE-----\n".as_bstr().into())]
        }
    );
    Ok(())
}

#[test]
fn signed_with_encoding() -> crate::Result {
    let tree = fixture_oid_hex("1973afa74d87b2bb73fa884aaaa8752aec43ea88");
    let parent = fixture_oid_hex("79c51cc86923e2b8ca0ee5c4eb75e48027133f9a");
    assert_eq!(
        CommitRef::from_bytes(&commit_fixture("signed-with-encoding.txt")?, crate::fixture_hash_kind())?,
        CommitRef {
            tree: tree.as_bytes().as_bstr(),
            parents: SmallVec::from(vec![parent.as_bytes().as_bstr()]),
            author: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592448995 +0800".as_bstr(),
            committer: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592449083 +0800".as_bstr(),
            encoding: Some(b"ISO-8859-1".as_bstr()),
            message: b"encoding & sig".as_bstr(),
            extra_headers: vec![(b"gpgsig".as_bstr(), SIGNATURE.as_bstr().into())]
        }
    );
    Ok(())
}

#[test]
fn with_encoding() -> crate::Result {
    let tree = fixture_oid_hex("4a1c03029e7407c0afe9fc0320b3258e188b115e");
    let parent = fixture_oid_hex("7ca98aad461a5c302cb4c9e3acaaa6053cc67a62");
    assert_eq!(
        CommitRef::from_bytes(&commit_fixture("with-encoding.txt")?, crate::fixture_hash_kind())?,
        CommitRef {
            tree: tree.as_bytes().as_bstr(),
            parents: SmallVec::from(vec![parent.as_bytes().as_bstr()]),
            author: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592438199 +0800".as_bstr(),
            committer: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592438199 +0800".as_bstr(),
            encoding: Some("ISO-8859-1".into()),
            message: b"commit with encoding".as_bstr(),
            extra_headers: vec![]
        }
    );
    Ok(())
}

#[test]
fn pre_epoch() -> crate::Result {
    let tree = fixture_oid_hex("71cdd4015386b764b178005cad4c88966bc9d61a");
    assert_eq!(
        CommitRef::from_bytes(&commit_fixture("pre-epoch.txt")?, crate::fixture_hash_kind())?,
        CommitRef {
            tree: tree.as_bytes().as_bstr(),
            parents: SmallVec::default(),
            author: "Législateur <> -5263834140 +0009".as_bytes().as_bstr(),
            committer: "Législateur <> -5263834140 +0009".as_bytes().as_bstr(),
            encoding: None,
            message: "Version consolidée au 14 mars 1803\n".into(),
            extra_headers: vec![]
        }
    );
    Ok(())
}

#[test]
fn double_dash_special_time_offset() -> crate::Result {
    assert_eq!(
        CommitRef::from_bytes(
            &fixture_name("commit", "double-dash-date-offset.txt"),
            gix_hash::Kind::Sha1
        )?,
        CommitRef {
            tree: b"0a851d7a2a66084ab10516c406a405d147e974ad".as_bstr(),
            parents: SmallVec::from(vec![b"31350f4f0f459485eff2131517e3450cf251f6fa".as_bstr()]),
            author: "name <name@example.com> 1288373970 --700".as_bytes().as_bstr(),
            committer: "name <name@example.com> 1288373970 --700".as_bytes().as_bstr(),
            encoding: None,
            message: "msg\n".into(),
            extra_headers: vec![]
        }
    );
    Ok(())
}

#[test]
fn with_trailer() -> crate::Result {
    let kim = SignatureRef {
        name: "Kim Altintop".into(),
        email: "kim@eagain.st".into(),
        time: "1631514803 +0200",
    };
    let backing = commit_fixture("message-with-footer.txt")?;
    let tree = fixture_oid_hex("25a19c29c5e36884c1ad85d8faf23f1246b7961b");
    let parent = fixture_oid_hex("699ae71105dddfcbb9711ed3a92df09e91a04e90");
    let commit = CommitRef::from_bytes(&backing, crate::fixture_hash_kind())?;
    assert_eq!(
        commit,
        CommitRef {
            tree: tree.as_bytes().as_bstr(),
            parents: SmallVec::from(vec![parent.as_bytes().as_bstr()]),
            author: "Kim Altintop <kim@eagain.st> 1631514803 +0200".as_bytes().as_bstr(),
            committer: "Kim Altintop <kim@eagain.st> 1631514803 +0200".as_bytes().as_bstr(),
            encoding: None,
            message: b"test: use gitoxide for link-git-protocol tests

Showcases the abilities of the `git-repository` crate, and standardises
on using the re-exports through this crate for [stability] reasons
instead of depending directly on the lower-level crates.

[stability]: https://github.com/Byron/gitoxide/blob/main/STABILITY.md

Signed-off-by: Sebastian Thiel <sebastian.thiel@icloud.com>
Signed-off-by: Kim Altintop <kim@eagain.st>"
                .as_bstr(),
            extra_headers: vec![(b"gpgsig".as_bstr(), b"-----BEGIN PGP SIGNATURE-----\n\niHUEABYIAB0WIQSuZwcGWSQItmusNgR5URpSUCnwXQUCYT7xpAAKCRB5URpSUCnw\nXWB3AP9q323HlxnI8MyqszNOeYDwa7Y3yEZaUM2y/IRjz+z4YQEAq0yr1Syt3mrK\nOSFCqL2vDm3uStP+vF31f6FnzayhNg0=\n=Mhpp\n-----END PGP SIGNATURE-----\n".as_bstr().into())]
        }
    );
    assert_eq!(commit.author()?, kim);
    assert_eq!(commit.committer()?, kim);
    let message = commit.message();
    assert_eq!(message.title, "test: use gitoxide for link-git-protocol tests");
    assert_eq!(
        message.body,
        Some(
            "Showcases the abilities of the `git-repository` crate, and standardises
on using the re-exports through this crate for [stability] reasons
instead of depending directly on the lower-level crates.

[stability]: https://github.com/Byron/gitoxide/blob/main/STABILITY.md

Signed-off-by: Sebastian Thiel <sebastian.thiel@icloud.com>
Signed-off-by: Kim Altintop <kim@eagain.st>"
                .into()
        )
    );
    assert_eq!(
        commit.message_summary(),
        message.summary(),
        "both summaries are the same, but the commit one does less parsing"
    );
    let body = message.body().expect("body present");
    assert_eq!(
        body.as_ref(),
        b"Showcases the abilities of the `git-repository` crate, and standardises
on using the re-exports through this crate for [stability] reasons
instead of depending directly on the lower-level crates.

[stability]: https://github.com/Byron/gitoxide/blob/main/STABILITY.md"
            .as_bstr(),
        "body doesn't contain footer"
    );
    assert_eq!(
        body.trailers().collect::<Vec<_>>(),
        vec![
            TrailerRef {
                token: "Signed-off-by".into(),
                value: b"Sebastian Thiel <sebastian.thiel@icloud.com>".as_bstr().into()
            },
            TrailerRef {
                token: "Signed-off-by".into(),
                value: b"Kim Altintop <kim@eagain.st>".as_bstr().into()
            }
        ]
    );
    assert_eq!(
        body.trailers().collect::<Vec<_>>(),
        commit.message_trailers().collect::<Vec<_>>(),
        "messages trailers are accessible on commit level and yield the same result"
    );
    Ok(())
}

#[test]
fn merge() -> crate::Result {
    let tree = fixture_oid_hex("0cf16ce8e229b59a761198975f0c0263229faf82");
    let parent_a = fixture_oid_hex("6a6054db4ce3c1e4e6a37f8c4d7acb63a4d6ad71");
    let parent_b = fixture_oid_hex("c91d592913d47ac4e4a76daf16fd649b276e211e");
    assert_eq!(
        CommitRef::from_bytes(&commit_fixture("merge.txt")?, crate::fixture_hash_kind())?,
        CommitRef {
            tree: tree.as_bytes().as_bstr(),
            parents: SmallVec::from(vec![parent_a.as_bytes().as_bstr(), parent_b.as_bytes().as_bstr()]),
            author: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592454703 +0800".as_bstr(),
            committer: b"Sebastian Thiel <sebastian.thiel@icloud.com> 1592454738 +0800".as_bstr(),
            encoding: Some("ISO-8859-1".into()),
            message: b"Merge branch 'branch'".as_bstr(),
            extra_headers: vec![]
        }
    );
    Ok(())
}

#[test]
fn newline_right_after_signature_multiline_header() -> crate::Result {
    let fixture = commit_fixture("signed-whitespace.txt")?;
    let commit = CommitRef::from_bytes(&fixture, crate::fixture_hash_kind())?;
    let pgp_sig = crate::commit::OTHER_SIGNATURE.as_bstr();
    assert_eq!(commit.extra_headers[0].1.as_ref(), pgp_sig);
    assert_eq!(commit.extra_headers().pgp_signature(), Some(pgp_sig));
    assert_eq!(
        commit.extra_headers().find(gix_object::commit::SIGNATURE_FIELD_NAME),
        Some(pgp_sig)
    );
    assert_eq!(commit.extra_headers().find_pos("gpgsig"), Some(0));
    assert_eq!(commit.extra_headers().find_pos("something else"), None);
    assert!(commit.message.starts_with(b"Rollup"));
    Ok(())
}

#[test]
fn bogus_multi_gpgsig_header() -> crate::Result {
    let fixture = commit_fixture("bogus-gpgsig-lines-in-git.git.txt")?;
    let commit = CommitRef::from_bytes(&fixture, crate::fixture_hash_kind())?;
    let pgp_sig = b"-----BEGIN PGP SIGNATURE-----".as_bstr();
    assert_eq!(commit.extra_headers().pgp_signature(), Some(pgp_sig));
    assert_eq!(
        commit.extra_headers().find_all("gpgsig").count(),
        17,
        "Each signature header line is prefixed with `gpgsig` here, so we parse it as extra header"
    );
    assert!(commit.message.starts_with(b"pretty: %G[?GS] placeholders"));

    let mut buf = Vec::<u8>::new();
    commit.write_to(&mut buf)?;
    let hash_kind = crate::fixture_hash_kind();
    let expected = gix_object::compute_hash(hash_kind, gix_object::Kind::Commit, &fixture)?;
    let actual = gix_object::compute_hash(hash_kind, gix_object::Kind::Commit, &buf)?;
    assert_eq!(actual, expected, "round-tripping works despite the strangeness");
    Ok(())
}

fn commit_fixture(path: &str) -> crate::Result<Vec<u8>> {
    crate::object_fixture(&format!("commit/{path}"))
}

fn fixture_oid_hex(hex: &str) -> String {
    fixture_oid(hex).to_hex().to_string()
}
