use crate::hex_to_id;
use crate::util::{commit_graph, fixture, fixture_odb};
use gix_hash::{oid, ObjectId};
use gix_object::bstr::ByteSlice;
use gix_traverse::commit::{topo, Parents};
use std::path::PathBuf;

fn odb() -> crate::Result<gix_odb::Handle> {
    fixture_odb("make_repo_for_topo.sh")
}

fn fixture_dir() -> crate::Result<PathBuf> {
    fixture("make_repo_for_topo.sh")
}

/// Run a topo traversal with both commit-graph enabled and disabled to ensure consistency.
fn traverse_both(
    tips: impl IntoIterator<Item = ObjectId> + Clone,
    ends: impl IntoIterator<Item = ObjectId> + Clone,
    odb: &gix_odb::Handle,
    sorting: topo::Sorting,
    parents: Parents,
) -> crate::Result<Vec<ObjectId>> {
    // Without commit graph
    let without_graph: Vec<_> = topo::Builder::from_iters(odb, tips.clone(), Some(ends.clone()))
        .sorting(sorting)
        .with_commit_graph(None)
        .parents(parents)
        .build()?
        .map(|res| res.map(|info| info.id))
        .collect::<Result<Vec<_>, _>>()?;

    // With commit graph
    let graph = commit_graph(odb.store_ref());
    let with_graph: Vec<_> = topo::Builder::from_iters(odb, tips, Some(ends))
        .sorting(sorting)
        .with_commit_graph(graph)
        .parents(parents)
        .build()?
        .map(|res| res.map(|info| info.id))
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(
        without_graph, with_graph,
        "results must be consistent with and without commit-graph"
    );
    Ok(with_graph)
}

/// Run a topo traversal with a predicate filter.
fn traverse_with_predicate(
    tips: impl IntoIterator<Item = ObjectId> + Clone,
    ends: impl IntoIterator<Item = ObjectId> + Clone,
    odb: &gix_odb::Handle,
    sorting: topo::Sorting,
    parents: Parents,
    predicate: impl FnMut(&oid) -> bool + Clone,
) -> crate::Result<Vec<ObjectId>> {
    // Without commit graph
    let without_graph: Vec<_> = topo::Builder::from_iters(odb, tips.clone(), Some(ends.clone()))
        .sorting(sorting)
        .with_commit_graph(None)
        .parents(parents)
        .with_predicate(predicate.clone())
        .build()?
        .map(|res| res.map(|info| info.id))
        .collect::<Result<Vec<_>, _>>()?;

    // With commit graph
    let graph = commit_graph(odb.store_ref());
    let with_graph: Vec<_> = topo::Builder::from_iters(odb, tips, Some(ends))
        .sorting(sorting)
        .with_commit_graph(graph)
        .parents(parents)
        .with_predicate(predicate)
        .build()?
        .map(|res| res.map(|info| info.id))
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(
        without_graph, with_graph,
        "results must be consistent with and without commit-graph"
    );
    Ok(with_graph)
}

/// Read baseline file and parse expected commit hashes.
fn read_baseline(fixture_dir: &std::path::Path, name: &str) -> crate::Result<Vec<String>> {
    let buf = std::fs::read(fixture_dir.join(format!("{name}.baseline")))?;
    Ok(buf.lines().map(|s| s.to_str().unwrap().to_string()).collect())
}

mod basic {
    use super::*;

    #[test]
    fn simple() -> crate::Result {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "3be0c4c793c634c8fd95054345d4935d10a0879a",
            "2083b02a78e88b747e305b6ed3d5a861cf9fb73f",
            "302a5d0530ec688c241f32c2f2b61b964dd17bee",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "22fbc169eeca3c9678fc7028aa80fad5ef49019f",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
            "f1cce1b5c7efcdfa106e95caa6c45a2cae48a481",
            "945d8a360915631ad545e0cf04630d86d3d4eaa1",
            "a863c02247a6c5ba32dff5224459f52aa7f77f7b",
            "2f291881edfb0597493a52d26ea09dd7340ce507",
            "9c46b8765703273feb10a2ebd810e70b8e2ca44a",
            "fb3e21cf45b04b617011d2b30973f3e5ce60d0cd",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [], &odb, topo::Sorting::TopoOrder, Parents::All)?;
        assert_eq!(result, expected);

        // Verify against baseline
        let baseline = read_baseline(&fixture_dir()?, "all-commits")?;
        let expected_strs: Vec<_> = expected.iter().map(std::string::ToString::to_string).collect();
        assert_eq!(expected_strs, baseline, "Baseline must match the expectation");

        Ok(())
    }

    #[test]
    fn one_end() -> crate::Result {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");
        let end = hex_to_id("f1cce1b5c7efcdfa106e95caa6c45a2cae48a481");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "3be0c4c793c634c8fd95054345d4935d10a0879a",
            "2083b02a78e88b747e305b6ed3d5a861cf9fb73f",
            "302a5d0530ec688c241f32c2f2b61b964dd17bee",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "22fbc169eeca3c9678fc7028aa80fad5ef49019f",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [end], &odb, topo::Sorting::TopoOrder, Parents::All)?;
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn empty_range() -> crate::Result {
        let odb = odb()?;
        let tip = hex_to_id("f1cce1b5c7efcdfa106e95caa6c45a2cae48a481");
        let end = hex_to_id("eeab3243aad67bc838fc4425f759453bf0b47785");

        let result = traverse_both([tip], [end], &odb, topo::Sorting::TopoOrder, Parents::All)?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn two_tips_two_ends() -> crate::Result {
        let odb = odb()?;
        let tips = [
            hex_to_id("d09384f312b03e4a1413160739805ff25e8fe99d"),
            hex_to_id("3be0c4c793c634c8fd95054345d4935d10a0879a"),
        ];
        let ends = [
            hex_to_id("1a27cb1a26c9faed9f0d1975326fe51123ab01ed"),
            hex_to_id("22fbc169eeca3c9678fc7028aa80fad5ef49019f"),
        ];

        let expected = [
            "3be0c4c793c634c8fd95054345d4935d10a0879a",
            "2083b02a78e88b747e305b6ed3d5a861cf9fb73f",
            "302a5d0530ec688c241f32c2f2b61b964dd17bee",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
        ]
        .map(hex_to_id);

        let result = traverse_both(tips, ends, &odb, topo::Sorting::TopoOrder, Parents::All)?;
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn with_dummy_predicate() -> crate::Result {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");
        let filter_out = hex_to_id("eeab3243aad67bc838fc4425f759453bf0b47785");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "3be0c4c793c634c8fd95054345d4935d10a0879a",
            "2083b02a78e88b747e305b6ed3d5a861cf9fb73f",
            "302a5d0530ec688c241f32c2f2b61b964dd17bee",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "22fbc169eeca3c9678fc7028aa80fad5ef49019f",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
            "f1cce1b5c7efcdfa106e95caa6c45a2cae48a481",
            "945d8a360915631ad545e0cf04630d86d3d4eaa1",
            "a863c02247a6c5ba32dff5224459f52aa7f77f7b",
            "2f291881edfb0597493a52d26ea09dd7340ce507",
            "9c46b8765703273feb10a2ebd810e70b8e2ca44a",
            "fb3e21cf45b04b617011d2b30973f3e5ce60d0cd",
        ]
        .map(hex_to_id);

        let result = traverse_with_predicate([tip], [], &odb, topo::Sorting::TopoOrder, Parents::All, move |oid| {
            oid != filter_out
        })?;
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn end_along_first_parent() -> crate::Result {
        let odb = odb()?;
        let tip = hex_to_id("d09384f312b03e4a1413160739805ff25e8fe99d");
        let end = hex_to_id("33eb18340e4eaae3e3dcf80222b02f161cd3f966");

        let expected = [
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "22fbc169eeca3c9678fc7028aa80fad5ef49019f",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [end], &odb, topo::Sorting::TopoOrder, Parents::All)?;
        assert_eq!(result, expected);
        Ok(())
    }
}

mod first_parent {
    use super::*;

    #[test]
    fn basic() -> crate::Result {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
            "f1cce1b5c7efcdfa106e95caa6c45a2cae48a481",
            "945d8a360915631ad545e0cf04630d86d3d4eaa1",
            "a863c02247a6c5ba32dff5224459f52aa7f77f7b",
            "2f291881edfb0597493a52d26ea09dd7340ce507",
            "9c46b8765703273feb10a2ebd810e70b8e2ca44a",
            "fb3e21cf45b04b617011d2b30973f3e5ce60d0cd",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [], &odb, topo::Sorting::TopoOrder, Parents::First)?;
        assert_eq!(result, expected);

        // Verify against baseline
        let baseline = read_baseline(&fixture_dir()?, "first-parent")?;
        let expected_strs: Vec<_> = expected.iter().map(std::string::ToString::to_string).collect();
        assert_eq!(expected_strs, baseline, "Baseline must match the expectation");

        Ok(())
    }

    #[test]
    fn with_end() -> crate::Result {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");
        let end = hex_to_id("f1cce1b5c7efcdfa106e95caa6c45a2cae48a481");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [end], &odb, topo::Sorting::TopoOrder, Parents::First)?;
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn end_is_second_parent() -> crate::Result {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");
        let end = hex_to_id("3be0c4c793c634c8fd95054345d4935d10a0879a");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [end], &odb, topo::Sorting::TopoOrder, Parents::First)?;
        assert_eq!(result, expected);
        Ok(())
    }
}

mod date_order {
    use super::*;

    #[test]
    fn with_ends() -> crate::Result {
        let odb = odb()?;
        // Same tip and end as basic::one_end() but the order should be different.
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");
        let end = hex_to_id("f1cce1b5c7efcdfa106e95caa6c45a2cae48a481");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "3be0c4c793c634c8fd95054345d4935d10a0879a",
            "2083b02a78e88b747e305b6ed3d5a861cf9fb73f",
            "302a5d0530ec688c241f32c2f2b61b964dd17bee",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "22fbc169eeca3c9678fc7028aa80fad5ef49019f",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [end], &odb, topo::Sorting::DateOrder, Parents::All)?;
        assert_eq!(result, expected);

        // Verify against baseline
        let baseline = read_baseline(&fixture_dir()?, "date-order")?;
        let expected_strs: Vec<_> = expected.iter().map(std::string::ToString::to_string).collect();
        assert_eq!(expected_strs, baseline, "Baseline must match the expectation");

        Ok(())
    }
}
