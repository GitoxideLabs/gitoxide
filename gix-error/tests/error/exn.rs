use crate::fixup_paths;
use gix_error::{ErrorExt, IteratorExt, Message, message};

fn remove_stackstrace(s: String) -> String {
    fixup_paths(s.find("Stack backtrace:").map_or(s.clone(), |pos| s[..pos].into()))
}

#[test]
fn into_chain() {
    let e1 = [
        [message("E1c1-1"), message("E1c1-2")]
            .into_iter()
            .raise(message("E1-2")),
        [message("E1c2-1"), message("E1c2-2")]
            .into_iter()
            .raise(message("E1-3")),
    ]
    .into_iter()
    .raise(message("E1"));
    let e2 = e1.raise(message("E2"));
    let root = e2.raise(Message::new("root"));

    // It's a linked list as linked up with the first child, but also has multiple children.
    let root = gix_error::ChainedError::from(root);
    // By default, there is paths displayed, just like everywhere.
    insta::assert_debug_snapshot!(causes_display(&root, Style::Normal), "into_chain exposes locations for every source", @r#"
    [
        "root, at gix-error/tests/error/exn.rs:21",
        "E2, at gix-error/tests/error/exn.rs:20",
        "E1, at gix-error/tests/error/exn.rs:19",
        "E1-2, at gix-error/tests/error/exn.rs:13",
        "E1-3, at gix-error/tests/error/exn.rs:16",
        "E1c1-1, at gix-error/tests/error/exn.rs:13",
        "E1c1-2, at gix-error/tests/error/exn.rs:13",
        "E1c2-1, at gix-error/tests/error/exn.rs:16",
        "E1c2-2, at gix-error/tests/error/exn.rs:16",
    ]
    "#);

    // But these can also be turned off
    insta::assert_debug_snapshot!(causes_display(&root, Style::Alternate), "alternate source display omits locations", @r#"
    [
        "root",
        "E2",
        "E1",
        "E1-2",
        "E1-3",
        "E1c1-1",
        "E1c1-2",
        "E1c2-1",
        "E1c2-2",
    ]
    "#);

    // This should look similar.
    insta::assert_snapshot!(remove_stackstrace(format!("{:?}", anyhow::Error::from(root))), "into_chain matches anyhow source traversal", @"
    root, at gix-error/tests/error/exn.rs:21

    Caused by:
        0: E2, at gix-error/tests/error/exn.rs:20
        1: E1, at gix-error/tests/error/exn.rs:19
        2: E1-2, at gix-error/tests/error/exn.rs:13
        3: E1-3, at gix-error/tests/error/exn.rs:16
        4: E1c1-1, at gix-error/tests/error/exn.rs:13
        5: E1c1-2, at gix-error/tests/error/exn.rs:13
        6: E1c2-1, at gix-error/tests/error/exn.rs:16
        7: E1c2-2, at gix-error/tests/error/exn.rs:16
    ");
}

enum Style {
    Normal,
    Alternate,
}

fn causes_display(err: &(dyn std::error::Error + 'static), style: Style) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = Some(err);
    while let Some(err) = current {
        out.push(fixup_paths(match style {
            Style::Normal => err.to_string(),
            Style::Alternate => {
                format!("{err:#}")
            }
        }));
        current = err.source();
    }
    out
}

#[test]
fn boxed_upstream_exception_retains_its_tree_and_types() {
    let native: exn::Exn<_> = gix_error::ValidationError::new("invalid input")
        .raise()
        .raise(message("operation failed"));
    let boxed: Box<dyn std::error::Error + Send + Sync> = native.into();
    let error = gix_error::Error::from_boxed(boxed);
    assert_eq!(
        error.iter_errors().map(ToString::to_string).collect::<Vec<_>>(),
        ["operation failed", "invalid input"]
    );
    assert!(
        error.is_validation(),
        "boxing an upstream frame must retain concrete causes for classification"
    );
}
