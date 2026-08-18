use bstr::ByteSlice;

pub(crate) mod pipeline;
mod platform;
mod slider;
mod textconv;
mod unified_diff;

pub(crate) fn skip_header_and_fold_to_unidiff(content: &[u8]) -> String {
    let mut lines = content.lines();

    assert!(lines.next().expect("diff header").starts_with(b"diff --git "));
    assert!(lines.next().expect("index header").starts_with(b"index "));
    assert!(lines.next().expect("--- header").starts_with(b"--- "));
    assert!(lines.next().expect("+++ header").starts_with(b"+++ "));

    let mut out = String::new();
    for line in lines {
        if line.starts_with(b"\\") {
            continue;
        }
        out.push_str(line.to_str().expect("baseline diff is valid utf-8"));
        out.push('\n');
    }
    out
}
