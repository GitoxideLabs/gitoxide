use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

use gix_error::{OptionExt, Result, bail, message};
use serde_json::Value;

#[derive(Debug)]
pub(super) struct Selection {
    pub filter: String,
    pub selected: usize,
    pub total: usize,
}

pub(super) fn select(index: &Value, inventory: &Value, changed: &BTreeSet<String>) -> Result<Selection> {
    let mut files = BTreeMap::new();
    for (id, path) in index["files"]
        .as_array()
        .ok_or_raise(|| message("Coverage index files must be an array"))?
        .iter()
        .enumerate()
    {
        let path = path
            .as_str()
            .ok_or_raise(|| message("Coverage paths must be strings"))?;
        validate_path(path)?;
        if files.insert(path, id).is_some() {
            bail!("Duplicate coverage path: {path}");
        }
    }
    let mut changed_ids = BTreeSet::new();
    for path in changed {
        validate_path(path)?;
        if std::path::Path::new(path)
            .extension()
            .is_none_or(|extension| extension != "rs")
        {
            bail!("Coverage selection does not support changed non-Rust input: {path}");
        }
        changed_ids.insert(
            *files
                .get(path.as_str())
                .ok_or_raise(|| message!("Changed path has no coverage record: {path}"))?,
        );
    }

    let mut tests = BTreeMap::new();
    for test in index["tests"]
        .as_array()
        .ok_or_raise(|| message("Coverage index tests must be an array"))?
    {
        let binary = filter_name(test["binary"].as_str())?;
        let name = filter_name(test["name"].as_str())?;
        let mut coverage = BTreeSet::new();
        if let Some(ids) = test.get("files") {
            for id in ids
                .as_array()
                .ok_or_raise(|| message("Test coverage IDs must be an array"))?
            {
                let id = id
                    .as_u64()
                    .and_then(|id| usize::try_from(id).ok())
                    .filter(|id| *id < files.len())
                    .ok_or_raise(|| message("Invalid coverage file ID"))?;
                if !coverage.insert(id) {
                    bail!("Duplicate coverage file ID for {binary}::{name}");
                }
            }
        }
        if tests.insert((binary, name), coverage).is_some() {
            bail!("Duplicate coverage test: {binary}::{name}");
        }
    }

    let suites = inventory["rust-suites"]
        .as_object()
        .filter(|suites| !suites.is_empty())
        .ok_or_raise(|| message("Nextest inventory has no Rust test suites"))?;
    let mut result = Selection {
        filter: String::new(),
        selected: 0,
        total: 0,
    };
    let mut usable = false;
    for (id, suite) in suites {
        let binary = filter_name(suite["binary-id"].as_str())?;
        if id != binary {
            bail!("Inconsistent Nextest suite identity: {id}");
        }
        let cases = suite["testcases"]
            .as_object()
            .ok_or_raise(|| message("Nextest testcases must be an object"))?;
        match suite["status"].as_str() {
            Some("listed") => {}
            Some("skipped" | "skipped-default-filter") if cases.is_empty() => continue,
            _ => bail!("Unsupported Nextest suite listing status: {id}"),
        }
        for (name, test) in cases {
            filter_name(Some(name))?;
            let ignored = test["ignored"]
                .as_bool()
                .ok_or_raise(|| message("Nextest test is missing its ignored status"))?;
            if test["kind"] != "test"
                || !matches!(test["filter-match"]["status"].as_str(), Some("matches" | "mismatch"))
            {
                bail!("Unsupported Nextest test kind or filter status: {binary}::{name}");
            }
            if ignored || test["filter-match"]["status"] != "matches" {
                continue;
            }
            result.total += 1;
            let coverage = tests
                .get(&(binary, name.as_str()))
                .filter(|coverage| !coverage.is_empty());
            usable |= coverage.is_some();
            // Historical execution is a heuristic, not evidence about compile-time dependencies.
            if !changed.is_empty() && coverage.is_some_and(|coverage| coverage.is_disjoint(&changed_ids)) {
                continue;
            }
            if result.selected != 0 {
                result.filter.push_str(" | ");
            }
            write!(
                result.filter,
                "(binary_id(={}) & test(={}))",
                binary.replace('/', "\\/"),
                name.replace('/', "\\/")
            )
            .expect("writing to a String cannot fail");
            if result.filter.len() > 16 * 1024 {
                bail!("Coverage filter exceeds 16 KiB");
            }
            result.selected += 1;
        }
    }
    if result.total == 0 {
        bail!("Nextest inventory has no active tests");
    }
    if !usable {
        bail!("No active Nextest test has usable coverage");
    }
    if result.selected == 0 {
        result.filter.push_str("none()");
    }
    Ok(result)
}

fn validate_path(path: &str) -> Result<()> {
    if path.is_empty()
        || path
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b'\\' | b':'))
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        bail!("Coverage paths must be normalized workspace-relative paths: {path:?}");
    }
    Ok(())
}

fn filter_name(name: Option<&str>) -> Result<&str> {
    let name = name.ok_or_raise(|| message("Coverage test identities must be strings"))?;
    // Unsupported names retain the full run instead of risking Nextest expression injection.
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_:-./".contains(&byte))
    {
        bail!("Test identity cannot be safely expressed in a coverage filter: {name:?}");
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn inventory() -> Value {
        let test = json!({"kind":"test", "ignored":false, "filter-match":{"status":"matches"}});
        json!({"rust-suites":{
            "a::integration":{"binary-id":"a::integration", "status":"listed", "testcases":{"same":test}},
            "b::integration":{"binary-id":"b::integration", "status":"listed", "testcases":{"same":test}}
        }})
    }

    fn index() -> Value {
        json!({"files":["a/src/lib.rs", "b/src/lib.rs"], "tests":[
            {"binary":"a::integration", "name":"same", "files":[0]},
            {"binary":"b::integration", "name":"same", "files":[1]}
        ]})
    }

    fn changed() -> BTreeSet<String> {
        BTreeSet::from(["a/src/lib.rs".into()])
    }

    #[test]
    fn exact_binary_identity_separates_equal_test_names() -> gix_error::TestResult {
        let selected = select(&index(), &inventory(), &changed())?;
        assert_eq!((selected.selected, selected.total), (1, 2));
        assert_eq!(selected.filter, "(binary_id(=a::integration) & test(=same))");
        let all = select(&index(), &inventory(), &BTreeSet::new())?;
        assert_eq!(
            (all.selected, all.total),
            (2, 2),
            "an empty diff preserves policy scope"
        );
        Ok(())
    }

    #[test]
    fn new_tests_and_missing_or_empty_profiles_always_run() -> gix_error::TestResult {
        for records in [
            json!([{"binary":"b::integration", "name":"same", "files":[0]}]),
            json!([{"binary":"b::integration", "name":"same"}]),
            json!([{"binary":"b::integration", "name":"same", "files":[]}]),
        ] {
            let mut index = index();
            index["tests"] = records;
            index["tests"]
                .as_array_mut()
                .expect("records is an array")
                .push(json!({"binary":"a::integration", "name":"same", "files":[0]}));
            let mut inventory = inventory();
            inventory["rust-suites"]["a::integration"]["testcases"]["new"] =
                json!({"kind":"test", "ignored":false, "filter-match":{"status":"matches"}});
            assert_eq!(select(&index, &inventory, &changed())?.selected, 3);
        }
        Ok(())
    }

    #[test]
    fn unusable_map_retains_full_execution() {
        for tests in [
            json!([]),
            json!([{"binary":"a::integration", "name":"same", "files":[]}]),
        ] {
            let mut index = index();
            index["tests"] = tests;
            assert!(select(&index, &inventory(), &changed()).is_err());
        }
    }

    #[test]
    fn binary_target_separators_are_escaped() -> gix_error::TestResult {
        let mut inventory = inventory();
        let suite = inventory["rust-suites"]
            .as_object_mut()
            .expect("suites is an object")
            .remove("a::integration")
            .expect("suite exists");
        inventory["rust-suites"]["a::bin/test"] = suite;
        inventory["rust-suites"]["a::bin/test"]["binary-id"] = "a::bin/test".into();
        let mut index = index();
        index["tests"][0]["binary"] = "a::bin/test".into();
        assert_eq!(
            select(&index, &inventory, &changed())?.filter,
            "(binary_id(=a::bin\\/test) & test(=same))"
        );
        Ok(())
    }

    #[test]
    fn excludes_ignored_and_filtered_tests_and_can_select_none() -> gix_error::TestResult {
        let mut inventory = inventory();
        let tests = &mut inventory["rust-suites"]["a::integration"]["testcases"];
        tests["ignored"] = json!({"kind":"test", "ignored":true, "filter-match":{"status":"matches"}});
        tests["filtered"] = json!({"kind":"test", "ignored":false, "filter-match":{"status":"mismatch"}});
        assert_eq!(select(&index(), &inventory, &changed())?.total, 2);
        inventory["rust-suites"]["a::integration"]["testcases"]["same"]["ignored"] = true.into();
        let selected = select(&index(), &inventory, &changed())?;
        assert_eq!((selected.selected, selected.total), (0, 1));
        assert_eq!(selected.filter, "none()");
        Ok(())
    }

    #[test]
    fn filtered_binaries_are_excluded_but_an_empty_active_inventory_is_unsupported() -> gix_error::TestResult {
        for status in ["skipped", "skipped-default-filter"] {
            let mut inventory = inventory();
            inventory["rust-suites"]["b::integration"]["status"] = status.into();
            inventory["rust-suites"]["b::integration"]["testcases"] = json!({});
            assert_eq!(select(&index(), &inventory, &changed())?.total, 1);
            inventory["rust-suites"]["a::integration"]["testcases"]["same"]["ignored"] = true.into();
            assert!(select(&index(), &inventory, &changed()).is_err());
        }
        Ok(())
    }

    #[test]
    fn unknown_and_non_rust_changes_retain_full_execution() {
        for path in [
            "new.rs",
            "a/fixture",
            "Cargo.toml",
            "",
            "../a.rs",
            "/a.rs",
            "a//b.rs",
            "a/./b.rs",
            "C:\\a.rs",
        ] {
            assert!(
                select(&index(), &inventory(), &BTreeSet::from([path.into()])).is_err(),
                "{path:?} must fall back to the full case"
            );
        }
    }

    #[test]
    fn malformed_indexes_fail_closed() {
        for (pointer, value) in [
            ("/files", json!(null)),
            ("/files", json!(["a.rs", "a.rs"])),
            ("/files/0", json!("")),
            ("/files/0", json!("../a.rs")),
            ("/tests", json!(null)),
            ("/tests/0/binary", json!("")),
            ("/tests/0/name", json!(12)),
            ("/tests/0/files", json!([2])),
            ("/tests/0/files", json!([-1])),
            ("/tests/0/files", json!([0.5])),
            ("/tests/0/files", json!([0, 0])),
            ("/tests/0/files", json!(null)),
            (
                "/tests/1",
                json!({"binary":"a::integration", "name":"same", "files":[0]}),
            ),
        ] {
            let mut index = index();
            *index.pointer_mut(pointer).expect("the fixture has each tested field") = value;
            assert!(
                select(&index, &inventory(), &BTreeSet::new()).is_err(),
                "invalid {pointer}"
            );
        }
    }

    #[test]
    fn unsupported_inventory_and_names_fail_closed() {
        for inventory in [json!({}), json!({"rust-suites":{}}), json!({"rust-suites":[]})] {
            assert!(select(&index(), &inventory, &changed()).is_err());
        }
        for (pointer, value) in [
            ("/rust-suites/a::integration/binary-id", json!("b::integration")),
            ("/rust-suites/a::integration/status", json!("skipped")),
            ("/rust-suites/a::integration/testcases/same/ignored", json!(null)),
            ("/rust-suites/a::integration/testcases/same/kind", json!("unknown")),
            (
                "/rust-suites/a::integration/testcases/same/filter-match/status",
                json!("unknown"),
            ),
        ] {
            let mut inventory = inventory();
            *inventory
                .pointer_mut(pointer)
                .expect("the fixture has each tested field") = value;
            assert!(select(&index(), &inventory, &changed()).is_err(), "invalid {pointer}");
        }
        for name in [
            "",
            "has space",
            "a\"b",
            "a'b",
            "a\\b",
            "a)b",
            "a|all()",
            "a,b",
            "ümlaut",
        ] {
            let mut inventory = inventory();
            let tests = &mut inventory["rust-suites"]["a::integration"]["testcases"];
            let test = tests
                .as_object_mut()
                .expect("testcases is an object")
                .remove("same")
                .expect("same exists");
            tests[name] = test;
            assert!(
                select(&index(), &inventory, &changed()).is_err(),
                "unsupported {name:?}"
            );
        }
    }

    #[test]
    fn bounds_command_length() {
        let mut inventory = inventory();
        let tests = &mut inventory["rust-suites"]["a::integration"]["testcases"];
        tests["x".repeat(16 * 1024)] = json!({"kind":"test", "ignored":false, "filter-match":{"status":"matches"}});
        assert!(
            select(&index(), &inventory, &changed()).is_err(),
            "oversized filters retain full execution"
        );
    }
}
