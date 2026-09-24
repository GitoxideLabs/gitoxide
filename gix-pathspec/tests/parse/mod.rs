use crate::Result;
use std::collections::HashMap;

use bstr::{BStr, BString, ByteSlice};
use gix_attributes::State;
use gix_pathspec::{MagicSignature, Pattern, SearchMode};
use std::sync::LazyLock;

#[test]
fn baseline() {
    for (pattern, exit_code) in BASELINE.iter() {
        let res = gix_pathspec::parse(pattern, Default::default());
        assert_eq!(
            res.is_ok(),
            *exit_code == 0,
            "{pattern:?} disagrees with baseline: {res:?}"
        );
        if let Ok(pat) = res {
            let actual = pat.to_bstring();
            assert_eq!(
                pat,
                gix_pathspec::parse(actual.as_ref(), Default::default()).expect("still valid"),
                "{pattern} != {actual}: display must roundtrip into actual pattern"
            );
        }
        let p = gix_pathspec::Pattern::from_literal(pattern, Default::default());
        assert!(matches!(p.search_mode, SearchMode::Literal));
    }
}

mod invalid;
mod valid;

/// A way to specify expectations more easily by simplifying assignments.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedPattern {
    path: BString,
    signature: MagicSignature,
    search_mode: SearchMode,
    attributes: Vec<(BString, State)>,
}

impl From<Pattern> for NormalizedPattern {
    fn from(p: Pattern) -> Self {
        NormalizedPattern {
            path: p.path().to_owned(),
            signature: p.signature,
            search_mode: p.search_mode,
            attributes: p
                .attributes
                .into_iter()
                .map(|attr| (attr.name.as_str().into(), attr.state))
                .collect(),
        }
    }
}

static BASELINE: LazyLock<HashMap<BString, usize>> = LazyLock::new(|| {
    let base = gix_testtools::scripted_fixture_read_only("parse_baseline.sh").unwrap();

    (|| -> Result<_> {
        let mut map = HashMap::new();
        let baseline = std::fs::read(base.join("baseline.git"))?;
        let mut lines = baseline.lines();
        while let Some(spec) = lines.next() {
            let exit_code = lines.next().expect("two lines per baseline").to_str()?.parse()?;
            map.insert(spec.into(), exit_code);
        }
        Ok(map)
    })()
    .unwrap()
});

fn check_valid_inputs<'a>(inputs: impl IntoIterator<Item = (&'a str, NormalizedPattern)>) {
    for (input, expected) in inputs.into_iter() {
        assert!(
            check_against_baseline(input),
            "This pathspec is invalid in git: {input}"
        );

        let pattern = gix_pathspec::parse(input.as_bytes(), Default::default())
            .unwrap_or_else(|_| panic!("parsing should not fail with pathspec {input}"));
        let pattern: NormalizedPattern = pattern.into();
        assert_eq!(pattern, expected, "while checking input: \"{input}\"");
    }
}

fn check_against_baseline(pathspec: &str) -> bool {
    let key: &BStr = pathspec.into();
    let base = BASELINE
        .get(key)
        .unwrap_or_else(|| panic!("missing baseline for pathspec: {pathspec:?}"));
    *base == 0
}
