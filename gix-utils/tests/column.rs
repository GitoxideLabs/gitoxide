use gix_utils::column::{Layout, Options};

#[test]
fn layouts_and_dense_columns() -> gix_testtools::TestResult {
    let items: Vec<_> = [
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
    ]
    .iter()
    .map(|item| item.as_bytes())
    .collect();
    for (layout, dense, expected) in [
        (
            Layout::Column,
            false,
            "one    seven
two    eight
three  nine
four   ten
five   eleven
six
",
        ),
        (
            Layout::Column,
            true,
            "one   five  nine
two   six   ten
three seven eleven
four  eight
",
        ),
        (
            Layout::Row,
            true,
            "one   two    three
four  five   six
seven eight  nine
ten   eleven
",
        ),
    ] {
        let mut actual = Vec::new();
        gix_utils::column::write(
            &items,
            Options {
                width: 19,
                layout,
                dense,
                ..Options::default()
            },
            &mut actual,
        )?;
        assert_eq!(actual, expected.as_bytes(), "layout {layout:?}, dense {dense}");
    }
    Ok(())
}

#[test]
fn display_width_and_extreme_dimensions() -> gix_testtools::TestResult {
    let items: Vec<&[u8]> = vec!["é".as_bytes(), b"\x1b[31ma\x1b[m", "界".as_bytes(), b"z"];
    let mut actual = Vec::new();
    gix_utils::column::write(
        &items,
        Options {
            width: 6,
            ..Options::default()
        },
        &mut actual,
    )?;
    assert_eq!(
        actual,
        "é  界\n\x1b[31ma\x1b[m  z\n".as_bytes(),
        "colors do not consume columns and wide characters do"
    );
    for item in [b"\xff\x1b[31ma\x1b[m".as_slice(), "👩‍💻".as_bytes()] {
        let mut output = Vec::new();
        gix_utils::column::write(
            &[item, b"X"],
            Options {
                width: 6,
                layout: Layout::Row,
                ..Options::default()
            },
            &mut output,
        )?;
        assert_eq!(
            output,
            [item, b"\nX\n"].concat(),
            "invalid UTF-8 and emoji use Git character widths"
        );
    }
    for width in [0, 1, usize::MAX] {
        let mut output = Vec::new();
        gix_utils::column::write(
            &[b"", b""],
            Options {
                width,
                padding: 0,
                ..Options::default()
            },
            &mut output,
        )?;
        assert!(
            !output.is_empty(),
            "zero-width items and zero padding must not divide by zero"
        );
    }
    Ok(())
}

#[test]
fn output_errors_are_propagated() {
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let error = gix_utils::column::write(&[b"x"], Options::default(), Broken).expect_err("closed output fails");
    assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
}
