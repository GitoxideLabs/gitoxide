use bstr::ByteSlice;
use gix_object::commit::{MessageRef, message::body::TrailerRef};

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

/// Via `MessageRef`: a subject-only message with a trailer immediately
/// after the blank line (the common case in the wild) must surface the
/// trailer through the public `MessageRef` API.
#[test]
fn trailer_as_sole_body_content() {
    let msg = MessageRef::from_bytes(b"Fix the thing\n\nSigned-off-by: Alice <alice@example.com>\n");
    let body = msg.body().expect("body is present");
    let trailers: Vec<_> = body.trailers().collect();
    assert_eq!(msg.title, "Fix the thing");
    assert_eq!(body.without_trailer(), "");
    assert_eq!(
        trailers,
        vec![TrailerRef {
            token: "Signed-off-by".into(),
            value: b"Alice <alice@example.com>".as_bstr().into(),
        }],
    );
}

#[test]
fn folded_trailer_as_sole_body_content_via_message_ref() {
    let msg = MessageRef::from_bytes(b"Fix the thing\n\nAcked-by: Alice\n continuation line\n");
    let body = msg.body().expect("body is present");
    assert_eq!(msg.title, "Fix the thing");
    assert_eq!(body.without_trailer(), "");
    let trailers: Vec<_> = body.trailers().collect();
    assert_eq!(
        trailers,
        vec![TrailerRef {
            token: "Acked-by".into(),
            value: b"Alice continuation line".as_bstr().into(),
        }],
    );
}

mod body {
    use bstr::ByteSlice;
    use gix_object::commit::{
        MessageRef,
        message::{BodyRef, body::TrailerRef},
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
                value: b"value".as_bstr().into()
            }]
        );
    }

    #[test]
    fn generic_trailers_mixed_with_prose_after_a_few_paragraphs_are_not_a_trailer_block() {
        let input = "foo\nbar\n\nbar\n\nbaz\n\na: b\ncannot parse this\r\nc: d\n";
        let body = body(input);
        assert_eq!(body.as_ref(), input);
        assert_eq!(body.trailers().collect::<Vec<_>>(), vec![]);
    }

    #[test]
    fn no_trailer_after_a_paragraph_windows() {
        let input = "foo\nbar\n\nbar\r\n\r\nbaz";
        assert_eq!(body(input).as_ref(), input);
    }

    /// A commit whose body is *only* trailers (no preceding body paragraph) should
    /// have its trailers detected, matching the behaviour of `git interpret-trailers`.
    ///
    /// This arises naturally when a commit message has a subject line followed
    /// immediately by trailers and no other body text, e.g.:
    ///
    /// ```text
    /// Fix the thing
    ///
    /// Signed-off-by: Alice <alice@example.com>
    /// ```
    ///
    /// The full message bytes are `"Fix the thing\n\nSigned-off-by: …"`.
    /// `MessageRef::from_bytes` splits at the first `\n\n`, yielding the body
    /// `"Signed-off-by: …"` — which contains no second `\n\n`.  Prior to this
    /// fix `BodyRef::from_bytes` therefore returned zero trailers for such
    /// messages, diverging from `git interpret-trailers --parse`.
    #[test]
    fn trailer_as_sole_body_content() {
        let input = "Signed-off-by: Alice <alice@example.com>\nCo-authored-by: Bob <bob@example.com>";
        let body = body(input);
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![
                TrailerRef {
                    token: "Signed-off-by".into(),
                    value: b"Alice <alice@example.com>".as_bstr().into(),
                },
                TrailerRef {
                    token: "Co-authored-by".into(),
                    value: b"Bob <bob@example.com>".as_bstr().into(),
                },
            ],
        );
        assert_eq!(body.without_trailer(), "", "body-without-trailer must be empty");
    }

    /// Generic trailer-looking lines mixed with prose in the same paragraph do not
    /// form a trailer block without a recognized Git prefix.
    #[test]
    fn body_text_then_generic_trailer_without_blank_line_is_not_a_trailer() {
        let input = "some body text\ntoken: value";
        let body = body(input);
        assert_eq!(body.without_trailer(), input, "must be returned unchanged");
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![],
            "generic trailer-like lines are not enough to form a trailer block"
        );
    }

    /// Git recognizes `Signed-off-by` as a special trailer prefix, so a footer block
    /// that mixes prose with such trailers still parses even without an extra blank
    /// line inside the body.
    #[test]
    fn body_text_then_recognized_trailer_without_blank_line_is_a_trailer_block() {
        let input = "some body text\nSigned-off-by: Alice <alice@example.com>";
        let body = body(input);
        assert_eq!(body.without_trailer(), "", "the whole body is the trailer block");
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![TrailerRef {
                token: "Signed-off-by".into(),
                value: b"Alice <alice@example.com>".as_bstr().into(),
            }],
        );
    }

    /// A body whose first line looks like a trailer but whose subsequent
    /// lines are plain prose must not be treated as a trailer block.
    #[test]
    fn trailer_like_first_line_followed_by_prose_is_not_a_trailer() {
        let input = "Note: this is not a trailer despite the colon\nmore explanation";
        let body = body(input);
        assert_eq!(body.without_trailer(), input, "must be returned unchanged");
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![],
            "not a trailer block when non-trailer lines are present"
        );
    }

    #[test]
    fn trailer_as_sole_body_content_with_folded_value() {
        let input = "Acked-by: Alice\n continuation line";
        let body = body(input);
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![TrailerRef {
                token: "Acked-by".into(),
                value: b"Alice continuation line".as_bstr().into(),
            }],
        );
        assert_eq!(body.without_trailer(), "", "body-without-trailer must be empty");
    }

    #[test]
    fn trailer_as_sole_body_content_with_space_before_separator() {
        let input = "Acked-by : Alice";
        let body = body(input);
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![TrailerRef {
                token: "Acked-by".into(),
                value: b"Alice".as_bstr().into(),
            }],
        );
        assert_eq!(body.without_trailer(), "", "body-without-trailer must be empty");
    }

    #[test]
    fn mixed_footer_with_recognized_prefix_and_prose_is_a_trailer_block() {
        let input = "not a trailer\nSigned-off-by: Alice <alice@example.com>\nanother note\nSigned-off-by: Bob <bob@example.com>";
        let body = body(input);
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![
                TrailerRef {
                    token: "Signed-off-by".into(),
                    value: b"Alice <alice@example.com>".as_bstr().into(),
                },
                TrailerRef {
                    token: "Signed-off-by".into(),
                    value: b"Bob <bob@example.com>".as_bstr().into(),
                },
            ],
        );
        assert_eq!(
            body.without_trailer(),
            "",
            "the entire body is the trailer block, even though not everything can be parsed"
        );
    }

    #[test]
    fn recognized_prefix_footer_at_exactly_twenty_five_percent_is_a_trailer_block() {
        let input = "Signed-off-by: Alice <alice@example.com>\n\
not a trailer 1\n\
not a trailer 2\n\
not a trailer 3";
        let body = body(input);
        assert_eq!(body.without_trailer(), "", "the whole body is the trailer block");
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![TrailerRef {
                token: "Signed-off-by".into(),
                value: b"Alice <alice@example.com>".as_bstr().into(),
            }],
        );
    }

    #[test]
    fn recognized_prefix_footer_below_twenty_five_percent_is_not_a_trailer_block() {
        let input = "Signed-off-by: Alice <alice@example.com>\n\
not a trailer 1\n\
not a trailer 2\n\
not a trailer 3\n\
not a trailer 4";
        let body = body(input);
        assert_eq!(body.without_trailer(), input, "must be returned unchanged");
        assert_eq!(body.trailers().collect::<Vec<_>>(), vec![]);
    }

    #[test]
    fn mixed_footer_with_only_generic_trailers_and_prose_is_not_a_trailer_block() {
        let input = "a: b\nnot a trailer\nc: d";
        let body = body(input);
        assert_eq!(body.without_trailer(), input, "must be returned unchanged");
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![],
            "generic trailers mixed with prose do not form a trailer block"
        );
    }

    #[test]
    fn folded_trailer_after_body_paragraph() {
        let input = "body paragraph\n\nAcked-by: Alice\n continuation line";
        let body = body(input);
        assert_eq!(body.without_trailer(), "body paragraph");
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![TrailerRef {
                token: "Acked-by".into(),
                value: b"Alice continuation line".as_bstr().into(),
            }],
        );
    }

    #[test]
    fn trailer_as_sole_body_content_with_folded_value_windows() {
        let input = "Acked-by: Alice\r\n continuation line\r\n";
        let body = body(input);
        assert_eq!(
            body.trailers().collect::<Vec<_>>(),
            vec![TrailerRef {
                token: "Acked-by".into(),
                value: b"Alice continuation line".as_bstr().into(),
            }],
        );
        assert_eq!(body.without_trailer(), "", "body-without-trailer must be empty");
    }
}

mod summary {
    use std::borrow::Cow;

    use gix_actor::SignatureRef;
    use gix_object::{
        CommitRef,
        bstr::{BStr, ByteSlice},
        commit::MessageRef,
    };

    fn summary(input: &[u8]) -> Cow<'_, BStr> {
        let summary = MessageRef::from_bytes(input).summary();
        let actor = SignatureRef {
            name: "name".into(),
            email: "email".into(),
            time: "0 0000",
        };
        let commit = CommitRef {
            tree: "tree".into(),
            parents: Default::default(),
            author: "name <email> 0 0000".as_bytes().as_bstr(),
            committer: "name <email> 0 0000".as_bytes().as_bstr(),
            encoding: None,
            message: input.as_bstr(),
            extra_headers: vec![],
        };
        assert_eq!(
            commit.message_summary(),
            summary,
            "both versions create the same result"
        );
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
