use gix_diff::blob::intern::InternedInput;
use gix_diff::blob::{
    git_diff::{ChangeGroup, ChangeKind},
    Algorithm, GitDiff,
};

#[test]
fn basic() {
    let before = r#"struct SomeStruct {
    field1: f64,
    field2: f64,
}

fn main() {
    // Some comment
    let c = SomeStruct { field1: 10.0, field2: 10.0 };

    println!(
        "Print field1 from SomeStruct {}",
        get_field1(&c)
    );
}

fn get_field1(c: &SomeStruct) -> f64 {
    c.field1
}
"#;

    let after = r#"/// This is a struct
struct SomeStruct {
    field1: f64,
    field2: f64,
}

fn main() {
    let c = SomeStruct { field1: 10.0, field2: 10.0 };

    println!(
        "Print field1 and field2 from SomeStruct {} {}",
        get_field1(&c), get_field2(&c)
    );
    println!("Print another line");
}

fn get_field1(c: &SomeStruct) -> f64 {
    c.field1
}

fn get_field2(c: &SomeStruct) -> f64 {
    c.field2
}
"#;
    use crate::blob::git_diff::ChangeKind;

    let input = InternedInput::new(before, after);
    let diff = gix_diff::blob::diff(Algorithm::Histogram, &input, GitDiff::new(&input));
    assert_eq!(
        diff,
        vec![
            ChangeGroup {
                before: 0..0,
                after: 0..1,
                change_kind: ChangeKind::Added
            },
            ChangeGroup {
                before: 6..7,
                after: 7..7,
                change_kind: ChangeKind::RemovedBelow
            },
            ChangeGroup {
                before: 10..12,
                after: 10..12,
                change_kind: ChangeKind::Modified
            },
            ChangeGroup {
                before: 13..13,
                after: 13..14,
                change_kind: ChangeKind::Added
            },
            ChangeGroup {
                before: 17..17,
                after: 19..23,
                change_kind: ChangeKind::Added
            }
        ]
    );
}
