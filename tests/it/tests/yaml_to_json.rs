use std::{
    io::{self, Write},
    path::Path,
    process::{Command, Output, Stdio},
};

use gix_error::TestResult;
use serde_json::{Value, json};

fn convert(path: Option<&Path>, input: &[u8]) -> io::Result<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_it"));
    command.arg("yaml-to-json");
    if let Some(path) = path {
        command.arg(path);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().expect("stdin is piped").write_all(input)?;
    child.wait_with_output()
}

#[test]
fn workflow_yaml_from_file_or_stdin_preserves_its_meaning() -> TestResult {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workflow.yml");
    let yaml = include_bytes!("fixtures/workflow.yml").as_slice();
    for (path, input) in [
        (Some(fixture.as_path()), &b""[..]),
        (Some(Path::new("-")), yaml),
        (None, yaml),
    ] {
        let output = convert(path, input)?;
        assert!(
            output.status.success(),
            "conversion succeeds: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            actual["on"],
            json!(["push", "pull_request"]),
            "on is a YAML 1.2 string key"
        );
        let job = &actual["jobs"]["test"];
        assert_eq!(
            job["env"],
            json!({"ENABLED": true, "RETRIES": 3, "TEXT": "3"}),
            "scalar types are preserved"
        );
        assert_eq!(
            job["steps"][1]["env"], job["env"],
            "aliases resolve to the anchored value"
        );
        assert_eq!(
            job["steps"][1]["run"],
            "echo \"${{ github.ref }}\"
echo done
",
            "multiline scripts and GitHub expressions remain literal"
        );
        assert_eq!(
            job["steps"][0]["with"]["persist-credentials"], false,
            "policy booleans retain their type"
        );
    }
    Ok(())
}

#[test]
fn invalid_or_ambiguous_yaml_fails_without_json_output() -> TestResult {
    for input in [
        "jobs: [",
        "jobs: {}
jobs: {}
",
        "---
jobs: {}
---
jobs: {}
",
        "? [complex, key]
: value
",
        "1: value\n",
        "nested: [0, {value: .inf}]\n",
        "nested: [.nan]\n",
    ] {
        let output = convert(None, input.as_bytes())?;
        assert!(!output.status.success(), "invalid YAML must fail: {input}");
        assert!(output.stdout.is_empty(), "failed conversion must not emit partial JSON");
        assert!(!output.stderr.is_empty(), "failed conversion explains the error");
    }
    Ok(())
}
