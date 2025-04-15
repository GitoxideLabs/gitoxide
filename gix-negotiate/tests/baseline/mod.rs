use std::cell::RefCell;

use gix_negotiate::Algorithm;
use gix_object::{bstr, bstr::ByteSlice, FindExt};
use gix_ref::{file::ReferenceExt, store::WriteReflog};

#[test]
fn run() -> crate::Result {
    let root = gix_testtools::scripted_fixture_read_only("make_repos.sh")?;
    for case in [
        "no_parents",
        "clock_skew",
        "two_colliding_skips",
        "multi_round",
        "advertisement_as_filter",
    ] {
        let base = root.join(case);

        for (algo_name, algo) in [
            ("noop", Algorithm::Noop),
            ("consecutive", Algorithm::Consecutive),
            ("skipping", Algorithm::Skipping),
        ] {
            let obj_buf = RefCell::new(Vec::new());
            let buf = std::fs::read(base.join(format!("baseline.{algo_name}")))?;
            let store = gix_odb::at(base.join("client").join(".git/objects"))?;
            let refs = gix_ref::file::Store::at(
                base.join("client").join(".git"),
                gix_ref::store::init::Options {
                    write_reflog: WriteReflog::Disable,
                    ..Default::default()
                },
            );
            let lookup_names = |names: &[&str]| -> Vec<gix_hash::ObjectId> {
                names
                    .iter()
                    .filter_map(|name| {
                        refs.try_find(*name).expect("one tag per commit").map(|mut r| {
                            r.peel_to_id_in_place(&refs, &store).expect("works");
                            r.target.into_id()
                        })
                    })
                    .collect()
            };
            let message = |id: gix_hash::ObjectId| {
                store
                    .find_commit(&id, obj_buf.borrow_mut().as_mut())
                    .expect("present")
                    .message
                    .trim()
                    .as_bstr()
                    .to_owned()
            };

            let debug = false;
            for use_cache in [false, true] {
                let cache = use_cache
                    .then(|| gix_commitgraph::at(store.store_ref().path().join("info")).ok())
                    .flatten();
                let mut graph = gix_revwalk::Graph::new(&store, cache.as_ref());
                let mut negotiator = algo.into_negotiator();
                if debug {
                    eprintln!("ALGO {algo_name} CASE {case}");
                }
                // // In --negotiate-only mode, which seems to be the only thing that's working after trying --dry-run, we unfortunately
                // // don't get to see what happens if known-common commits are added as git itself doesn't do that in this mode
                // // for some reason.
                // for common in lookup_names(&["origin/main"]) {
                //     eprintln!("COMMON {name} {common}", name = message(common));
                //     negotiator.known_common(common)?;
                // }
                for tip in lookup_names(&["HEAD"]).into_iter().chain(
                    refs.iter()?
                        .prefixed(b"refs/heads".try_into().unwrap())?
                        .filter_map(Result::ok)
                        .map(|r| r.target.into_id()),
                ) {
                    if debug {
                        eprintln!("TIP {name} {tip}", name = message(tip));
                    }
                    negotiator.add_tip(tip, &mut graph)?;
                }
                for (round, Round { mut haves, common }) in ParseRounds::new(buf.lines()).enumerate() {
                    if algo == Algorithm::Skipping {
                        if case == "clock_skew" {
                            // Here for some reason the prio-queue of git manages to not sort the parent of C2, which is in the future, to be
                            // ahead of old4 that is in the past. In the git version of this test, they say to expect exactly this sequence
                            // as well even though it's not actually happening (but that they can't see due to the way they are testing).
                            haves = lookup_names(&["c2", "c1", "old4", "old2", "old1"]);
                        } else if case == "two_colliding_skips" {
                            // The same thing, we actually get exactly the right order, whereas git for some reason doesn't.
                            // This is the order expected in the git tests.
                            haves = lookup_names(&["c5side", "c11", "c9", "c6", "c1"]);
                        } else if case == "multi_round" && round == 1 {
                            // Here, probably also because of priority queue quirks, `git` emits the commits out of order, with only one
                            // branch, b5 I think, being out of place. This list puts the expectation in the right order, which is ordered
                            // by commit date.
                            haves = lookup_names(&[
                                "b8.c14", "b7.c14", "b6.c14", "b5.c14", "b4.c14", "b3.c14", "b2.c14", "b8.c9", "b7.c9",
                                "b6.c9", "b5.c9", "b4.c9", "b3.c9", "b2.c9", "b8.c1", "b7.c1", "b6.c1", "b5.c1",
                                "b4.c1", "b3.c1", "b2.c1", "b8.c0", "b7.c0", "b6.c0", "b5.c0", "b4.c0", "b3.c0",
                                "b2.c0",
                            ]);
                        } else if case == "advertisement_as_filter" {
                            haves = lookup_names(&["c2side", "c5", "origin/main"])
                                .into_iter()
                                .chain(Some(
                                    gix_hash::ObjectId::from_hex(b"f36cefa0be2ac180d360a54b1cc4214985cea60a").unwrap(),
                                ))
                                .collect();
                        }
                    }
                    for have in haves {
                        let actual = negotiator.next_have(&mut graph).unwrap_or_else(|| {
                            panic!("{algo_name}:cache={use_cache}: one have per baseline: {have} missing or in wrong order", have = message(have))
                        })?;
                        assert_eq!(
                            actual,
                            have,
                            "{algo_name}:cache={use_cache}: order and commit matches exactly, wanted {expected}, got {actual}, commits left: {:?}",
                            std::iter::from_fn(|| negotiator.next_have(&mut graph)).map(|id| message(id.unwrap())).collect::<Vec<_>>(),
                            actual = message(actual),
                            expected = message(have)
                        );
                        if debug {
                            eprintln!("have {}", message(actual));
                        }
                    }
                    for common_revision in common {
                        if debug {
                            eprintln!("ACK {}", message(common_revision));
                        }
                        negotiator.in_common_with_remote(common_revision, &mut graph)?;
                    }
                }
                assert!(
                    negotiator.next_have(&mut graph).is_none(),
                    "{algo_name}:cache={use_cache}: negotiator should be depleted after all recorded baseline rounds"
                );
            }
        }
    }
    Ok(())
}

struct ParseRounds<'a> {
    lines: bstr::Lines<'a>,
}

impl<'a> ParseRounds<'a> {
    pub fn new(mut lines: bstr::Lines<'a>) -> Self {
        parse::command(&mut lines, parse::Command::Incoming).expect("handshake");
        Self { lines }
    }
}

impl Iterator for ParseRounds<'_> {
    type Item = Round;

    fn next(&mut self) -> Option<Self::Item> {
        let haves = parse::object_ids("have", parse::command(&mut self.lines, parse::Command::Outgoing)?);
        let common = parse::object_ids("ACK", parse::command(&mut self.lines, parse::Command::Incoming)?);
        if haves.is_empty() {
            assert!(common.is_empty(), "cannot ack what's not there");
            return None;
        }
        Round { haves, common }.into()
    }
}

struct Round {
    pub haves: Vec<gix_hash::ObjectId>,
    pub common: Vec<gix_hash::ObjectId>,
}

mod parse {
    use gix_object::{
        bstr,
        bstr::{BStr, ByteSlice},
    };

    #[derive(Debug, Eq, PartialEq, Copy, Clone)]
    pub enum Command {
        Incoming,
        Outgoing,
    }

    pub fn object_ids(prefix: &str, lines: impl IntoIterator<Item = impl AsRef<[u8]>>) -> Vec<gix_hash::ObjectId> {
        lines
            .into_iter()
            .filter_map(|line| {
                line.as_ref()
                    .strip_prefix(prefix.as_bytes())
                    .map(|id| gix_hash::ObjectId::from_hex(id.trim()).expect("valid hash"))
            })
            .collect()
    }

    pub fn command<'a>(lines: &mut bstr::Lines<'a>, wanted: Command) -> Option<Vec<&'a BStr>> {
        let mut out = Vec::new();
        for line in lines {
            let pos = line.find(b"fetch").expect("fetch token");
            let line_mode = match &line[pos + 5..][..2] {
                b"< " => Command::Incoming,
                b"> " => Command::Outgoing,
                invalid => unreachable!("invalid fetch token: {:?}", invalid.as_bstr()),
            };
            assert_eq!(line_mode, wanted, "command with unexpected mode");
            let line = line[pos + 7..].as_bstr();
            if line == "0000" {
                break;
            }
            out.push(line);
        }
        (!out.is_empty()).then_some(out)
    }
}
