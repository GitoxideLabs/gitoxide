use crate::{check_common, graph_and_expected, graph_and_expected_named};

#[test]
fn single_parent() {
    let (cg, refs) = graph_and_expected("single_parent.sh", &["parent", "child"]);
    check_common(&cg, &refs);

    assert_eq!(cg.commit_at(refs["parent"].pos()).generation(), 1);
    assert_eq!(cg.commit_at(refs["child"].pos()).generation(), 2);
}

#[test]
fn single_commit_huge_dates_generation_v2_also_do_not_allow_huge_dates() {
    let (cg, refs) = graph_and_expected_named("single_commit_huge_dates.sh", "v2", &["HEAD"]);
    let info = &refs["HEAD"];
    let actual = cg.commit_by_id(info.id).expect("present");
    assert_eq!(
        actual.committer_timestamp(),
        1,
        "overflow happened, can't represent huge dates"
    );
    assert_eq!(
        info.time.seconds, 68719476737,
        "this is the value we would want to see, but it's not possible in V2 either, as that is just about generations"
    );
    assert_eq!(actual.generation(), 1, "generations are fine though");
}

#[test]
fn single_commit_huge_dates_overflow_v1() {
    let (cg, refs) = graph_and_expected_named("single_commit_huge_dates.sh", "v1", &["HEAD"]);
    let info = &refs["HEAD"];
    let actual = cg.commit_by_id(info.id).expect("present");
    assert_eq!(actual.committer_timestamp(), 1, "overflow happened");
    assert_eq!(
        info.time.seconds, 68719476737,
        "this is the value we would want to see, but it's not possible in V1"
    );
    assert_eq!(actual.generation(), 1, "generations are fine though");
}

#[test]
fn single_commit_future_64bit_dates_work() {
    let (cg, refs) = graph_and_expected_named("single_commit_huge_dates.sh", "max-date", &["HEAD"]);
    let info = &refs["HEAD"];
    let actual = cg.commit_by_id(info.id).expect("present");
    assert_eq!(
        actual.committer_timestamp(),
        info.time.seconds.try_into().expect("timestamps in bound"),
        "this is close the highest representable value in the graph, like year 2500, so we are good for longer than I should care about"
    );
    assert_eq!(actual.generation(), 1);
}

#[test]
fn generation_numbers_overflow_is_handled_in_chained_graph() {
    let names = ["extra", "old-2", "future-2", "old-1", "future-1"];
    let (cg, mut refs) = graph_and_expected("generation_number_overflow.sh", &names);
    for (r, expected) in names
        .iter()
        .map(|n| refs.remove(n.to_owned()).expect("present"))
        .zip((1..=5).rev())
    {
        assert_eq!(
            cg.commit_by_id(r.id).expect("present").generation(),
            expected,
            "actually, this test seems to have valid generation numbers from the get-go. How to repro the actual issue?"
        );
    }
}

#[test]
fn octopus_merges() {
    let (cg, refs) = graph_and_expected(
        "octopus_merges.sh",
        &[
            "root",
            "parent1",
            "parent2",
            "parent3",
            "parent4",
            "three_parents",
            "four_parents",
        ],
    );
    check_common(&cg, &refs);

    assert_eq!(cg.commit_at(refs["root"].pos()).generation(), 1);
    assert_eq!(cg.commit_at(refs["parent1"].pos()).generation(), 2);
    assert_eq!(cg.commit_at(refs["parent2"].pos()).generation(), 2);
    assert_eq!(cg.commit_at(refs["parent3"].pos()).generation(), 2);
    assert_eq!(cg.commit_at(refs["parent4"].pos()).generation(), 2);
    assert_eq!(cg.commit_at(refs["three_parents"].pos()).generation(), 3);
    assert_eq!(cg.commit_at(refs["four_parents"].pos()).generation(), 3);
}

#[test]
fn single_commit() {
    let (cg, refs) = graph_and_expected("single_commit.sh", &["commit"]);
    check_common(&cg, &refs);

    assert_eq!(cg.commit_at(refs["commit"].pos()).generation(), 1);
}

#[test]
fn two_parents() {
    let (cg, refs) = graph_and_expected("two_parents.sh", &["parent1", "parent2", "child"]);
    check_common(&cg, &refs);

    assert_eq!(cg.commit_at(refs["parent1"].pos()).generation(), 1);
    assert_eq!(cg.commit_at(refs["parent2"].pos()).generation(), 1);
    assert_eq!(cg.commit_at(refs["child"].pos()).generation(), 2);
}
