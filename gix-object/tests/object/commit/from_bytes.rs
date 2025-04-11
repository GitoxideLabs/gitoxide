use crate::{
    commit::{LONG_MESSAGE, MERGE_TAG, SIGNATURE},
    fixture_name, linus_signature, signature,
};
use gix_actor::SignatureRef;
use gix_object::{bstr::ByteSlice, commit::message::body::TrailerRef, CommitRef};
use smallvec::SmallVec;

#[test]
fn invalid_timestsamp() {
    let actor = gix_actor::SignatureRef {
        name: b"Name".as_bstr(),
        email: b"name@example.com".as_bstr(),
        time: b"1312735823 +051800".as_bstr(),
    };
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "invalid-timestamp.txt"))
            .expect("auto-correct invalid timestamp by discarding it (time is still valid UTC)"),
        CommitRef {
            tree: b"7989dfb2ec2f41914611a22fb30bbc2b3849df9a".as_bstr(),
            parents: [b"8845ae683e2688bc619baade49510c17e978518f".as_bstr()].into(),
            author: actor,
            committer: actor,
            encoding: None,
            message: b"edit changelog to mention about x_sendfile_header default change".as_bstr(),
            extra_headers: vec![]
        },
        "the offset of the actor is null, leaving the UTC time"
    );
}

#[test]
fn invalid_email_of_committer() {
    let actor = gix_actor::SignatureRef {
        name: b"Gregor Hartmann".as_bstr(),
        email: b"gh <Gregor Hartmann<gh@openoffice.org".as_bstr(),
        time: b"1282910542 +0200".as_bstr(),
    };
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "invalid-actor.txt"))
            .expect("ignore strangely formed actor format"),
        CommitRef {
            tree: b"220738fd4199e95a2b244465168366a73ebdf271".as_bstr(),
            parents: [b"209fbe2d632761b30b7b17422914e11b93692833".as_bstr()].into(),
            author: actor,
            committer: actor,
            encoding: None,
            message: b"build breakers".as_bstr(),
            extra_headers: vec![]
        }
    );
}

#[test]
fn unsigned() -> crate::Result {
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "unsigned.txt"))?,
        CommitRef {
            tree: b"1b2dfb4ac5e42080b682fc676e9738c94ce6d54d".as_bstr(),
            parents: SmallVec::default(),
            author: signature(b"1592437401 +0800"),
            committer: signature(b"1592437401 +0800"),
            encoding: None,
            message: b"without sig".as_bstr(),
            extra_headers: vec![]
        }
    );
    Ok(())
}

#[test]
fn whitespace() -> crate::Result {
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "whitespace.txt"))?,
        CommitRef {
            tree: b"9bed6275068a0575243ba8409253e61af81ab2ff".as_bstr(),
            parents: SmallVec::from(vec![b"26b4df046d1776c123ac69d918f5aec247b58cc6".as_bstr()]),
            author: signature(b"1592448450 +0800"),
            committer: signature(b"1592448450 +0800"),
            encoding: None,
            message: b" nl".as_bstr(), // this one had a \n trailing it, but git seems to trim that
            extra_headers: vec![]
        }
    );
    Ok(())
}

#[test]
fn signed_singleline() -> crate::Result {
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "signed-singleline.txt"))?,
        CommitRef {
            tree: b"00fc39317701176e326974ce44f5bd545a32ec0b".as_bstr(),
            parents: SmallVec::from(vec![b"09d8d3a12e161a7f6afb522dbe8900a9c09bce06".as_bstr()]),
            author: signature(b"1592391367 +0800"),
            committer: signature(b"1592391367 +0800"),
            encoding: None,
            message: b"update tasks\n".as_bstr(),
            extra_headers: vec![(b"gpgsig".as_bstr(), b"magic:signature".as_bstr().into())]
        }
    );
    Ok(())
}

#[test]
fn mergetag() -> crate::Result {
    let fixture = fixture_name("commit", "mergetag.txt");
    let commit = CommitRef {
        tree: b"1c61918031bf2c7fab9e17dde3c52a6a9884fcb5".as_bstr(),
        parents: SmallVec::from(vec![
            b"44ebe016df3aad96e3be8f95ec52397728dd7701".as_bstr(),
            b"8d485da0ddee79d0e6713405694253d401e41b93".as_bstr(),
        ]),
        author: linus_signature(b"1591996221 -0700"),
        committer: linus_signature(b"1591996221 -0700"),
        encoding: None,
        message: LONG_MESSAGE.as_bytes().as_bstr(),
        extra_headers: vec![(
            b"mergetag".as_bstr(),
            std::borrow::Cow::Owned(MERGE_TAG.as_bytes().into()),
        )],
    };
    assert_eq!(CommitRef::from_bytes(&fixture)?, commit);
    assert_eq!(commit.extra_headers().find_all("mergetag").count(), 1);
    assert_eq!(commit.extra_headers().mergetags().count(), 1);
    Ok(())
}

#[test]
fn signed() -> crate::Result {
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "signed.txt"))?,
        CommitRef {
            tree: b"00fc39317701176e326974ce44f5bd545a32ec0b".as_bstr(),
            parents: SmallVec::from(vec![b"09d8d3a12e161a7f6afb522dbe8900a9c09bce06".as_bstr()]),
            author: signature(b"1592391367 +0800"),
            committer: signature(b"1592391367 +0800"),
            encoding: None,
            message: b"update tasks\n".as_bstr(),
            extra_headers: vec![(b"gpgsig".as_bstr(), b"-----BEGIN PGP SIGNATURE-----\n\niQEzBAABCAAdFiEEdjYp/sh4j8NRKLX27gKdHl60AwAFAl7p9tgACgkQ7gKdHl60\nAwBpegf+KQciv9AOIN7+yPmowecGxBnSfpKWTDzFxnyGR8dq63SpWT8WEKG5mf3a\nG6iUqpsDWaMHlzihaMKRvgRpZxFRbjnNPFBj6F4RRqfE+5R7k6DRSLUV5PqnsdSH\nuccfIDWi1imhsm7AaP5trwl1t+83U2JhHqPcPVFLMODYwWeO6NLR/JCzGSTQRa8t\nRgaVMKI19O/fge5OT5Ua8D47VKEhsJX0LfmkP5RfZQ8JJvNd40TupqKRdlv0sAzP\nya7NXkSHXCavHNR6kA+KpWxn900UoGK8/IDlwU6MeOkpPVawb3NFMqnc7KJDaC2p\nSMzpuEG8LTrCx2YSpHNLqHyzvQ1CZA==\n=5ITV\n-----END PGP SIGNATURE-----\n".as_bstr().into())]
        }
    );
    Ok(())
}

#[test]
fn signed_with_encoding() -> crate::Result {
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "signed-with-encoding.txt"))?,
        CommitRef {
            tree: b"1973afa74d87b2bb73fa884aaaa8752aec43ea88".as_bstr(),
            parents: SmallVec::from(vec![b"79c51cc86923e2b8ca0ee5c4eb75e48027133f9a".as_bstr()]),
            author: signature(b"1592448995 +0800"),
            committer: signature(b"1592449083 +0800"),
            encoding: Some(b"ISO-8859-1".as_bstr()),
            message: b"encoding & sig".as_bstr(),
            extra_headers: vec![(b"gpgsig".as_bstr(), SIGNATURE.as_bstr().into())]
        }
    );
    Ok(())
}

#[test]
fn with_encoding() -> crate::Result {
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "with-encoding.txt"))?,
        CommitRef {
            tree: b"4a1c03029e7407c0afe9fc0320b3258e188b115e".as_bstr(),
            parents: SmallVec::from(vec![b"7ca98aad461a5c302cb4c9e3acaaa6053cc67a62".as_bstr()]),
            author: signature(b"1592438199 +0800"),
            committer: signature(b"1592438199 +0800"),
            encoding: Some("ISO-8859-1".into()),
            message: b"commit with encoding".as_bstr(),
            extra_headers: vec![]
        }
    );
    Ok(())
}

#[test]
fn pre_epoch() -> crate::Result {
    let signature = || SignatureRef {
        name: "Législateur".into(),
        email: "".into(),
        time: b"-5263834140 +0009".as_bstr(),
    };
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "pre-epoch.txt"))?,
        CommitRef {
            tree: b"71cdd4015386b764b178005cad4c88966bc9d61a".as_bstr(),
            parents: SmallVec::default(),
            author: signature(),
            committer: signature(),
            encoding: None,
            message: "Version consolidée au 14 mars 1803\n".into(),
            extra_headers: vec![]
        }
    );
    Ok(())
}

#[test]
fn double_dash_special_time_offset() -> crate::Result {
    let signature = || SignatureRef {
        name: "name".into(),
        email: "name@example.com".into(),
        time: "1288373970 --700".into(),
    };
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "double-dash-date-offset.txt"))?,
        CommitRef {
            tree: b"0a851d7a2a66084ab10516c406a405d147e974ad".as_bstr(),
            parents: SmallVec::from(vec![b"31350f4f0f459485eff2131517e3450cf251f6fa".as_bstr()]),
            author: signature(),
            committer: signature(),
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
        time: "1631514803 +0200".into(),
    };
    let backing = fixture_name("commit", "message-with-footer.txt");
    let commit = CommitRef::from_bytes(&backing)?;
    assert_eq!(
        commit,
        CommitRef {
            tree: b"25a19c29c5e36884c1ad85d8faf23f1246b7961b".as_bstr(),
            parents: SmallVec::from(vec![b"699ae71105dddfcbb9711ed3a92df09e91a04e90".as_bstr()]),
            author: kim,
            committer: kim,
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
                value: "Sebastian Thiel <sebastian.thiel@icloud.com>".into()
            },
            TrailerRef {
                token: "Signed-off-by".into(),
                value: "Kim Altintop <kim@eagain.st>".into()
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
    assert_eq!(
        CommitRef::from_bytes(&fixture_name("commit", "merge.txt"))?,
        CommitRef {
            tree: b"0cf16ce8e229b59a761198975f0c0263229faf82".as_bstr(),
            parents: SmallVec::from(vec![
                b"6a6054db4ce3c1e4e6a37f8c4d7acb63a4d6ad71".as_bstr(),
                b"c91d592913d47ac4e4a76daf16fd649b276e211e".as_bstr()
            ]),
            author: signature(b"1592454703 +0800"),
            committer: signature(b"1592454738 +0800"),
            encoding: Some("ISO-8859-1".into()),
            message: b"Merge branch 'branch'".as_bstr(),
            extra_headers: vec![]
        }
    );
    Ok(())
}

#[test]
fn newline_right_after_signature_multiline_header() -> crate::Result {
    let fixture = fixture_name("commit", "signed-whitespace.txt");
    let commit = CommitRef::from_bytes(&fixture)?;
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
