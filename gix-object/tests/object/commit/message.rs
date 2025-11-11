use bstr::ByteSlice;
use gix_object::commit::MessageRef;

#[test]
fn only_title_no_trailing_newline() {
    let msg = MessageRef::from_bytes(b"hello there");
    assert_eq!(
        msg,
        MessageRef {
            title: b"hello there".as_bstr(),
            body: None
        }
    );
    assert_eq!(msg.summary().as_ref(), "hello there");
}

#[test]
fn title_and_body() {
    let msg = MessageRef::from_bytes(b"hello\n\nthere");
    assert_eq!(
        msg,
        MessageRef {
            title: b"hello".as_bstr(),
            body: Some("there".into())
        }
    );
    assert_eq!(msg.summary().as_ref(), "hello");
}

#[test]
fn title_and_body_inconsistent_newlines() {
    let msg = MessageRef::from_bytes(b"hello\n\r\nthere");
    assert_eq!(
        msg,
        MessageRef {
            title: b"hello".as_bstr(),
            body: Some("there".into())
        }
    );
    assert_eq!(msg.summary().as_ref(), "hello");
}

#[test]
fn only_title_trailing_newline_is_retained() {
    let msg = MessageRef::from_bytes(b"hello there\n");
    assert_eq!(
        msg,
        MessageRef {
            title: b"hello there\n".as_bstr(),
            body: None
        }
    );
    assert_eq!(msg.summary().as_ref(), "hello there");
}

#[test]
fn only_title_trailing_windows_newline_is_retained() {
    let msg = MessageRef::from_bytes(b"hello there\r\n");
    assert_eq!(
        msg,
        MessageRef {
            title: b"hello there\r\n".as_bstr(),
            body: None
        }
    );
    assert_eq!(msg.summary().as_ref(), "hello there");
}

#[test]
fn title_with_whitespace_and_body() {
    let msg = MessageRef::from_bytes(b"hello \t \r\n there\nanother line\n\nthe body\n\n");
    assert_eq!(msg.summary().as_ref(), "hello  there another line");
    assert_eq!(
        msg,
        MessageRef {
            title: b"hello \t \r\n there\nanother line".as_bstr(),
            body: Some(b"the body\n\n".as_bstr())
        }
    );
}

#[test]
fn title_with_more_whitespace_and_body() {
    let msg = MessageRef::from_bytes(b"hello \r\r\r\n there\nanother line\n\nthe body\n\n");
    assert_eq!(msg.summary().as_ref(), "hello  there another line");
    assert_eq!(
        msg,
        MessageRef {
            title: b"hello \r\r\r\n there\nanother line".as_bstr(),
            body: Some(b"the body\n\n".as_bstr())
        }
    );
}

#[test]
fn title_with_whitespace_and_body_windows_lineending() {
    let msg = MessageRef::from_bytes(b"hello \r\n \r\n there\nanother line\r\n\r\nthe body\n\r\n");
    assert_eq!(msg.summary().as_ref(), "hello   there another line");
    assert_eq!(
        msg,
        MessageRef {
            title: b"hello \r\n \r\n there\nanother line".as_bstr(),
            body: Some(b"the body\n\r\n".as_bstr())
        }
    );
}

#[test]
fn title_with_separator_and_empty_body() {
    let msg = MessageRef::from_bytes(b"hello\n\n");
    assert_eq!(msg.summary().as_ref(), "hello");
    assert_eq!(
        msg,
        MessageRef {
            title: b"hello".as_bstr(),
            body: None
        }
    );
}

#[test]
fn title_with_windows_separator_and_empty_body() {
    let msg = MessageRef::from_bytes(b"hello\r\n\r\n");
    assert_eq!(msg.summary().as_ref(), "hello");
    assert_eq!(
        msg,
        MessageRef {
            title: b"hello".as_bstr(),
            body: None
        }
    );
}

mod body {
    use gix_object::commit::{
        message::{body::TrailerRef, BodyRef},
        MessageRef,
    };

    fn body(input: &str) -> BodyRef<'_> {
        BodyRef::from_bytes(input.as_bytes())
    }

    #[test]
    fn created_manually_is_the_same_as_through_message_ref() {
        assert_eq!(
            MessageRef {
                title: "title unused".into(),
                body: Some("hello".into()),
            }
            .body()
            .expect("present"),
            BodyRef::from_bytes("hello".as_bytes())
        );
    }

    #[test]
    fn no_trailer() {
        let input = "foo\nbar";
        assert_eq!(body(input).as_ref(), input);
        assert_eq!(body(input).without_trailer(), input);
    }

    #[test]
    fn no_trailer_after_a_few_paragraphs_empty_last_block() {
        let input = "foo\nbar\n\nbar\n\nbaz\n\n";
        assert_eq!(body(input).as_ref(), input);
    }

    #[test]
    fn no_trailer_after_a_few_paragraphs_empty_last_block_windows() {
        let input = "foo\nbar\n\nbar\n\nbaz\r\n\r\n";
        assert_eq!(body(input).as_ref(), input);
    }

    #[test]
    fn no_trailer_after_a_few_paragraphs() {
        let input = "foo\nbar\n\nbar\n\nbaz";
        assert_eq!(body(input).as_ref(), input);
    }

    #[test]
    fn single_trailer_after_a_few_paragraphs() {
        let input = "foo\nbar\n\nbar\n\nbaz\n\ntoken: value";
        let body = body(input);
        assert_eq!(body.as_ref(), "foo\nbar\n\nbar\n\nbaz");
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![TrailerRef {
                token: "token".into(),
                value: "value".into()
            }]
        );
    }

    #[test]
    fn two_trailers_with_broken_one_inbetween_after_a_few_paragraphs() {
        let input = "foo\nbar\n\nbar\n\nbaz\n\na: b\ncannot parse this\r\nc: d\n";
        let body = body(input);
        assert_eq!(body.as_ref(), "foo\nbar\n\nbar\n\nbaz");
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![
                TrailerRef {
                    token: "a".into(),
                    value: "b".into()
                },
                TrailerRef {
                    token: "c".into(),
                    value: "d".into()
                }
            ]
        );
    }

    #[test]
    fn no_trailer_after_a_paragraph_windows() {
        let input = "foo\nbar\n\nbar\r\n\r\nbaz";
        assert_eq!(body(input).as_ref(), input);
    }
}

mod summary {
    use std::borrow::Cow;

    use gix_actor::SignatureRef;
    use gix_object::{
        bstr::{BStr, ByteSlice},
        commit::MessageRef,
        CommitRef,
    };

    fn summary(input: &[u8]) -> Cow<'_, BStr> {
        let summary = MessageRef::from_bytes(input).summary();
        let actor = SignatureRef {
            name: "name".into(),
            email: "email".into(),
            time: "0 0000",
        };
        assert_eq!(
            CommitRef {
                tree: "tree".into(),
                parents: Default::default(),
                author: "name <email> 0 0000".as_bytes().as_bstr(),
                committer: "name <email> 0 0000".as_bytes().as_bstr(),
                encoding: None,
                message: input.as_bstr(),
                extra_headers: vec![]
            }
            .message_summary(),
            summary,
            "both versions create the same result"
        );
        let commit = CommitRef {
            tree: "tree".into(),
            parents: Default::default(),
            author: "name <email> 0 0000".as_bytes().as_bstr(),
            committer: "name <email> 0 0000".as_bytes().as_bstr(),
            encoding: None,
            message: input.as_bstr(),
            extra_headers: vec![],
        };
        assert_eq!(commit.author().unwrap(), actor);
        assert_eq!(commit.committer().unwrap(), actor);
        summary
    }

    #[test]
    fn no_newline_yields_the_message_itself() {
        let input = b"hello world".as_bstr();
        assert_eq!(summary(input), Cow::Borrowed(input));
    }

    #[test]
    fn trailing_newlines_and_whitespace_are_trimmed() {
        let input = b"hello world \t\r\n \n";
        assert_eq!(summary(input), Cow::Borrowed(b"hello world".as_bstr()));
    }

    #[test]
    fn prefixed_newlines_and_whitespace_are_trimmed() {
        let input = b" \t\r\n \nhello world";
        assert_eq!(summary(input), Cow::Borrowed(b"hello world".as_bstr()));
    }

    #[test]
    fn whitespace_up_to_a_newline_is_collapsed_into_a_space() {
        let input = b" \t\r\n \nhello\r\nworld \t\r\n \n";
        assert_eq!(summary(input), Cow::Borrowed(b"hello world".as_bstr()));
    }

    #[test]
    fn whitespace_without_newlines_is_ignored_except_for_leading_and_trailing_whitespace() {
        let input = b" \t\r\n \nhello \t \rworld \t\r\n \n";
        assert_eq!(summary(input), Cow::Borrowed(b"hello \t \rworld".as_bstr()));
    }

    #[test]
    fn lines_separated_by_double_newlines_are_subjects() {
        let input = b" \t\r\n \nhello\t \r\nworld \t\r \nfoo\n\nsomething else we ignore";
        assert_eq!(summary(input), Cow::Borrowed(b"hello world foo".as_bstr()));
    }
}
