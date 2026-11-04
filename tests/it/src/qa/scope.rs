use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use gix_error::Result;
use serde_json::{Value, json};

use super::run::{self, Command};

const MAX_EXCLUSION_BYTES: usize = 12 * 1024;

pub fn prune(root: &Path, metadata: &Value, selection: &mut Value) -> Result<()> {
    if selection["mode"] != "fast" {
        return Ok(());
    }
    let Some(changed) = selection["changed_packages"].as_array() else {
        return Ok(());
    };
    let changed: BTreeSet<_> = changed.iter().filter_map(Value::as_str).collect();
    let mut commands = run::commands(
        metadata,
        selection,
        &["feature-checks", "variants", "isolated-tests", "doctest-variants"].map(String::from),
    )?;
    let selected = selection["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    commands.extend(super::ops::wasm_commands(metadata, &selected)?);
    let unaffected = unaffected_dependencies(commands, &changed, |command| {
        super::ops::output(&mut run::process(root, &run::normalize(command))?)
    });
    if !unaffected.is_empty()
        && let Some(reasons) = selection["reasons"].as_array_mut()
    {
        reasons.push(
            format!(
                "Excluded {} dependency configurations without any changed package",
                unaffected.len()
            )
            .into(),
        );
    }
    selection["unaffected_dependencies"] = unaffected.into();
    Ok(())
}

pub fn is_unaffected(selection: &Value, command: &Command) -> bool {
    let Some((_, _, key)) = tree_command(command) else {
        return false;
    };
    selection["unaffected_dependencies"]
        .as_array()
        .is_some_and(|commands| commands.contains(&key))
}

fn unaffected_dependencies(
    commands: Vec<Command>,
    changed: &BTreeSet<&str>,
    mut query: impl FnMut(Command) -> Result<Vec<u8>>,
) -> Vec<Value> {
    let mut unaffected = Vec::new();
    let mut bytes = 2;
    let mut queries = BTreeMap::new();
    for command in commands {
        let Some((tree, packages, key)) = tree_command(&command) else {
            continue;
        };
        if packages.iter().any(|package| changed.contains(package.as_str())) {
            continue;
        }
        let size = key.to_string().len() + usize::from(!unaffected.is_empty());
        // Retain excess cases so exclusions cannot overflow Windows' CI plan environment variable.
        if bytes + size > MAX_EXCLUSION_BYTES || unaffected.contains(&key) {
            continue;
        }
        // Target variants share the all-target query; failed queries retain conservative coverage.
        let excluded = queries
            .entry((tree.args.clone(), tree.env.clone()))
            .or_insert_with(|| query(tree).is_ok_and(|output| tree_is_unaffected(&output, &packages, changed)));
        if *excluded {
            bytes += size;
            unaffected.push(key);
        }
    }
    unaffected
}

fn tree_command(command: &Command) -> Option<(Command, Vec<String>, Value)> {
    if command.program != "cargo" {
        return None;
    }
    let (start, mut dev) = match command.args.first()?.as_str() {
        "check" | "build" => (1, false),
        "test" => (1, true),
        "nextest" if command.args.get(1).is_some_and(|arg| arg == "run") => (2, true),
        _ => return None,
    };
    let mut packages = Vec::new();
    let mut selection = Vec::new();
    let mut args = command.args[start..].iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--locked" | "--no-fail-fast" | "--doc" => {}
            "-p" | "--package" => {
                let package = args.next()?;
                packages.push(package.clone());
                selection.extend([arg.clone(), package.clone()]);
            }
            "--features" | "-F" => selection.extend([arg.clone(), args.next()?.clone()]),
            "--no-default-features" | "--all-features" => selection.push(arg.clone()),
            "--lib" | "--bins" => {}
            "--target" => {
                args.next()?;
            }
            "--tests" | "--all-targets" | "--examples" | "--benches" => dev = true,
            "--test" | "--example" | "--bench" => {
                args.next()?;
                dev = true;
            }
            "-E" | "--filter-expr" => {
                args.next()?;
            }
            _ => return None,
        }
    }
    if packages.is_empty() {
        return None;
    }
    let mut tree = Command::new(
        "cargo",
        &[
            "tree",
            "--locked",
            "--offline",
            "--target",
            "all",
            "--prefix",
            "none",
            "--format",
            "{p}",
            "--color",
            "never",
            "--edges",
            if dev { "normal,build,dev" } else { "normal,build" },
        ],
    );
    let mut key = json!({"args": selection, "dev": dev});
    if !command.env.is_empty() {
        key["env"] = json!(command.env);
    }
    tree.args.extend(selection);
    tree.env = command.env.clone();
    Some((tree, packages, key))
}

fn tree_is_unaffected(output: &[u8], packages: &[String], changed: &BTreeSet<&str>) -> bool {
    let Ok(output) = std::str::from_utf8(output) else {
        return false;
    };
    let names: BTreeSet<_> = output
        .lines()
        .filter_map(|line| line.split_ascii_whitespace().next())
        .collect();
    packages.iter().all(|package| names.contains(package.as_str())) && changed.is_disjoint(&names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gix_error::{ResultExt, message};

    fn key(command: &Command) -> Value {
        tree_command(command).expect("known QA selection").2
    }

    #[test]
    fn tree_queries_retain_features_packages_environment_and_relevant_edges() -> gix_error::TestResult {
        for (args, edges) in [
            (
                vec![
                    "check",
                    "--locked",
                    "-p",
                    "sample",
                    "--no-default-features",
                    "--features",
                    "optional",
                ],
                "normal,build",
            ),
            (vec!["check", "--locked", "-p", "sample", "--tests"], "normal,build,dev"),
            (vec!["test", "--doc", "--locked", "-p", "sample"], "normal,build,dev"),
            (
                vec![
                    "nextest",
                    "run",
                    "--locked",
                    "--no-fail-fast",
                    "-p",
                    "sample",
                    "--test",
                    "integration",
                    "-E",
                    "test(name)",
                ],
                "normal,build,dev",
            ),
        ] {
            let mut command = Command::new("cargo", &args);
            command.env.insert("GIX_TEST_FIXTURE_HASH".into(), "sha256".into());
            let (tree, packages, _) = tree_command(&command).expect("known QA arguments can be queried");
            assert_eq!(packages, ["sample"], "queries retain the package selection");
            assert_eq!(tree.env, command.env, "case environment reaches Cargo unchanged");
            assert!(
                tree.args.windows(2).any(|args| args == ["--edges", edges]),
                "test targets need dev dependency features"
            );
            assert!(
                tree.args.windows(2).any(|args| args == ["--target", "all"]),
                "one selection remains conservative on every CI target"
            );
            if args.contains(&"--features") {
                assert!(
                    tree.args
                        .ends_with(&["--no-default-features".into(), "--features".into(), "optional".into()]),
                    "Cargo resolves the declared feature combination"
                );
            }
        }
        for args in [
            vec!["check", "-p"],
            vec!["check", "-p", "sample", "--config", "option"],
            vec!["check", "--workspace"],
            vec!["build", "--target"],
        ] {
            assert!(
                tree_command(&Command::new("cargo", &args)).is_none(),
                "unknown selections must retain their checks: {args:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn only_successful_unaffected_queries_can_omit_a_configuration() -> gix_error::TestResult {
        let commands = ["changed", "affected", "unaffected", "failed", "unexpected"]
            .map(|name| Command::new("cargo", &["check", "--locked", "-p", name]));
        let changed = BTreeSet::from(["changed"]);
        let unaffected = unaffected_dependencies(commands.to_vec(), &changed, |command| {
            let name = command.args.last().expect("queries end in the selected package");
            match name.as_str() {
                "changed" => panic!("directly changed roots must run without needing a query"),
                "affected" => Ok(b"affected v1.0.0
changed v1.0.0
"
                .to_vec()),
                "unaffected" => Ok(b"unaffected v1.0.0
other v1.0.0
"
                .to_vec()),
                "failed" => Err(std::io::Error::other("unavailable registry")).or_raise(|| message("query failed")),
                _ => Ok(Vec::new()),
            }
        });
        assert_eq!(
            unaffected,
            [key(&commands[2])],
            "failed, affected, malformed and directly changed cases remain selected"
        );
        let selection = json!({"unaffected_dependencies": unaffected});
        assert!(is_unaffected(&selection, &commands[2]));
        let mut changed_env = commands[2].clone();
        changed_env
            .env
            .insert("CARGO_BUILD_TARGET".into(), "another-target".into());
        assert!(
            !is_unaffected(&selection, &changed_env),
            "a saved exclusion matches the entire command environment"
        );
        let mut changed_features = commands[2].clone();
        changed_features.args.extend(["--features", "extra"].map(String::from));
        assert!(
            !is_unaffected(&selection, &changed_features),
            "different features need their own dependency proof"
        );
        assert!(
            !is_unaffected(
                &json!({"unaffected_dependencies": [{"args": ["-p", "unaffected"]}]}),
                &commands[2]
            ),
            "incomplete saved dependency selections cannot omit work"
        );
        assert!(
            !is_unaffected(&json!({}), &commands[2]),
            "older saved plans retain every selected case"
        );
        assert!(
            !tree_is_unaffected(b"\xff", &["unaffected".into()], &changed),
            "invalid Cargo output cannot omit work"
        );
        Ok(())
    }

    #[test]
    fn oversized_exclusions_keep_checks_and_the_ci_payload_bounded() -> gix_error::TestResult {
        let changed = BTreeSet::from(["changed"]);
        let mut command = Command::new("cargo", &["check", "-p", "sample"]);
        command.env.insert("CASE_INPUT".into(), "x".repeat(MAX_EXCLUSION_BYTES));
        assert!(
            unaffected_dependencies(vec![command], &changed, |_| {
                panic!("oversized exclusions retain the check without querying Cargo")
            })
            .is_empty()
        );
        let commands = (0..100)
            .map(|index| {
                let mut command = Command::new("cargo", &["check", "-p", &format!("package-{index}")]);
                command.env.insert("CASE_INPUT".into(), "x".repeat(200));
                command
            })
            .collect();
        let unaffected = unaffected_dependencies(commands, &changed, |command| {
            let package = command.args.last().expect("queries end with their selected package");
            Ok(format!("{package} v0.0.0\n").into_bytes())
        });
        assert!(
            !unaffected.is_empty() && unaffected.len() < 100,
            "the bounded prefix is omitted and excess cases still run"
        );
        assert!(
            serde_json::to_vec(&unaffected)?.len() <= MAX_EXCLUSION_BYTES,
            "serialized exclusions stay within the reserved CI payload budget"
        );
        Ok(())
    }

    #[test]
    fn target_variants_reuse_the_same_dependency_query() -> gix_error::TestResult {
        let commands = ["wasm32-unknown-unknown", "wasm32-wasip1", "wasm32-wasip2"]
            .map(|target| Command::new("cargo", &["build", "--locked", "--target", target, "-p", "sample"]))
            .to_vec();
        let mut queries = 0;
        let unaffected = unaffected_dependencies(commands.clone(), &BTreeSet::from(["changed"]), |command| {
            queries += 1;
            assert!(
                command.args.windows(2).any(|args| args == ["--target", "all"]),
                "one query includes all platform dependencies"
            );
            Ok(b"sample v0.0.0\n".to_vec())
        });
        assert_eq!(
            queries, 1,
            "WASM target variants differ in execution but share their dependency query"
        );
        assert_eq!(
            unaffected,
            [key(&commands[0])],
            "one dependency exclusion covers all three target commands"
        );
        let selection = json!({"unaffected_dependencies": unaffected});
        assert!(
            commands.iter().all(|command| is_unaffected(&selection, command)),
            "all-target dependency proofs apply to every WASM target"
        );
        queries = 0;
        let unaffected = unaffected_dependencies(commands, &BTreeSet::from(["changed"]), |_| {
            queries += 1;
            Ok(b"sample v0.0.0
changed v0.0.0
"
            .to_vec())
        });
        assert!(unaffected.is_empty(), "affected targets keep every build");
        assert_eq!(queries, 1, "affected target variants reuse the successful query too");
        Ok(())
    }

    #[test]
    fn cargo_resolves_optional_aliases_and_dev_self_dependency_features() -> gix_error::TestResult {
        let workspace = gix_testtools::scripted_fixture_read_only("make_qa_scope_workspace.sh")?;
        let query = |command| super::super::ops::output(&mut run::process(&workspace, &run::normalize(command))?);
        let changed = BTreeSet::from(["changed"]);
        for (args, unaffected) in [
            (vec!["check", "-p", "app", "--no-default-features"], true),
            (
                vec!["check", "-p", "app", "--no-default-features", "--features", "weak"],
                true,
            ),
            (
                vec!["check", "-p", "app", "--no-default-features", "--features", "archive"],
                false,
            ),
            (vec!["check", "-p", "app", "--no-default-features", "--tests"], false),
            (vec!["build", "-p", "platform", "--target", "wasm32-wasip1"], false),
        ] {
            let (tree, packages, _) = tree_command(&Command::new("cargo", &args)).expect("known check arguments");
            assert_eq!(
                tree_is_unaffected(&query(tree)?, &packages, &changed),
                unaffected,
                "Cargo resolves renamed, weak, and dev-activated optional dependencies for {args:?}"
            );
        }
        Ok(())
    }
}
