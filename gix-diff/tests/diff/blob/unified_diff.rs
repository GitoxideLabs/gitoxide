use gix_diff::blob::UnifiedDiff;
use gix_diff::blob::unified_diff::ConsumeBinaryHunk;
use gix_diff::blob::{
    Algorithm,
    unified_diff::{ConsumeHunk, ContextSize, DiffLineKind, HunkHeader},
};
use gix_object::bstr::BString;

#[test]
fn removed_modified_added() -> crate::Result {
    let a = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10";
    let b = "2\n3\n4\n5\nsix\n7\n8\n9\n10\neleven\ntwelve";

    let interner = gix_diff::blob::InternedInput::new(
        gix_diff::blob::platform::resource::ByteLinesWithoutTerminator::new(a.as_bytes()),
        gix_diff::blob::platform::resource::ByteLinesWithoutTerminator::new(b.as_bytes()),
    );
    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(3),
    )?;

    // merged by context.
    insta::assert_snapshot!(actual, @r"
    @@ -1,10 +1,11 @@
    -1
     2
     3
     4
     5
    -6
    +six
     7
     8
     9
     10
    +eleven
    +twelve
    ");

    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(1),
    )?;
    // Small context lines keeps hunks separate.
    insta::assert_snapshot!(actual, @r"
    @@ -1,2 +1,1 @@
    -1
     2
    @@ -5,3 +4,3 @@
     5
    -6
    +six
     7
    @@ -10,1 +9,3 @@
     10
    +eleven
    +twelve
    ");

    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(0),
    )?;
    // No context is also fine
    insta::assert_snapshot!(actual, @r"
    @@ -1,1 +1,0 @@
    -1
    @@ -6,1 +5,1 @@
    -6
    +six
    @@ -11,0 +10,2 @@
    +eleven
    +twelve
    ");

    let actual = render(
        Algorithm::Myers,
        &interner,
        Recorder::new("\n"),
        ContextSize::symmetrical(1),
    )?;
    assert_eq!(
        actual,
        &[
            ((1, 2), (1, 1), "@@ -1,2 +1,1 @@\n".to_string()),
            ((5, 3), (4, 3), "@@ -5,3 +4,3 @@\n".into()),
            ((10, 1), (9, 3), "@@ -10,1 +9,3 @@\n".into())
        ]
    );

    Ok(())
}

#[test]
fn context_overlap_by_one_line_move_up() -> crate::Result {
    let a = "2\n3\n4\n5\n6\n7\n";
    let b = "7\n2\n3\n4\n5\n6\n";

    let interner = gix_diff::blob::InternedInput::new(a, b);
    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(3),
    )?;

    // merged by context.
    insta::assert_snapshot!(actual, @r"
    @@ -1,6 +1,6 @@
    +7
     2
     3
     4
     5
     6
    -7
    ");
    Ok(())
}

#[test]
fn non_utf8() -> crate::Result {
    let a = &b"\xC0\x80"[..];
    let b = b"ascii";

    let interner = gix_diff::blob::InternedInput::new(a, b);
    let err = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(3),
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "invalid UTF-8 found at byte offset 1",
        "strings enforce an encoding, which fails here"
    );

    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(BString::default(), "\n"),
        ContextSize::symmetrical(3),
    )?;
    insta::assert_snapshot!(actual, @r"
    @@ -1,1 +1,1 @@
    -��
    +ascii
    ");
    Ok(())
}

#[test]
fn context_overlap_by_one_line_move_down() -> crate::Result {
    let a = "2\n3\n4\n5\n6\n7\n";
    let b = "7\n2\n3\n4\n5\n6\n";

    let interner = gix_diff::blob::InternedInput::new(b, a);
    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(3),
    )?;

    // merged by context.
    insta::assert_snapshot!(actual, @r"
    @@ -1,6 +1,6 @@
    -7
     2
     3
     4
     5
     6
    +7
    ");
    Ok(())
}

#[test]
fn added_on_top_keeps_context_correctly_sized() -> crate::Result {
    let a = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10";
    let b = "1\n2\n3\n4\n4.5\n5\n6\n7\n8\n9\n10";

    let a = gix_diff::blob::sources::byte_lines(a.as_bytes());
    let b = gix_diff::blob::sources::byte_lines(b.as_bytes());
    let interner = gix_diff::blob::InternedInput::new(a, b);

    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(3),
    )?;
    // TODO: fix this
    insta::assert_snapshot!(actual, @r"
    @@ -2,6 +2,7 @@
     2
     3
     4
    +4.5
     5
     6
     7
    ");

    let a = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10";
    let b = "1\n2\n3\n4\n5\n6\n6.5\n7\n8\n9\n10";

    let a = gix_diff::blob::sources::byte_lines(a.as_bytes());
    let b = gix_diff::blob::sources::byte_lines(b.as_bytes());
    let interner = gix_diff::blob::InternedInput::new(a, b);

    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(3),
    )?;

    insta::assert_snapshot!(actual, @r"
    @@ -4,6 +4,7 @@
     4
     5
     6
    +6.5
     7
     8
     9
    ");
    let a = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10";
    let b = "1\n2\n3\n3.5\n4\n5\n6\n7\n8\n9\n10";

    let a = gix_diff::blob::sources::byte_lines(a.as_bytes());
    let b = gix_diff::blob::sources::byte_lines(b.as_bytes());
    let interner = gix_diff::blob::InternedInput::new(a, b);

    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(3),
    )?;

    insta::assert_snapshot!(actual, @r"
    @@ -1,6 +1,7 @@
     1
     2
     3
    +3.5
     4
     5
     6
    ");

    // From the end, for good measure
    let a = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10";
    let b = "1\n2\n3\n4\n5\n6\n7\n7.5\n8\n9\n10";

    let a = gix_diff::blob::sources::byte_lines(a.as_bytes());
    let b = gix_diff::blob::sources::byte_lines(b.as_bytes());
    let interner = gix_diff::blob::InternedInput::new(a, b);

    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(3),
    )?;
    insta::assert_snapshot!(actual, @r"
    @@ -5,6 +5,7 @@
     5
     6
     7
    +7.5
     8
     9
     10
    ");
    Ok(())
}

#[test]
fn removed_modified_added_with_newlines_in_tokens() -> crate::Result {
    let a = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10";
    let b = "2\n3\n4\n5\nsix\n7\n8\n9\n10\neleven\ntwelve";

    let a = gix_diff::blob::sources::byte_lines(a.as_bytes());
    let b = gix_diff::blob::sources::byte_lines(b.as_bytes());
    let interner = gix_diff::blob::InternedInput::new(a, b);
    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(3),
    )?;

    // merged by context.
    // newline diffs differently.
    insta::assert_snapshot!(actual, @r"
    @@ -1,10 +1,11 @@
    -1
     2
     3
     4
     5
    -6
    +six
     7
     8
     9
    -10
    +10
    +eleven
    +twelve
    ");

    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(1),
    )?;
    // Small context lines keeps hunks separate.
    insta::assert_snapshot!(actual, @r"
    @@ -1,2 +1,1 @@
    -1
     2
    @@ -5,3 +4,3 @@
     5
    -6
    +six
     7
    @@ -9,2 +8,4 @@
     9
    -10
    +10
    +eleven
    +twelve
    ");

    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(0),
    )?;
    // No context is also fine
    insta::assert_snapshot!(actual, @r"
    @@ -1,1 +1,0 @@
    -1
    @@ -6,1 +5,1 @@
    -6
    +six
    @@ -10,1 +9,3 @@
    -10
    +10
    +eleven
    +twelve
    ");

    let actual = render(
        Algorithm::Myers,
        &interner,
        Recorder::new("\r\n"),
        ContextSize::symmetrical(1),
    )?;
    assert_eq!(
        actual,
        &[
            ((1, 2), (1, 1), "@@ -1,2 +1,1 @@\r\n".to_string()),
            ((5, 3), (4, 3), "@@ -5,3 +4,3 @@\r\n".into()),
            ((9, 2), (8, 4), "@@ -9,2 +8,4 @@\r\n".into())
        ]
    );

    let actual = render(
        Algorithm::Myers,
        &interner,
        DiffLineKindRecorder::default(),
        ContextSize::symmetrical(1),
    )?;

    assert_eq!(
        actual,
        &[
            vec![DiffLineKind::Remove, DiffLineKind::Context],
            vec![
                DiffLineKind::Context,
                DiffLineKind::Remove,
                DiffLineKind::Add,
                DiffLineKind::Context
            ],
            vec![
                DiffLineKind::Context,
                DiffLineKind::Remove,
                DiffLineKind::Add,
                DiffLineKind::Add,
                DiffLineKind::Add
            ]
        ]
    );

    Ok(())
}

#[test]
fn all_added_or_removed() -> crate::Result {
    let content = "1\n2\n3\n4\n5";

    let samples = [0, 1, 3, 100];
    for context_lines in samples {
        let interner = gix_diff::blob::InternedInput::new("", content);
        let actual = render(
            Algorithm::Myers,
            &interner,
            ConsumeBinaryHunk::new(String::new(), "\n"),
            ContextSize::symmetrical(context_lines),
        )?;
        assert_eq!(
            actual,
            r#"@@ -1,0 +1,5 @@
+1
+2
+3
+4
+5
"#,
            "context lines don't matter here"
        );
    }

    for context_lines in samples {
        let interner = gix_diff::blob::InternedInput::new(content, "");
        let actual = render(
            Algorithm::Myers,
            &interner,
            ConsumeBinaryHunk::new(String::new(), "\n"),
            ContextSize::symmetrical(context_lines),
        )?;
        assert_eq!(
            actual,
            r"@@ -1,5 +1,0 @@
-1
-2
-3
-4
-5
",
            "context lines don't matter here"
        );
    }
    Ok(())
}

#[test]
fn empty() -> crate::Result {
    let interner = gix_diff::blob::InternedInput::new(&b""[..], &b""[..]);
    let actual = render(
        Algorithm::Myers,
        &interner,
        ConsumeBinaryHunk::new(String::new(), "\n"),
        ContextSize::symmetrical(3),
    )?;

    insta::assert_snapshot!(actual, @r"");
    Ok(())
}

fn render<T, D>(
    algorithm: Algorithm,
    input: &gix_diff::blob::InternedInput<T>,
    delegate: D,
    context_size: ContextSize,
) -> std::io::Result<D::Out>
where
    T: AsRef<[u8]> + std::hash::Hash + Eq,
    D: ConsumeHunk,
{
    let diff = gix_diff::blob::Diff::compute(algorithm, input);
    UnifiedDiff::new(&diff, input, delegate, context_size).consume()
}

struct Recorder {
    #[allow(clippy::type_complexity)]
    hunks: Vec<((u32, u32), (u32, u32), String)>,
    newline: &'static str,
}

impl Recorder {
    pub fn new(newline: &'static str) -> Self {
        Recorder {
            hunks: Vec::new(),
            newline,
        }
    }
}

impl ConsumeHunk for Recorder {
    type Out = Vec<((u32, u32), (u32, u32), String)>;

    fn consume_hunk(&mut self, header: HunkHeader, _hunk: &[(DiffLineKind, &[u8])]) -> std::io::Result<()> {
        let mut formatted_header = header.to_string();
        formatted_header.push_str(self.newline);

        self.hunks.push((
            (header.before_hunk_start, header.before_hunk_len),
            (header.after_hunk_start, header.after_hunk_len),
            formatted_header,
        ));
        Ok(())
    }

    fn finish(self) -> Self::Out {
        self.hunks
    }
}

#[derive(Default)]
struct DiffLineKindRecorder {
    line_kinds: Vec<Vec<DiffLineKind>>,
}

impl ConsumeHunk for DiffLineKindRecorder {
    type Out = Vec<Vec<DiffLineKind>>;

    fn consume_hunk(&mut self, _header: HunkHeader, hunk: &[(DiffLineKind, &[u8])]) -> std::io::Result<()> {
        self.line_kinds
            .push(hunk.iter().map(|(line_type, _)| *line_type).collect());
        Ok(())
    }

    fn finish(self) -> Self::Out {
        self.line_kinds
    }
}
