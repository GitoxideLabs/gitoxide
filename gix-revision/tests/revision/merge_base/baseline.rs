use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use bstr::ByteSlice;
use gix_error::TestResult;
use gix_hash::ObjectId;
use nonempty::NonEmpty;

/// The expectation as produced by Git itself
#[derive(Debug)]
pub struct Expectation {
    pub plain_input: String,
    pub first: ObjectId,
    pub others: Vec<ObjectId>,
    pub ahead_behind: Option<gix_revision::ahead_behind_counts::Outcome>,
    pub bases: Option<NonEmpty<ObjectId>>,
}

pub fn parse_expectations(baseline: &Path) -> TestResult<Vec<Expectation>> {
    let lines = std::fs::read(baseline)?;
    let mut lines = lines.lines();
    let mut out = Vec::new();
    while let Some(plain_input) = lines.next() {
        let plain_input = plain_input.to_str_lossy().into_owned();
        let mut input = lines
            .next()
            .expect("second line is resolved input objects")
            .split(|b| *b == b' ');
        let first = ObjectId::from_hex(input.next().expect("at least one object"))?;
        let others: Vec<_> = input.map(ObjectId::from_hex).collect::<Result<_, _>>()?;
        let counts = lines.next().expect("third line is ahead/behind counts for pairs");
        let ahead_behind = if others.len() == 1 {
            let mut counts = counts.to_str()?.split_whitespace();
            let counts_result = gix_revision::ahead_behind_counts::Outcome {
                ahead: counts.next().expect("Git returns the left count").parse()?,
                behind: counts.next().expect("Git returns the right count").parse()?,
            };
            assert!(counts.next().is_none(), "Git returns exactly two counts");
            Some(counts_result)
        } else {
            assert!(counts.is_empty(), "counts are only recorded for pairs");
            None
        };
        let bases: Vec<_> = lines
            .by_ref()
            .take_while(|l| !l.is_empty())
            .map(ObjectId::from_hex)
            .collect::<Result<_, _>>()?;
        out.push(Expectation {
            plain_input,
            first,
            others,
            ahead_behind,
            bases: NonEmpty::from_vec(bases),
        });
    }
    Ok(out)
}

pub fn expectation_paths(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out: Vec<_> = std::fs::read_dir(root)?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|e| (e.path().extension() == Some(OsStr::new("baseline"))).then(|| e.path()))
        .collect();
    out.sort();
    Ok(out)
}
