use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use gix_error::{OptionExt, Result, ResultExt, bail, message};
use serde_json::{Value, json};

use super::super::ops::output;

pub(super) struct Tools {
    pub compiler: String,
    profdata: PathBuf,
    cov: PathBuf,
}

pub(super) fn discover(toolchain: &str) -> Result<Tools> {
    let rustc = |args: &[&str]| {
        output(
            Command::new("rustup")
                .args(["run", toolchain, "rustc"])
                .args(args)
                .env("RUSTUP_AUTO_INSTALL", "0"),
        )
    };
    let compiler = String::from_utf8(rustc(&["-vV"])?).or_error()?;
    let host = compiler
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or_raise(|| message("rustc did not report its host target"))?;
    let sysroot = String::from_utf8(rustc(&["--print", "sysroot"])?).or_error()?;
    let directory = Path::new(sysroot.trim()).join("lib/rustlib").join(host).join("bin");
    let profdata = directory.join(format!("llvm-profdata{}", std::env::consts::EXE_SUFFIX));
    let cov = directory.join(format!("llvm-cov{}", std::env::consts::EXE_SUFFIX));
    if !profdata.is_file() || !cov.is_file() {
        bail!("Coverage requires `rustup component add llvm-tools-preview --toolchain {toolchain}`");
    }
    Ok(Tools {
        compiler,
        profdata,
        cov,
    })
}

impl Tools {
    pub(super) fn index(&self, root: &Path, inventory: &Value, raw: &Path) -> Result<Value> {
        let root = root.canonicalize().or_error()?;
        let suites = inventory["rust-suites"]
            .as_object()
            .ok_or_raise(|| message("Nextest inventory has no test suites"))?;
        let mut captures: BTreeMap<(String, String), Vec<PathBuf>> = BTreeMap::new();
        for entry in fs::read_dir(raw).or_error()? {
            let directory = entry.or_error()?.path();
            // A successful wrapper writes this last. Partial or uninstrumented tests stay unrecorded.
            if !directory.join("complete").is_file() {
                continue;
            }
            let identity: Value = serde_json::from_slice(&fs::read(directory.join("identity.json")).or_error()?)
                .or_raise(|| message("Read per-test coverage identity"))?;
            let binary = string(&identity, "binary")?;
            let name = string(&identity, "name")?;
            let suite = suites
                .get(binary)
                .ok_or_raise(|| message!("Coverage names an unknown binary: {binary}"))?;
            if !suite["testcases"]
                .as_object()
                .is_some_and(|tests| tests.contains_key(name))
            {
                bail!("Coverage names an unknown test: {binary}::{name}");
            }
            let mut profiles = Vec::new();
            for entry in fs::read_dir(&directory).or_error()? {
                let path = entry.or_error()?.path();
                if path.extension().is_some_and(|extension| extension == "profraw") {
                    if fs::metadata(&path).or_error()?.len() == 0 {
                        bail!("Empty LLVM coverage profile: {}", path.display());
                    }
                    profiles.push(path);
                }
            }
            if !profiles.is_empty() {
                captures
                    .entry((binary.into(), name.into()))
                    .or_default()
                    .extend(profiles);
            }
        }
        if captures.is_empty() {
            bail!("No complete per-test LLVM profiles were captured");
        }

        let analysis = raw.join("analysis");
        fs::create_dir_all(&analysis).or_error()?;
        let mut binaries: BTreeMap<String, Vec<(String, PathBuf, BTreeSet<String>)>> = BTreeMap::new();
        for (index, ((binary, name), profiles)) in captures.into_iter().enumerate() {
            let profile = if let [profile] = profiles.as_slice() {
                profile.clone()
            } else {
                let merged = analysis.join(format!("test-{index}.profdata"));
                self.merge(&profiles, &merged)?;
                merged
            };
            // Instrumentation profiles do not support llvm-profdata's JSON output.
            let names = String::from_utf8(output(
                Command::new(&self.profdata).args(["show", "--covered"]).arg(&profile),
            )?)
            .or_error()?
            .lines()
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect();
            binaries.entry(binary).or_default().push((name, profile, names));
        }

        let mut recorded = Vec::new();
        let mut files = BTreeSet::new();
        for (index, (binary, tests)) in binaries.into_iter().enumerate() {
            let merged = analysis.join(format!("binary-{index}.profdata"));
            // Export includes unexecuted functions, so one profile supplies the complete binary map.
            // llvm-cov needs an indexed profile even when llvm-profdata can read the raw capture.
            self.merge(std::slice::from_ref(&tests[0].1), &merged)?;
            let executable = string(&suites[&binary], "binary-path")?;
            let export: Value = serde_json::from_slice(&output(
                Command::new(&self.cov)
                    .arg("export")
                    .arg(executable)
                    .arg("-instr-profile")
                    .arg(&merged),
            )?)
            .or_raise(|| message("Read LLVM's function coverage map"))?;
            // Scope names to this binary and union duplicate entries and macro source files.
            // This can select extra tests; it avoids exporting the same large map once per test.
            let functions = function_files(&root, &export)?;
            // Keep mapped but unexecuted sources distinct from new or otherwise unknown files.
            files.extend(functions.values().flat_map(|files| files.iter()).cloned());
            for (name, _, names) in tests {
                let covered = covered_files(&functions, &names).unwrap_or_else(|| {
                    eprintln!("Coverage: unmapped functions in {binary}::{name}; this test will always run");
                    BTreeSet::new()
                });
                recorded.push((binary.clone(), name, covered));
            }
        }
        if recorded.iter().all(|(_, _, covered)| covered.is_empty()) {
            bail!("LLVM profiles contain no covered workspace source files");
        }
        let files: Vec<_> = files.into_iter().collect();
        let tests: Vec<_> = recorded
            .into_iter()
            .map(|(binary, name, covered)| {
                let indices: Vec<_> = covered
                    .iter()
                    .map(|path| {
                        files
                            .binary_search(path)
                            .expect("covered files are in the shared dictionary")
                    })
                    .collect();
                json!({"binary":binary,"name":name,"files":indices})
            })
            .collect();
        Ok(json!({"files":files,"tests":tests}))
    }

    fn merge(&self, profiles: &[PathBuf], destination: &Path) -> Result<()> {
        let mut inputs = String::new();
        for path in profiles {
            let path = path
                .to_str()
                .ok_or_raise(|| message("LLVM profile path must be UTF-8"))?;
            if path.contains(['\n', '\r']) {
                bail!("LLVM profile paths cannot contain newlines");
            }
            writeln!(inputs, "1,{path}").expect("writing to a String cannot fail");
        }
        // A large test binary can exceed the operating system's argument-length limit.
        let list = destination.with_extension("inputs");
        fs::write(&list, inputs).or_error()?;
        output(
            Command::new(&self.profdata)
                .args(["merge", "-sparse"])
                .arg("--input-files")
                .arg(list)
                .arg("-o")
                .arg(destination),
        )?;
        Ok(())
    }
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_raise(|| message!("Coverage metadata is missing string field {key}"))
}

fn covered_files(functions: &BTreeMap<String, BTreeSet<String>>, names: &BTreeSet<String>) -> Option<BTreeSet<String>> {
    let mut covered = BTreeSet::new();
    for name in names {
        // Child executables can add functions absent from this binary; partial coverage cannot prune safely.
        covered.extend(functions.get(name)?.iter().cloned());
    }
    Some(covered)
}

fn function_files(root: &Path, export: &Value) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let data = export["data"]
        .as_array()
        .ok_or_raise(|| message("LLVM export has no data array"))?;
    let mut functions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut sources = BTreeMap::new();
    for unit in data {
        for function in unit["functions"]
            .as_array()
            .ok_or_raise(|| message("LLVM export has no functions array"))?
        {
            let files = functions.entry(string(function, "name")?.into()).or_default();
            for filename in function["filenames"]
                .as_array()
                .ok_or_raise(|| message("LLVM function has no filenames array"))?
            {
                let filename = filename
                    .as_str()
                    .ok_or_raise(|| message("LLVM filename is not a string"))?;
                if !sources.contains_key(filename) {
                    sources.insert(filename.to_owned(), relative_source(root, filename)?);
                }
                if let Some(path) = &sources[filename] {
                    files.insert(path.clone());
                }
            }
        }
    }
    Ok(functions)
}

fn relative_source(root: &Path, filename: &str) -> Result<Option<String>> {
    if filename.is_empty() {
        bail!("LLVM coverage source path is empty");
    }
    let path = root.join(filename);
    if !path.starts_with(root) {
        return Ok(None);
    }
    let path = path
        .canonicalize()
        .or_raise(|| message!("Resolve coverage source {}", path.display()))?;
    let Ok(relative) = path.strip_prefix(root) else {
        return Ok(None);
    };
    Ok(Some(
        relative
            .to_str()
            .ok_or_raise(|| message("Coverage source path must be UTF-8"))?
            .replace('\\', "/"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmapped_child_functions_invalidate_the_entire_test_profile() {
        let files = BTreeSet::from(["src/lib.rs".into()]);
        let functions = BTreeMap::from([("parent".into(), files.clone())]);
        assert_eq!(
            covered_files(&functions, &BTreeSet::from(["parent".into()])),
            Some(files),
            "complete profiles retain their covered sources"
        );
        assert!(
            covered_files(&functions, &BTreeSet::from(["parent".into(), "z-child".into()])).is_none(),
            "an unmapped child discards even the already resolved parent coverage"
        );
    }

    #[test]
    fn mappings_union_duplicate_names_and_skip_external_sources() -> gix_error::TestResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).canonicalize()?;
        let export = json!({"data":[{"functions":[
            {"name":"shared","filenames":["Cargo.toml"]},
            {"name":"shared","filenames":["src/main.rs"]},
            {"name":"unexecuted","count":0,"filenames":["src/main.rs"]},
            {"name":"external","filenames":[root.join("../../Cargo.toml")]}
        ]}]});
        let functions = function_files(&root, &export)?;
        assert_eq!(
            functions["shared"],
            BTreeSet::from(["Cargo.toml".into(), "src/main.rs".into()]),
            "all source files of ambiguous names contribute coverage"
        );
        assert!(
            functions["external"].is_empty(),
            "external sources cannot enter the workspace index"
        );
        assert!(
            functions["unexecuted"].contains("src/main.rs"),
            "mapped files remain known even when their functions did not execute"
        );
        assert!(
            function_files(&root, &json!({"data":[{}]})).is_err(),
            "malformed exports are not empty coverage"
        );
        Ok(())
    }
}
