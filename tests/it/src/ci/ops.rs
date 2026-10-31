use std::{
    collections::{BTreeMap, BTreeSet, hash_map::DefaultHasher},
    ffi::{OsStr, OsString},
    fs,
    hash::{Hash, Hasher},
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Command, ExitStatus},
    time::{SystemTime, UNIX_EPOCH},
};

use gix_error::{OptionExt, Result, ResultExt, bail, message};
use serde_json::Value;

pub fn execute(root: &Path, args: &[String]) -> Result<()> {
    let (operation, args) = args.split_first().ok_or_raise(|| message("Missing CI operation"))?;
    match operation.as_str() {
        "badge" if args.is_empty() => badge(root),
        "workflows" if args.is_empty() => workflows(root),
        "mode" if args.is_empty() => mode(root),
        "msrv" if args.is_empty() => msrv(root),
        "check-clean" => check_clean(root, args),
        "no-git" => no_git(root, args),
        "wasm" => wasm(root, args),
        "pure-rust" if args.is_empty() => pure_rust(root),
        _ => bail!("Unknown CI operation or invalid arguments: {operation} {args:?}"),
    }
}

pub(super) fn checked(command: &mut Command) -> Result<()> {
    let status = command.status().or_raise(|| message!("Could not run {command:?}"))?;
    require_success(command, status)
}

fn require_success(command: &Command, status: ExitStatus) -> Result<()> {
    if !status.success() {
        bail!("{command:?} failed with {status}");
    }
    Ok(())
}

pub(super) fn output(command: &mut Command) -> Result<Vec<u8>> {
    let output = command.output().or_raise(|| message!("Could not run {command:?}"))?;
    if !output.status.success() {
        bail!(
            "{command:?} failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output.stdout)
}

pub(super) fn git(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("--no-optional-locks").arg("-C").arg(root);
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    ] {
        command.env_remove(name);
    }
    command
}

fn version(metadata: &Value) -> Result<String> {
    let version = metadata["packages"]
        .as_array()
        .and_then(|packages| packages.iter().find(|package| package["name"] == "gix"))
        .and_then(|package| package["rust_version"].as_str())
        .ok_or_raise(|| message("Cargo metadata has no gix MSRV"))?;
    Ok(if version.split('.').count() == 2 {
        format!("{version}.0")
    } else {
        version.into()
    })
}

fn badge(root: &Path) -> Result<()> {
    let expected = fs::read_to_string(root.join("etc/msrv-badge.template.svg"))
        .or_raise(|| message("Read MSRV badge template"))?
        .replace("{MSRV}", &version(&super::plan::metadata(root)?)?);
    let actual = fs::read(root.join("etc/msrv-badge.svg")).or_raise(|| message("Read MSRV badge"))?;
    if expected.as_bytes() != actual {
        bail!("MSRV badge is stale; regenerate it with just msrv-badge");
    }
    Ok(())
}

fn workflows(root: &Path) -> Result<()> {
    let mut paths = fs::read_dir(root.join(".github/workflows"))
        .or_raise(|| message("List GitHub workflows"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .or_raise(|| message("Read GitHub workflow directory"))?;
    paths.sort();
    for path in paths
        .iter()
        .filter(|path| matches!(path.extension().and_then(OsStr::to_str), Some("yml" | "yaml")))
    {
        let document: Value = serde_json::from_slice(&output(Command::new("yq").args(["-o=json", "."]).arg(path))?)
            .or_raise(|| message!("Parse workflow {}", path.display()))?;
        validate_workflow(&document, path.file_name() == Some(OsStr::new("ci.yml")))
            .or_raise(|| message!("Check workflow {}", path.display()))?;
    }
    Ok(())
}

fn dependencies(job: &Value) -> Result<Vec<&str>> {
    match job.get("needs") {
        None => Ok(Vec::new()),
        Some(Value::String(name)) => Ok(vec![name.as_str()]),
        Some(Value::Array(names)) => names
            .iter()
            .map(|name| name.as_str().ok_or_raise(|| message("Job needs must be strings")))
            .collect(),
        _ => bail!("Job needs must be a string or an array"),
    }
}

fn validate_workflow(document: &Value, ci: bool) -> Result<()> {
    let jobs = document["jobs"]
        .as_object()
        .ok_or_raise(|| message("Workflow is missing jobs"))?;
    for (name, job) in jobs {
        for dependency in dependencies(job)? {
            if !jobs.contains_key(dependency) {
                bail!("Job {name} depends on missing job {dependency}");
            }
        }
        if let Some(steps) = job["steps"].as_array() {
            for step in steps {
                if step["uses"]
                    .as_str()
                    .is_some_and(|action| action.starts_with("actions/checkout@"))
                    && step["with"]["persist-credentials"] != false
                {
                    bail!("Job {name} must set persist-credentials: false on checkout");
                }
            }
        }
    }
    if ci {
        if jobs.keys().map(String::as_str).collect::<BTreeSet<_>>() != BTreeSet::from(["plan", "run", "tests-pass"]) {
            bail!("CI checks must be scheduled through the shared plan/run jobs");
        }
        if dependencies(&jobs["tests-pass"])?.into_iter().collect::<BTreeSet<_>>() != BTreeSet::from(["plan", "run"])
            || !jobs["tests-pass"]["if"]
                .as_str()
                .is_some_and(|condition| condition.contains("always()"))
        {
            bail!("The Tests pass gate must always inspect both plan and run");
        }
    }
    Ok(())
}

fn path_from_git(bytes: &[u8]) -> Result<PathBuf> {
    #[cfg(unix)]
    let path = {
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(OsStr::from_bytes(bytes))
    };
    #[cfg(not(unix))]
    let path = PathBuf::from(std::str::from_utf8(bytes).or_raise(|| message("Git path is not UTF-8"))?);
    if path.as_os_str().is_empty() || path.components().any(|part| !matches!(part, Component::Normal(_))) {
        bail!("Git returned a path outside the source directory: {path:?}");
    }
    Ok(path)
}

fn mode(root: &Path) -> Result<()> {
    let index = output(git(root).args(["ls-files", "--stage", "-z", "--", "*.sh"]))?;
    let mut paths = BTreeMap::new();
    for record in index.split(|byte| *byte == 0).filter(|record| !record.is_empty()) {
        let separator = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_raise(|| message("Malformed Git index record"))?;
        let (header, path) = record.split_at(separator);
        let path = &path[1..];
        let mode = header
            .split(|byte| *byte == b' ')
            .next()
            .ok_or_raise(|| message("Missing Git mode"))?;
        if matches!(mode, b"100644" | b"100755") {
            paths.insert(path_from_git(path)?, mode == b"100755");
        }
    }
    for path in output(git(root).args(["ls-files", "--others", "--exclude-standard", "-z", "--", "*.sh"]))?
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        paths.insert(path_from_git(path)?, false);
    }
    for (path, index_executable) in paths {
        let absolute = root.join(&path);
        let metadata = match fs::symlink_metadata(&absolute) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).or_raise(|| message!("Inspect script {}", path.display())),
        };
        if !metadata.is_file() {
            continue;
        }
        let executable = executable(&metadata, index_executable);
        let mut prefix = [0; 2];
        let read = fs::File::open(&absolute)
            .and_then(|mut file| file.read(&mut prefix))
            .or_raise(|| message!("Read script {}", path.display()))?;
        if mode_mismatch(&path, executable, read == 2 && prefix == *b"#!") {
            bail!(
                "Script {} has mismatched executable permission and shebang",
                path.display()
            );
        }
    }
    Ok(())
}

fn executable(metadata: &fs::Metadata, index_executable: bool) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = index_executable;
        metadata.permissions().mode() & 0o100 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        index_executable
    }
}

fn mode_mismatch(path: &Path, executable: bool, shebang: bool) -> bool {
    executable != shebang && (executable || !path.components().any(|part| part.as_os_str() == "fuzz"))
}

pub(super) struct Snapshot {
    pub path: PathBuf,
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path) {
            eprintln!("Could not remove CI snapshot {}: {error}", self.path.display());
        }
    }
}

pub(super) fn source_snapshot(root: &Path) -> Result<Snapshot> {
    let files = output(git(root).args(["ls-files", "--cached", "--others", "--exclude-standard", "-z"]))?;
    snapshot_files(root, &files)
}

fn snapshot_files(root: &Path, files: &[u8]) -> Result<Snapshot> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .or_raise(|| message("Read time for CI snapshot"))?
        .as_nanos();
    let path = std::env::temp_dir().join(format!("gitoxide-ci-source-{}-{stamp}", std::process::id()));
    fs::create_dir(&path).or_raise(|| message!("Create CI snapshot {}", path.display()))?;
    let snapshot = Snapshot { path };
    for path in source_paths(files)? {
        let source = root.join(&path);
        let Some(metadata) = source_metadata(root, &path)? else {
            continue;
        };
        let destination = snapshot.path.join(&path);
        let mut directory = snapshot.path.clone();
        for component in path
            .parent()
            .ok_or_raise(|| message("Snapshot file has no parent"))?
            .components()
        {
            directory.push(component);
            match fs::create_dir(&directory) {
                Ok(()) => {}
                Err(error)
                    if error.kind() == std::io::ErrorKind::AlreadyExists
                        && fs::symlink_metadata(&directory).is_ok_and(|metadata| metadata.is_dir()) => {}
                Err(error) => {
                    return Err(error).or_raise(|| message!("Create snapshot directory {}", directory.display()));
                }
            }
        }
        if metadata.file_type().is_symlink() {
            let target = fs::read_link(&source).or_raise(|| message!("Read symlink {}", path.display()))?;
            validate_snapshot_symlink(&path, &target)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(target, &destination)
                .or_raise(|| message!("Copy symlink {}", path.display()))?;
            #[cfg(windows)]
            {
                use std::os::windows::fs::FileTypeExt;
                let result = if metadata.file_type().is_symlink_dir() {
                    std::os::windows::fs::symlink_dir(target, &destination)
                } else {
                    std::os::windows::fs::symlink_file(target, &destination)
                };
                result.or_raise(|| message!("Copy symlink {}", path.display()))?;
            }
        } else if metadata.is_file() {
            fs::copy(&source, &destination).or_raise(|| message!("Copy source {}", path.display()))?;
        } else {
            bail!("Unsupported source entry in CI snapshot: {}", path.display());
        }
    }
    Ok(snapshot)
}

fn validate_snapshot_symlink(path: &Path, target: &Path) -> Result<()> {
    let mut depth = path.components().count() - 1;
    for component in target.components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir if depth > 0 => depth -= 1,
            _ => bail!(
                "Cannot isolate symlink {} -> {}: its target escapes the source snapshot",
                path.display(),
                target.display()
            ),
        }
    }
    Ok(())
}

fn source_paths(files: &[u8]) -> Result<BTreeSet<PathBuf>> {
    files
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(path_from_git)
        .collect()
}

fn source_metadata(root: &Path, path: &Path) -> Result<Option<fs::Metadata>> {
    let mut absolute = root.to_owned();
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        absolute.push(component);
        let metadata = match fs::symlink_metadata(&absolute) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).or_raise(|| message!("Inspect source {}", absolute.display())),
        };
        if components.peek().is_none() {
            return Ok(Some(metadata));
        }
        // A dirty checkout can replace a tracked directory with a file or symlink.
        // Its old index entries are no longer files in this working tree.
        if !metadata.is_dir() {
            return Ok(None);
        }
    }
    bail!("Source path is empty")
}

fn msrv(root: &Path) -> Result<()> {
    let metadata = super::plan::metadata(root)?;
    let rust_version = version(&metadata)?;
    let target = metadata["target_directory"]
        .as_str()
        .ok_or_raise(|| message("Missing Cargo target directory"))?;
    let snapshot = source_snapshot(root)?;
    let mut cargo = Command::new("cargo");
    cargo.current_dir(&snapshot.path).env("CARGO_TARGET_DIR", target);
    checked(cargo.args(["+nightly", "update", "-Zminimal-versions"]))?;
    if cfg!(windows) {
        checked(
            Command::new("cargo")
                .current_dir(&snapshot.path)
                .env("CARGO_TARGET_DIR", target)
                .args([
                    "+nightly",
                    "update",
                    "-Zminimal-versions",
                    "-p",
                    "errno@0.2",
                    "--precise",
                    "0.2.8",
                ]),
        )?;
    }
    for additional in [
        &[][..],
        &[
            "--no-default-features",
            "--features",
            "async-network-client,max-performance,sha1",
        ][..],
    ] {
        checked(
            Command::new("cargo")
                .current_dir(&snapshot.path)
                .env("CARGO_TARGET_DIR", target)
                .arg(format!("+{rust_version}"))
                .args(["build", "--locked", "-p", "gix"])
                .args(additional),
        )?;
    }
    Ok(())
}

#[derive(PartialEq, Eq)]
pub(super) struct SourceState {
    index: Vec<u8>,
    files: BTreeMap<PathBuf, Option<u64>>,
}

pub(super) fn source_state(root: &Path) -> Result<SourceState> {
    let index = output(git(root).args(["ls-files", "--stage", "-v", "-z"]))?;
    let files = output(git(root).args(["ls-files", "--cached", "--others", "--exclude-standard", "-z"]))?;
    Ok(SourceState {
        index,
        files: file_state(root, &files)?,
    })
}

fn file_state(root: &Path, files: &[u8]) -> Result<BTreeMap<PathBuf, Option<u64>>> {
    let mut state = BTreeMap::new();
    for path in source_paths(files)? {
        let Some(metadata) = source_metadata(root, &path)? else {
            state.insert(path, None);
            continue;
        };
        let absolute = root.join(&path);
        let mut hash = DefaultHasher::new();
        metadata.file_type().is_symlink().hash(&mut hash);
        if metadata.file_type().is_symlink() {
            fs::read_link(&absolute)
                .or_raise(|| message!("Read source symlink {}", path.display()))?
                .hash(&mut hash);
        } else if metadata.is_file() {
            executable(&metadata, false).hash(&mut hash);
            let mut file = fs::File::open(&absolute).or_raise(|| message!("Read source {}", path.display()))?;
            let mut buffer = [0; 32768];
            loop {
                let count = file
                    .read(&mut buffer)
                    .or_raise(|| message!("Read source {}", path.display()))?;
                if count == 0 {
                    break;
                }
                hash.write(&buffer[..count]);
            }
        } else {
            bail!("Unsupported source entry: {}", path.display());
        }
        state.insert(path, Some(hash.finish()));
    }
    Ok(state)
}

fn check_clean(root: &Path, args: &[String]) -> Result<()> {
    let (program, args) = args
        .split_first()
        .ok_or_raise(|| message("check-clean needs a command"))?;
    let before = source_state(root)?;
    let mut command = Command::new(program);
    command.args(args).current_dir(root);
    let status = command.status().or_raise(|| message!("Could not run {command:?}"))?;
    if before != source_state(root)? {
        bail!("{command:?} changed tracked or visible untracked files; changes were retained for inspection");
    }
    require_success(&command, status)
}

fn without_git(path: &OsStr) -> Result<OsString> {
    let entries: Vec<_> = std::env::split_paths(path).collect();
    let installations: Vec<_> = entries
        .iter()
        .filter(|entry| entry.join("git.exe").is_file())
        .filter_map(|entry| {
            let parent = entry.parent()?;
            if parent.join("mingw64").is_dir() || parent.join("mingw32").is_dir() || parent.join("usr").is_dir() {
                Some(parent.to_owned())
            } else {
                None
            }
        })
        .collect();
    std::env::join_paths(entries.iter().filter(|entry| {
        !installations.iter().any(|installation| entry.starts_with(installation))
            && !["git.exe", "git.cmd", "git.bat", "git.com"]
                .iter()
                .any(|name| entry.join(name).is_file())
    }))
    .or_raise(|| message("Construct PATH without Git"))
}

fn no_git(root: &Path, args: &[String]) -> Result<()> {
    if !cfg!(windows) {
        bail!("The no-git operation requires Windows");
    }
    let path = without_git(&std::env::var_os("PATH").ok_or_raise(|| message("PATH is unset"))?)?;
    let mut probe = Command::new("where.exe");
    probe
        .arg("git")
        .current_dir(root)
        .env("PATH", &path)
        .env_remove("EXEPATH");
    let probe = probe
        .output()
        .or_raise(|| message("Check that Git is absent from the child PATH"))?;
    if probe.status.success() || !probe.stdout.is_empty() {
        bail!("Git remains available in the no-git test environment");
    }
    if probe.status.code() != Some(1) {
        bail!(
            "Could not verify Git's absence: where.exe failed with {}: {}",
            probe.status,
            String::from_utf8_lossy(&probe.stderr)
        );
    }
    checked(
        Command::new("cargo")
            .current_dir(root)
            .env("PATH", path)
            .env_remove("EXEPATH")
            .args(["nextest", "run", "--locked"])
            .args(args),
    )
}

const WASM_DEFAULT_PACKAGES: &[&str] = &[
    "gix-actor",
    "gix-attributes",
    "gix-bitmap",
    "gix-chunk",
    "gix-command",
    "gix-config-value",
    "gix-date",
    "gix-glob",
    "gix-mailmap",
    "gix-packetline",
    "gix-path",
    "gix-pathspec",
    "gix-prompt",
    "gix-quote",
    "gix-url",
    "gix-validate",
];
const WASM_SHA1_PACKAGES: &[&str] = &[
    "gix-commitgraph",
    "gix-hash",
    "gix-hashtable",
    "gix-index",
    "gix-object",
    "gix-refspec",
    "gix-revision",
    "gix-traverse",
];

pub(super) fn supports_wasm(name: &str) -> bool {
    WASM_DEFAULT_PACKAGES.contains(&name)
        || WASM_SHA1_PACKAGES.contains(&name)
        || matches!(name, "gix-sec" | "gix-parallel" | "gix-utils" | "gix-pack")
}

fn wasm(root: &Path, args: &[String]) -> Result<()> {
    let (target, selected) = args
        .split_first()
        .ok_or_raise(|| message("wasm needs a target and selected packages"))?;
    if !matches!(
        target.as_str(),
        "wasm32-unknown-unknown" | "wasm32-wasip1" | "wasm32-wasip2"
    ) {
        bail!("Unsupported WASM target {target}");
    }
    let mut cases: Vec<(&str, Vec<&str>)> = Vec::new();
    if target.starts_with("wasm32-wasi") {
        cases.push(("gix-sec", Vec::new()));
    }
    for package in WASM_DEFAULT_PACKAGES {
        cases.push((package, Vec::new()));
    }
    for feature in ["", "parallel", "once_cell", "parallel,once_cell"] {
        cases.push(("gix-parallel", vec!["--no-default-features", "--features", feature]));
    }
    for feature in [
        "",
        "progress",
        "progress,progress-unit-bytes",
        "progress,progress-unit-human-numbers",
        "interrupt",
        "io-pipe",
        "cache-efficiency-debug",
    ] {
        cases.push(("gix-utils", vec!["--no-default-features", "--features", feature]));
    }
    cases.push(("gix-pack", vec!["--features", "sha1,wasm"]));
    for package in WASM_SHA1_PACKAGES {
        cases.push((package, vec!["--features", "sha1"]));
    }
    cases.push(("gix-pack", vec!["--all-features"]));
    for (package, args) in cases {
        if selected.iter().any(|selected| selected == package) {
            checked(
                Command::new("cargo")
                    .current_dir(root)
                    .args(["build", "--locked", "-p", package, "--target", target])
                    .args(args),
            )?;
        }
    }
    Ok(())
}

fn pure_rust(root: &Path) -> Result<()> {
    if !cfg!(target_os = "linux") {
        bail!("The constrained pure-Rust build requires a Linux Docker host");
    }
    let source_mount = format!("type=bind,source={},target=/src,readonly", root.display());
    checked(Command::new("docker").args([
        "run",
        "--rm",
        "--mount",
        &source_mount,
        "--mount",
        "type=volume,source=gitoxide-ci-pure-rust,target=/cache",
        "--workdir",
        "/src",
        "debian:stable-slim",
        "bash",
        "tests/it/ci/pure-rust.sh",
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn snapshots_copy_current_files_without_mutating_the_source() -> gix_error::TestResult {
        let source = snapshot_files(Path::new(""), b"")?;
        fs::create_dir(source.path.join("src"))?;
        fs::write(source.path.join("src/changed"), "current working contents")?;
        fs::write(source.path.join("untracked"), "not yet staged")?;
        fs::write(source.path.join("ignored"), "not included by git")?;
        let files = b"src/changed\0src/deleted\0untracked\0src/changed\0";
        let before = file_state(&source.path, files)?;
        let snapshot = snapshot_files(&source.path, files)?;
        assert_eq!(
            fs::read_to_string(snapshot.path.join("src/changed"))?,
            "current working contents",
            "a snapshot uses working contents instead of older index contents"
        );
        assert_eq!(
            fs::read_to_string(snapshot.path.join("untracked"))?,
            "not yet staged",
            "visible untracked inputs participate in local checks"
        );
        assert!(
            !snapshot.path.join("src/deleted").exists() && !snapshot.path.join("ignored").exists(),
            "deleted and ignored files are absent from the snapshot"
        );
        fs::write(snapshot.path.join("src/changed"), "test output")?;
        fs::remove_file(snapshot.path.join("untracked"))?;
        assert_eq!(
            file_state(&source.path, files)?,
            before,
            "writes and deletions in a snapshot cannot change source files"
        );
        fs::write(source.path.join("src/changed"), "new working contents")?;
        assert_ne!(
            file_state(&source.path, files)?,
            before,
            "working changes are detected without relying on Git's index flags"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn snapshots_preserve_safe_symlinks_and_modes_without_traversing_links() -> gix_error::TestResult {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let source = snapshot_files(Path::new(""), b"")?;
        fs::create_dir(source.path.join("src"))?;
        fs::write(source.path.join("src/target"), "original")?;
        fs::set_permissions(source.path.join("src/target"), fs::Permissions::from_mode(0o755))?;
        symlink("src", source.path.join("directory-link"))?;
        symlink("target", source.path.join("src/link"))?;
        symlink("missing", source.path.join("src/dangling"))?;
        let files = b"directory-link\0directory-link/target\0src/link\0src/dangling\0src/target\0";
        let before = file_state(&source.path, files)?;
        let snapshot = snapshot_files(&source.path, files)?;
        assert_eq!(
            fs::read_link(snapshot.path.join("directory-link"))?,
            Path::new("src"),
            "a replaced directory stays a symlink instead of traversing stale index entries"
        );
        assert_eq!(fs::read_link(snapshot.path.join("src/dangling"))?, Path::new("missing"));
        assert_eq!(
            fs::metadata(snapshot.path.join("src/target"))?.permissions().mode() & 0o777,
            0o755,
            "working executable permissions are retained"
        );
        fs::write(snapshot.path.join("src/link"), "snapshot output")?;
        assert_eq!(
            file_state(&source.path, files)?,
            before,
            "internal links point into the snapshot, never back at the source"
        );
        fs::set_permissions(source.path.join("src/target"), fs::Permissions::from_mode(0o644))?;
        assert_ne!(file_state(&source.path, files)?, before, "mode changes are detected");
        symlink(source.path.join("src/target"), source.path.join("absolute-link"))?;
        assert!(
            snapshot_files(&source.path, b"absolute-link\0").is_err(),
            "absolute links cannot give snapshot tests access to source files"
        );
        symlink("../outside", source.path.join("escaping-link"))?;
        assert!(
            snapshot_files(&source.path, b"escaping-link\0").is_err(),
            "relative links cannot escape the snapshot"
        );
        assert!(
            validate_snapshot_symlink(Path::new("src/license"), Path::new("../LICENSE")).is_ok(),
            "relative license symlinks remain supported"
        );
        Ok(())
    }

    #[test]
    fn badge_versions_and_script_modes() -> gix_error::TestResult {
        for (input, expected) in [("1.88", "1.88.0"), ("1.88.1", "1.88.1")] {
            assert_eq!(
                version(&json!({"packages": [{"name": "gix", "rust_version": input}]}))?,
                expected,
                "badge versions include patch level"
            );
        }
        assert!(
            mode_mismatch(Path::new("fixture.sh"), false, true),
            "fixture scripts need their executable bit"
        );
        assert!(
            !mode_mismatch(Path::new("gix/fuzz/fixture.sh"), false, true),
            "fuzzer scripts retain their existing exemption"
        );
        assert!(
            mode_mismatch(Path::new("gix/fuzz/fixture.sh"), true, false),
            "executable scripts always need a shebang"
        );
        assert!(
            path_from_git(b"../outside").is_err(),
            "snapshot paths cannot escape the source root"
        );
        Ok(())
    }

    #[test]
    fn workflow_checks_include_credentials_and_gate() -> gix_error::TestResult {
        let mut workflow = json!({"jobs": {
            "plan": {"steps": [{"uses": "actions/checkout@version", "with": {"persist-credentials": false}}]},
            "run": {"needs": "plan"},
            "tests-pass": {"needs": ["plan", "run"], "if": "always()"}
        }});
        validate_workflow(&workflow, true)?;
        workflow["jobs"]["plan"]["steps"][0]["with"]["persist-credentials"] = true.into();
        assert!(
            validate_workflow(&workflow, true).is_err(),
            "checkout credentials must not be retained"
        );
        workflow["jobs"]["plan"]["steps"][0]["with"]["persist-credentials"] = false.into();
        workflow["jobs"]["tests-pass"]["needs"] = json!(["run"]);
        assert!(
            validate_workflow(&workflow, true).is_err(),
            "the planner must remain blocking"
        );
        workflow["jobs"]["run"]["needs"] = "missing".into();
        assert!(
            validate_workflow(&workflow, false).is_err(),
            "all workflow dependency names must resolve"
        );
        Ok(())
    }
}
