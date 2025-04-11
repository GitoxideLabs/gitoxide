use gix_object::{bstr::ByteSlice, commit::ref_iter::Token, CommitRefIter};

use crate::{
    commit::{LONG_MESSAGE, MERGE_TAG, SIGNATURE},
    fixture_name, hex_to_id, linus_signature, signature,
};

#[test]
fn newline_right_after_signature_multiline_header() -> crate::Result {
    let data = fixture_name("commit", "signed-whitespace.txt");
    let tokens = CommitRefIter::from_bytes(&data).collect::<Result<Vec<_>, _>>()?;
    assert_eq!(tokens.len(), 7, "mainly a parsing exercise");
    match tokens.last().expect("there are tokens") {
        Token::Message(msg) => {
            assert!(msg.starts_with(b"Rollup"));
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[test]
fn signed_with_encoding() -> crate::Result {
    let input = fixture_name("commit", "signed-with-encoding.txt");
    let iter = CommitRefIter::from_bytes(&input);
    assert_eq!(
        iter.collect::<Result<Vec<_>, _>>()?,
        vec![
            Token::Tree {
                id: hex_to_id("1973afa74d87b2bb73fa884aaaa8752aec43ea88")
            },
            Token::Parent {
                id: hex_to_id("79c51cc86923e2b8ca0ee5c4eb75e48027133f9a")
            },
            Token::Author {
                signature: signature(b"1592448995 +0800")
            },
            Token::Committer {
                signature: signature(b"1592449083 +0800")
            },
            Token::Encoding(b"ISO-8859-1".as_bstr()),
            Token::ExtraHeader((b"gpgsig".as_bstr(), SIGNATURE.as_bytes().as_bstr().into())),
            Token::Message(b"encoding & sig".as_bstr()),
        ]
    );

    assert_eq!(iter.author().ok(), Some(signature(b"1592448995 +0800")));
    assert_eq!(iter.committer().ok(), Some(signature(b"1592449083 +0800")));
    Ok(())
}

#[test]
fn whitespace() -> crate::Result {
    assert_eq!(
        CommitRefIter::from_bytes(&fixture_name("commit", "whitespace.txt")).collect::<Result<Vec<_>, _>>()?,
        vec![
            Token::Tree {
                id: hex_to_id("9bed6275068a0575243ba8409253e61af81ab2ff")
            },
            Token::Parent {
                id: hex_to_id("26b4df046d1776c123ac69d918f5aec247b58cc6")
            },
            Token::Author {
                signature: signature(b"1592448450 +0800")
            },
            Token::Committer {
                signature: signature(b"1592448450 +0800")
            },
            Token::Message(b" nl".as_bstr())
        ]
    );
    Ok(())
}

#[test]
fn unsigned() -> crate::Result {
    assert_eq!(
        CommitRefIter::from_bytes(&fixture_name("commit", "unsigned.txt")).collect::<Result<Vec<_>, _>>()?,
        vec![
            Token::Tree {
                id: hex_to_id("1b2dfb4ac5e42080b682fc676e9738c94ce6d54d")
            },
            Token::Author {
                signature: signature(b"1592437401 +0800")
            },
            Token::Committer {
                signature: signature(b"1592437401 +0800")
            },
            Token::Message(b"without sig".as_bstr())
        ]
    );
    Ok(())
}

#[test]
fn signed_singleline() -> crate::Result {
    assert_eq!(
        CommitRefIter::from_bytes(&fixture_name("commit", "signed-singleline.txt")).collect::<Result<Vec<_>, _>>()?,
        vec![
            Token::Tree {
                id: hex_to_id("00fc39317701176e326974ce44f5bd545a32ec0b")
            },
            Token::Parent {
                id: hex_to_id("09d8d3a12e161a7f6afb522dbe8900a9c09bce06")
            },
            Token::Author {
                signature: signature(b"1592391367 +0800")
            },
            Token::Committer {
                signature: signature(b"1592391367 +0800")
            },
            Token::ExtraHeader((b"gpgsig".as_bstr(), b"magic:signature".as_bstr().into())),
            Token::Message(b"update tasks\n".as_bstr()),
        ]
    );
    assert_eq!(
        CommitRefIter::from_bytes(&fixture_name("commit", "signed-singleline.txt"))
            .parent_ids()
            .collect::<Vec<_>>(),
        vec![hex_to_id("09d8d3a12e161a7f6afb522dbe8900a9c09bce06")]
    );
    Ok(())
}

#[test]
fn error_handling() -> crate::Result {
    let data = fixture_name("commit", "unsigned.txt");
    let iter = CommitRefIter::from_bytes(&data[..data.len() / 2]);
    let tokens = iter.collect::<Vec<_>>();
    assert!(
        tokens.last().expect("at least the errored token").is_err(),
        "errors are propagated and none is returned from that point on"
    );
    Ok(())
}

#[test]
fn mergetag() -> crate::Result {
    let input = fixture_name("commit", "mergetag.txt");
    let iter = CommitRefIter::from_bytes(&input);
    assert_eq!(
        iter.collect::<Result<Vec<_>, _>>()?,
        vec![
            Token::Tree {
                id: hex_to_id("1c61918031bf2c7fab9e17dde3c52a6a9884fcb5")
            },
            Token::Parent {
                id: hex_to_id("44ebe016df3aad96e3be8f95ec52397728dd7701")
            },
            Token::Parent {
                id: hex_to_id("8d485da0ddee79d0e6713405694253d401e41b93")
            },
            Token::Author {
                signature: linus_signature(b"1591996221 -0700")
            },
            Token::Committer {
                signature: linus_signature(b"1591996221 -0700")
            },
            Token::ExtraHeader((b"mergetag".as_bstr(), MERGE_TAG.as_bytes().as_bstr().into())),
            Token::Message(LONG_MESSAGE.into()),
        ]
    );
    assert_eq!(
        iter.parent_ids().collect::<Vec<_>>(),
        vec![
            hex_to_id("44ebe016df3aad96e3be8f95ec52397728dd7701"),
            hex_to_id("8d485da0ddee79d0e6713405694253d401e41b93")
        ]
    );
    assert_eq!(iter.message().ok(), Some(LONG_MESSAGE.into()));
    Ok(())
}

mod method {
    use gix_object::CommitRefIter;

    use crate::{fixture_name, hex_to_id, signature};

    #[test]
    fn tree_id() -> crate::Result {
        let input = fixture_name("commit", "unsigned.txt");
        let iter = CommitRefIter::from_bytes(&input);
        assert_eq!(
            iter.clone().tree_id().ok(),
            Some(hex_to_id("1b2dfb4ac5e42080b682fc676e9738c94ce6d54d"))
        );
        assert_eq!(
            iter.signatures().collect::<Vec<_>>(),
            vec![signature(b"1592437401 +0800"), signature(b"1592437401 +0800")]
        );
        assert_eq!(iter.parent_ids().count(), 0);
        Ok(())
    }

    #[test]
    fn signatures() -> crate::Result {
        let input = fixture_name("commit", "unsigned.txt");
        let iter = CommitRefIter::from_bytes(&input);
        assert_eq!(
            iter.signatures().collect::<Vec<_>>(),
            vec![signature(b"1592437401 +0800"), signature(b"1592437401 +0800")]
        );
        assert_eq!(iter.author().ok(), Some(signature(b"1592437401 +0800")));
        assert_eq!(iter.committer().ok(), Some(signature(b"1592437401 +0800")));
        assert_eq!(
            iter.author().ok(),
            Some(signature(b"1592437401 +0800")),
            "it's not consuming"
        );
        Ok(())
    }

    mod signature {
        use bstr::{BStr, BString, ByteSlice};
        use gix_object::CommitRefIter;

        use crate::{
            commit::{OTHER_SIGNATURE, SIGNATURE},
            fixture_name,
        };

        fn validate<'a>(
            fixture: &str,
            expected_signature: impl Into<&'a BStr>,
            signature_lines: std::ops::RangeInclusive<usize>,
        ) -> crate::Result {
            let expected_signature = expected_signature.into();
            let fixture_data = fixture_name("commit", fixture);

            let (actual_signature, actual_signed_data) = CommitRefIter::signature(&fixture_data)?.expect("sig present");
            let expected_signed_data: BString = fixture_data
                .lines_with_terminator()
                .enumerate()
                .filter_map(|(i, line)| (!signature_lines.contains(&i)).then_some(line))
                .collect();

            assert_eq!(actual_signed_data.to_bstring(), expected_signed_data);
            assert_eq!(actual_signature, expected_signature);

            Ok(())
        }

        #[test]
        fn single_line() -> crate::Result {
            validate("signed-singleline.txt", b"magic:signature", 4..=4)
        }

        #[test]
        fn signed() -> crate::Result {
            validate("signed.txt", b"-----BEGIN PGP SIGNATURE-----\n\niQEzBAABCAAdFiEEdjYp/sh4j8NRKLX27gKdHl60AwAFAl7p9tgACgkQ7gKdHl60\nAwBpegf+KQciv9AOIN7+yPmowecGxBnSfpKWTDzFxnyGR8dq63SpWT8WEKG5mf3a\nG6iUqpsDWaMHlzihaMKRvgRpZxFRbjnNPFBj6F4RRqfE+5R7k6DRSLUV5PqnsdSH\nuccfIDWi1imhsm7AaP5trwl1t+83U2JhHqPcPVFLMODYwWeO6NLR/JCzGSTQRa8t\nRgaVMKI19O/fge5OT5Ua8D47VKEhsJX0LfmkP5RfZQ8JJvNd40TupqKRdlv0sAzP\nya7NXkSHXCavHNR6kA+KpWxn900UoGK8/IDlwU6MeOkpPVawb3NFMqnc7KJDaC2p\nSMzpuEG8LTrCx2YSpHNLqHyzvQ1CZA==\n=5ITV\n-----END PGP SIGNATURE-----\n", 4..=14)
        }

        #[test]
        fn with_encoding() -> crate::Result {
            validate("signed-with-encoding.txt", SIGNATURE, 5..=15)
        }

        #[test]
        fn msg_footer() -> crate::Result {
            validate("message-with-footer.txt", b"-----BEGIN PGP SIGNATURE-----\n\niHUEABYIAB0WIQSuZwcGWSQItmusNgR5URpSUCnwXQUCYT7xpAAKCRB5URpSUCnw\nXWB3AP9q323HlxnI8MyqszNOeYDwa7Y3yEZaUM2y/IRjz+z4YQEAq0yr1Syt3mrK\nOSFCqL2vDm3uStP+vF31f6FnzayhNg0=\n=Mhpp\n-----END PGP SIGNATURE-----\n", 4..=10)
        }

        #[test]
        fn whitespace() -> crate::Result {
            validate("signed-whitespace.txt", OTHER_SIGNATURE, 5..=15)
        }
    }
}
