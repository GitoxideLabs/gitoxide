//! Per-test selection from LLVM coverage, with ordinary execution when coverage is unavailable.
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use gix_error::{OptionExt, Result, ResultExt, bail, message};
use serde_json::{Value, json};

use super::{ops, run};

mod index;
mod llvm;

const PROFILE_SINK: &str = if cfg!(windows) { "NUL" } else { "/dev/null" };
const INLINE_FILTER_LIMIT: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(super) enum Mode {
    /// Automatically collect missing coverage and reuse matching per-test indexes in fast QA.
    Auto,
    /// Use ordinary package and feature selection without coverage.
    Off,
    /// Run all selected tests with instrumentation and record a local index.
    Collect,
    /// Report the historical selection, but run all selected tests.
    Shadow,
    /// Select tests using saved coverage without collecting missing configurations.
    Prune,
}

#[derive(Clone, Debug)]
pub(super) struct Summary {
    pub mode: Mode,
    pub cases: usize,
    pub measured_cases: usize,
    /// Counts test executions across configurations, not unique test names.
    pub executed: usize,
    pub omitted: usize,
    pub reused: usize,
    pub collected: usize,
    pub fallback: usize,
    pub inventory_time: Duration,
    pub selection_time: Duration,
    pub analysis_time: Duration,
}

impl Summary {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            cases: 0,
            measured_cases: 0,
            executed: 0,
            omitted: 0,
            reused: 0,
            collected: 0,
            fallback: 0,
            inventory_time: Duration::ZERO,
            selection_time: Duration::ZERO,
            analysis_time: Duration::ZERO,
        }
    }
}

pub(super) struct Run {
    mode: Mode,
    directory: PathBuf,
    work: PathBuf,
    compiler: String,
    cargo_config: Value,
    tools: Option<llvm::Tools>,
    baseline: Option<String>,
    pending: BTreeMap<String, Value>,
    changed: BTreeSet<String>,
    dirty: BTreeSet<String>,
    source: Option<ops::SourceState>,
    cases: usize,
    pub summary: Summary,
}

impl Drop for Run {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.work) {
            eprintln!(
                "Could not remove coverage scratch directory {}: {error}",
                self.work.display()
            );
        }
    }
}

impl Run {
    pub fn optional(root: &Path, mode: Option<Mode>, fast: bool, base: Option<&str>) -> Option<Self> {
        let mode = mode
            .or_else(|| {
                use clap::ValueEnum;
                let value = std::env::var("GIX_QA_COVERAGE")
                    .ok()
                    .filter(|value| !value.is_empty())?;
                match Mode::from_str(&value, false) {
                    Ok(mode) => Some(mode),
                    Err(error) => {
                        eprintln!("Coverage disabled: {error}");
                        Some(Mode::Off)
                    }
                }
            })
            .unwrap_or(Mode::Auto);
        if mode == Mode::Off || (mode == Mode::Auto && !fast) {
            return None;
        }
        match Self::new(root, mode, fast, base) {
            Ok(run) => Some(run),
            Err(error) => {
                unavailable(&error);
                None
            }
        }
    }

    fn new(root: &Path, mut mode: Mode, fast: bool, base: Option<&str>) -> Result<Self> {
        if mode == Mode::Prune && !fast {
            eprintln!("Coverage: exhaustive QA reports candidates but never omits tests.");
            mode = Mode::Shadow;
        }
        let changed = if mode == Mode::Auto {
            let paths = changed_paths(root, base.ok_or_raise(|| message("Missing QA comparison base"))?)?;
            if paths.is_empty()
                || paths
                    .iter()
                    .any(|path| Path::new(path).extension().is_none_or(|ext| ext != "rs"))
            {
                bail!("Automatic coverage requires a nonempty Rust-only change");
            }
            paths
        } else if matches!(mode, Mode::Prune | Mode::Shadow)
            && let Some(base) = base
        {
            changed_paths(root, base)?
        } else {
            BTreeSet::new()
        };
        let tools = matches!(mode, Mode::Auto | Mode::Collect)
            .then(|| llvm::discover("stable"))
            .transpose()?;
        let compiler = if let Some(tools) = &tools {
            tools.compiler.clone()
        } else {
            String::from_utf8(ops::output(
                Command::new("rustc")
                    .env("RUSTUP_TOOLCHAIN", "stable")
                    .env("RUSTUP_AUTO_INSTALL", "0")
                    .arg("-vV"),
            )?)
            .or_error()?
        };
        // Capture before enumerating dirty paths so an intervening edit cannot escape provenance.
        let source = (mode == Mode::Auto).then(|| ops::source_state(root)).transpose()?;
        let baseline = if mode == Mode::Auto {
            Some(head(root)?)
        } else {
            clean_head(root)?
        };
        let dirty = if mode == Mode::Auto {
            changed_paths(root, baseline.as_deref().expect("automatic coverage has a HEAD"))?
        } else {
            BTreeSet::new()
        };
        let cargo_config = cargo_config(root)?;
        let directory = root.join("target/qa-coverage");
        fs::create_dir_all(&directory).or_error()?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).or_error()?.as_nanos();
        let work = directory.join(format!("work-{}-{stamp}", std::process::id()));
        fs::create_dir(&work).or_error()?;
        eprintln!(
            "Coverage: {mode:?}; local indexes in {}",
            directory.join("indexes").display()
        );
        Ok(Self {
            mode,
            directory,
            work,
            compiler,
            cargo_config,
            tools,
            baseline,
            pending: BTreeMap::new(),
            changed,
            dirty,
            source,
            cases: 0,
            summary: Summary::new(mode),
        })
    }

    pub fn configuration(&self, build: &Command, replay: &Command) -> Value {
        json!({"compiler": self.compiler, "cargo_config": self.cargo_config, "build": signature(build), "replay": signature(replay)})
    }

    fn instrument(&self, build: &Command) -> Result<Command> {
        supported_compiler(&signature(build))?;
        if std::env::var_os("RUSTC_WRAPPER").is_some() || std::env::var_os("RUSTC_WORKSPACE_WRAPPER").is_some() {
            bail!("Coverage collection does not replace an existing compiler wrapper");
        }
        let mut command = copy_command(build);
        // A compiler wrapper preserves Cargo's configured rustflags, including target-specific flags.
        command
            .env("RUSTC_WORKSPACE_WRAPPER", std::env::current_exe().or_error()?)
            .env("GIX_QA_COVERAGE_RUSTC", "1")
            .env("RUSTUP_AUTO_INSTALL", "0")
            .env("CARGO_TARGET_DIR", self.directory.join("build"))
            .env("LLVM_PROFILE_FILE", PROFILE_SINK);
        Ok(command)
    }

    fn execute(
        &mut self,
        root: &Path,
        configuration: Value,
        command: &mut Command,
        instrumented: bool,
        log: Option<&File>,
    ) -> Result<()> {
        self.cases += 1;
        let raw = self.work.join(format!("case-{}", self.cases));
        let mut collect = self.mode == Mode::Collect;
        let mut total = None;
        let mut omitted = 0;
        let mut reused = false;
        if instrumented {
            // Listing and unwrapped subprocesses must not leave profiles in the source snapshot.
            command.env("LLVM_PROFILE_FILE", PROFILE_SINK);
        }
        let preparation = (|| {
            supported_compiler(&configuration["build"])?;
            let inventory = self.inventory(command, log)?;
            total = inventory_count(&inventory, command);
            if matches!(self.mode, Mode::Auto | Mode::Prune | Mode::Shadow) {
                let started = Instant::now();
                let selection = {
                    let cached = (|| {
                        let data = self.load(&configuration)?;
                        let baseline = data["baseline"]
                            .as_str()
                            .ok_or_raise(|| message("Missing coverage baseline"))?;
                        let paths = affected_paths(&data, changed_paths(root, baseline)?, &self.changed)?;
                        Ok((data, paths))
                    })();
                    match cached {
                        Ok((data, paths)) => match index::select(&data, &inventory, &paths) {
                            Ok(selection) => Ok(Some(selection)),
                            Err(error)
                                if self.mode == Mode::Auto
                                    && needs_refresh(
                                        &data,
                                        self.source
                                            .as_ref()
                                            .expect("automatic coverage captures source state")
                                            .content_hash(),
                                    ) =>
                            {
                                eprintln!("Coverage: refreshing an unusable index for changed source inputs: {error}");
                                collect = true;
                                Ok(None)
                            }
                            Err(error) => Err(error),
                        },
                        Err(error) if self.mode == Mode::Auto => {
                            eprintln!("Coverage: collecting this complete configuration for reuse: {error}");
                            collect = true;
                            Ok(None)
                        }
                        Err(error) => Err(error),
                    }
                };
                let elapsed = started.elapsed();
                self.summary.selection_time += elapsed;
                if let Some(selection) = selection? {
                    eprintln!(
                        "Coverage: {:?} selects {}/{} tests ({:.3}s)",
                        self.mode,
                        selection.selected,
                        selection.total,
                        elapsed.as_secs_f64()
                    );
                    if matches!(self.mode, Mode::Auto | Mode::Prune) && selection.selected < selection.total {
                        let candidate = filtered_with_file(root, &raw, command, &selection.filter)?;
                        if selection.filter.len() > INLINE_FILTER_LIMIT {
                            self.inventory(&candidate, log)?;
                        }
                        *command = candidate;
                        omitted = selection.total - selection.selected;
                    }
                    reused = true;
                }
            }
            if collect {
                if !instrumented {
                    bail!("Coverage instrumentation was unavailable for this configuration");
                }
                if signature(command)["env"]["NEXTEST_PROFILE"].is_string()
                    || command.get_args().any(|arg| {
                        matches!(arg.to_str(), Some("--profile" | "-P"))
                            || arg.to_str().is_some_and(|arg| arg.starts_with("--profile="))
                    })
                {
                    bail!("Coverage collection keeps custom Nextest profiles unchanged");
                }
                let config = wrapper_config(root, &raw)?;
                let mut candidate = copy_command(command);
                candidate.arg("--config-file").arg(config);
                // Configuration/tool-version errors are optional preparation failures, not test failures.
                self.inventory(&candidate, log)?;
                *command = candidate;
            }
            Ok(inventory)
        })();
        let inventory = match preparation {
            Ok(inventory) => Some(inventory),
            Err(error) => {
                unavailable(&error);
                None
            }
        };
        run::redirect(command, log, true)?;
        let started = Instant::now();
        // Once tests start, their failure remains a QA failure, even when recording fails separately.
        ops::checked(command)?;
        eprintln!("Coverage: test execution {:.3}s", started.elapsed().as_secs_f64());
        self.summary.cases += 1;
        if let Some(total) = total {
            self.summary.measured_cases += 1;
            self.summary.executed += total - omitted;
        }
        self.summary.omitted += omitted;
        let mut collected = false;
        if collect && let Some(inventory) = inventory {
            let started = Instant::now();
            let result = self
                .tools
                .as_ref()
                .expect("collection discovers LLVM tools")
                .index(root, &inventory, &raw);
            self.summary.analysis_time += started.elapsed();
            match result {
                Ok(mut data) => {
                    data["version"] = 2.into();
                    data["configuration"] = configuration.clone();
                    data["baseline"] = json!(self.baseline);
                    data["dirty_paths"] = json!(self.dirty);
                    if let Some(source) = &self.source {
                        data["source_hash"] = json!(source.content_hash());
                    }
                    eprintln!(
                        "Coverage: index {} tests, {} files, {} bytes in {:.3}s",
                        data["tests"].as_array().map_or(0, Vec::len),
                        data["files"].as_array().map_or(0, Vec::len),
                        data.to_string().len(),
                        started.elapsed().as_secs_f64()
                    );
                    self.pending.insert(key(&configuration), data);
                    collected = true;
                }
                Err(error) => unavailable(&error),
            }
            if let Err(error) = fs::remove_dir_all(&raw) {
                eprintln!("Could not remove processed coverage profiles: {error}");
            }
        }
        if reused {
            self.summary.reused += 1;
        } else if collected {
            self.summary.collected += 1;
        } else {
            self.summary.fallback += 1;
        }
        Ok(())
    }

    fn inventory(&mut self, command: &Command, log: Option<&File>) -> Result<Value> {
        let started = Instant::now();
        let inventory = current_inventory(command, log);
        let elapsed = started.elapsed();
        self.summary.inventory_time += elapsed;
        eprintln!("Coverage: list tests {:.3}s", elapsed.as_secs_f64());
        inventory
    }

    pub fn finish(&mut self, root: &Path) {
        let result = (|| {
            if self.pending.is_empty() {
                return Ok(());
            }
            let unchanged = if let Some(source) = &self.source {
                self.baseline.as_deref() == Some(head(root)?.as_str()) && *source == ops::source_state(root)?
            } else {
                self.baseline.is_some() && self.baseline == clean_head(root)?
            };
            if !unchanged {
                eprintln!(
                    "Coverage: not publishing measurements because the source checkout changed or collection needs a clean HEAD."
                );
                return Ok(());
            }
            let directory = self.directory.join("indexes");
            fs::create_dir_all(&directory).or_error()?;
            for (key, data) in &self.pending {
                let temporary = self.work.join(format!("{key}.json"));
                fs::write(&temporary, serde_json::to_vec(data).or_error()?).or_error()?;
                fs::rename(temporary, directory.join(format!("{key}.json"))).or_error()?;
            }
            eprintln!(
                "Coverage: published {} complete configuration indexes.",
                self.pending.len()
            );
            Ok(())
        })();
        if let Err(error) = result {
            unavailable(&error);
        }
    }

    fn load(&self, configuration: &Value) -> Result<Value> {
        let path = self
            .directory
            .join("indexes")
            .join(format!("{}.json", key(configuration)));
        let data: Value = serde_json::from_slice(
            &fs::read(&path).or_raise(|| message!("No usable coverage index at {}", path.display()))?,
        )
        .or_error()?;
        if !matches!(data["version"].as_u64(), Some(1 | 2)) || data["configuration"] != *configuration {
            bail!("Coverage index schema or configuration differs");
        }
        Ok(data)
    }
}

fn recorded_dirty(data: &Value) -> Result<BTreeSet<String>> {
    if data["version"] == 1 {
        return Ok(BTreeSet::new());
    }
    data["dirty_paths"]
        .as_array()
        .ok_or_raise(|| message("Missing capture-time dirty paths"))?
        .iter()
        .map(|path| {
            path.as_str()
                .map(str::to_owned)
                .ok_or_raise(|| message("Invalid capture-time dirty path"))
        })
        .collect()
}

fn needs_refresh(data: &Value, source_hash: u64) -> bool {
    // This hash only avoids repeated optional analysis; a match never authorizes test omission.
    data["source_hash"].as_u64() != Some(source_hash)
}

fn affected_paths(
    data: &Value,
    mut current: BTreeSet<String>,
    requested: &BTreeSet<String>,
) -> Result<BTreeSet<String>> {
    current.extend(requested.iter().cloned());
    // A capture made with local edits is not the baseline commit, even after those edits are reverted.
    current.extend(recorded_dirty(data)?);
    Ok(current)
}

fn unavailable(error: &gix_error::Error) {
    eprintln!("Coverage unavailable; retaining ordinary QA selection: {error}");
}

pub(super) fn build(command: &mut Command, coverage: Option<&Run>, log: Option<&File>) -> Result<(Vec<u8>, bool)> {
    if let Some(coverage) = coverage.filter(|coverage| matches!(coverage.mode, Mode::Auto | Mode::Collect)) {
        let started = Instant::now();
        let result = (|| {
            let mut instrumented = coverage.instrument(command)?;
            run::redirect(&mut instrumented, log, false)?;
            ops::output(&mut instrumented)
        })();
        match result {
            Ok(binaries) => {
                eprintln!("Coverage: instrumented build {:.3}s", started.elapsed().as_secs_f64());
                return Ok((binaries, true));
            }
            Err(error) => unavailable(&error),
        }
    }
    Ok((ops::output(command)?, false))
}

pub(super) fn execute(
    root: &Path,
    configuration: Option<Value>,
    command: &mut Command,
    instrumented: bool,
    coverage: Option<&mut Run>,
    log: Option<&File>,
) -> Result<()> {
    if let (Some(coverage), Some(configuration)) = (coverage, configuration) {
        coverage.execute(root, configuration, command, instrumented, log)
    } else {
        run::redirect(command, log, true)?;
        ops::checked(command)
    }
}

pub(super) fn compiler(args: &[OsString]) -> Result<()> {
    let (compiler, args) = args.split_first().ok_or_raise(|| message("Missing wrapped compiler"))?;
    ops::checked(
        Command::new(compiler)
            .args(args)
            .arg("-Cinstrument-coverage")
            .env_remove("GIX_QA_COVERAGE_RUSTC"),
    )
}

pub(super) fn test(args: &[String]) -> Result<()> {
    let (raw, args) = args
        .split_first()
        .ok_or_raise(|| message("Missing coverage output path"))?;
    let (program, args) = args
        .split_first()
        .ok_or_raise(|| message("Missing wrapped test command"))?;
    let mut command = Command::new(program);
    command.args(args);
    let capture = (|| {
        let identity = json!({
            "binary": std::env::var("NEXTEST_BINARY_ID").or_error()?,
            "name": std::env::var("NEXTEST_TEST_NAME").or_error()?,
        });
        let path = Path::new(raw).join(format!("{}-{}", key(&identity), std::process::id()));
        fs::create_dir_all(&path).or_error()?;
        fs::write(path.join("identity.json"), serde_json::to_vec(&identity).or_error()?).or_error()?;
        command.env("LLVM_PROFILE_FILE", path.join("%p-%m.profraw"));
        Ok(path)
    })();
    let path = match capture {
        Ok(path) => Some(path),
        Err(error) => {
            unavailable(&error);
            None
        }
    };
    ops::checked(&mut command)?;
    if let Some(path) = path
        && let Err(error) = fs::write(path.join("complete"), b"")
    {
        eprintln!("Coverage completion could not be recorded: {error}");
    }
    Ok(())
}

fn copy_command(command: &Command) -> Command {
    with_args(command, command.get_args().map(OsStr::to_owned).collect())
}

fn replay_args(command: &Command, list: bool, filter: Option<&str>) -> Result<Vec<OsString>> {
    let mut args = command.get_args();
    if args.next() != Some(OsStr::new("nextest")) || args.next() != Some(OsStr::new("run")) {
        bail!("Coverage requires a Nextest replay command");
    }
    let mut out: Vec<OsString> = vec!["nextest".into(), if list { "list" } else { "run" }.into()];
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some(arg) if filter.is_some() && (arg == "--run-ignored" || arg.starts_with("--run-ignored=")) => {
                bail!("Coverage keeps custom ignored-test selection unchanged");
            }
            Some("--no-fail-fast") if list => {}
            Some("--no-tests") if list || filter.is_some() => {
                args.next().ok_or_raise(|| message("Missing --no-tests value"))?;
            }
            Some(arg) if (list || filter.is_some()) && arg.starts_with("--no-tests=") => {}
            Some("-E" | "--filter-expr") if filter.is_some() => {
                args.next().ok_or_raise(|| message("Missing Nextest filter"))?;
            }
            Some(arg) if filter.is_some() && arg.starts_with("--filter-expr=") => {}
            _ => out.push(arg.into()),
        }
    }
    if list {
        out.extend(["--message-format".into(), "json".into()]);
    }
    if let Some(filter) = filter {
        out.extend(["-E".into(), filter.into(), "--no-tests=pass".into()]);
    }
    Ok(out)
}

fn with_args(command: &Command, args: Vec<OsString>) -> Command {
    // std::process::Command has no argument replacement operation.
    let mut replaced = Command::new(command.get_program());
    if let Some(directory) = command.get_current_dir() {
        replaced.current_dir(directory);
    }
    for (key, value) in command.get_envs() {
        if let Some(value) = value {
            replaced.env(key, value);
        } else {
            replaced.env_remove(key);
        }
    }
    replaced.args(args).stderr(Stdio::inherit());
    replaced
}

fn current_inventory(command: &Command, log: Option<&File>) -> Result<Value> {
    let mut list = with_args(command, replay_args(command, true, None)?);
    run::redirect(&mut list, log, false)?;
    serde_json::from_slice(&ops::output(&mut list)?).or_raise(|| message("Read current Nextest inventory"))
}

fn inventory_count(inventory: &Value, command: &Command) -> Option<usize> {
    if command.get_args().any(|arg| {
        arg.to_str()
            .is_some_and(|arg| arg == "--run-ignored" || arg.starts_with("--run-ignored="))
    }) {
        return None;
    }
    let mut count = 0;
    for suite in inventory["rust-suites"].as_object()?.values() {
        let tests = suite["testcases"].as_object()?;
        match suite["status"].as_str()? {
            "listed" => {}
            "skipped" | "skipped-default-filter" if tests.is_empty() => continue,
            _ => return None,
        }
        for test in tests.values() {
            let ignored = test["ignored"].as_bool()?;
            if test["kind"] != "test" {
                return None;
            }
            match test["filter-match"]["status"].as_str()? {
                "matches" => count += usize::from(!ignored),
                "mismatch" => {}
                _ => return None,
            }
        }
    }
    Some(count)
}

fn filtered(command: &Command, filter: &str) -> Result<Command> {
    if filter.len() > INLINE_FILTER_LIMIT {
        bail!("Coverage filter exceeds the 16 KiB command argument limit");
    }
    Ok(with_args(command, replay_args(command, false, Some(filter))?))
}

fn filtered_with_file(root: &Path, directory: &Path, command: &Command, filter: &str) -> Result<Command> {
    if filter.len() <= INLINE_FILTER_LIMIT {
        return filtered(command, filter);
    }
    if signature(command)["env"]["NEXTEST_PROFILE"].is_string()
        || command.get_args().any(|arg| {
            let Some(arg) = arg.to_str() else { return false };
            matches!(
                arg,
                "--profile" | "--config-file" | "--tool-config-file" | "--ignore-default-filter"
            ) || ["-P", "--profile=", "--config-file=", "--tool-config-file="]
                .iter()
                .any(|prefix| arg.starts_with(prefix))
        })
    {
        bail!("File-backed coverage filters keep custom Nextest profiles and configuration unchanged");
    }
    let mut candidate = filtered(command, "all()")?;
    let mut config = match fs::read_to_string(root.join(".config/nextest.toml")) {
        Ok(config) => config.parse::<toml_edit::DocumentMut>().or_error()?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => toml_edit::DocumentMut::new(),
        Err(error) => return Err(error).or_error(),
    };
    let profiles = config
        .entry("profile")
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_mut()
        .ok_or_raise(|| message("Nextest profiles must be a table"))?;
    let profile = profiles
        .entry("default")
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_mut()
        .ok_or_raise(|| message("The default Nextest profile must be a table"))?;
    if let Some(overrides) = profile.get("overrides") {
        let overrides = overrides
            .as_array_of_tables()
            .ok_or_raise(|| message("File-backed coverage filters require table-array Nextest overrides"))?;
        if overrides.iter().any(|rule| rule.contains_key("default-filter")) {
            bail!("File-backed coverage filters keep platform-specific default filters unchanged");
        }
    }
    // Nextest intersects -E all() with this exact selection without putting it on the command line.
    profile["default-filter"] = toml_edit::value(filter);
    fs::create_dir_all(directory).or_error()?;
    let path = directory.join("selection.toml");
    fs::write(&path, config.to_string()).or_error()?;
    candidate.arg("--config-file").arg(path);
    Ok(candidate)
}

fn signature(command: &Command) -> Value {
    let mut args = command.get_args();
    let mut stable_args = Vec::new();
    while let Some(arg) = args.next() {
        if matches!(
            arg.to_str(),
            Some("--cargo-metadata" | "--binaries-metadata" | "--workspace-remap")
        ) {
            args.next();
        } else {
            stable_args.push(arg.to_string_lossy().into_owned());
        }
    }
    let mut env = BTreeMap::new();
    for (key, value) in std::env::vars_os() {
        let Some(name) = key.to_str() else { continue };
        if ["CARGO_PROFILE_", "CARGO_TARGET_", "CARGO_BUILD_"]
            .iter()
            .any(|prefix| name.starts_with(prefix))
        {
            env.insert(name.to_owned(), Some(value.to_string_lossy().into_owned()));
        }
    }
    for key in [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_TARGET",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_PROFILE_DEV_OPT_LEVEL",
        "CARGO_PROFILE_DEV_DEBUG",
        "CARGO_PROFILE_DEV_STRIP",
        "NEXTEST_PROFILE",
        "RUSTC_BOOTSTRAP",
        "CARGO_HOME",
        "CARGO_INCREMENTAL",
    ] {
        env.insert(
            key.to_owned(),
            std::env::var_os(key).map(|value| value.to_string_lossy().into_owned()),
        );
    }
    for (key, value) in command.get_envs() {
        if !matches!(
            key.to_str(),
            Some("GIX_TEST_FIXTURE_CACHE" | "GIX_TEST_FIXTURE_SOURCE_ROOT")
        ) {
            env.insert(
                key.to_string_lossy().into_owned(),
                value.map(|value| value.to_string_lossy().into_owned()),
            );
        }
    }
    json!({"args": stable_args, "env": env})
}

fn key(value: &Value) -> String {
    // The complete configuration is checked after lookup, so hash collisions only cause a cache miss.
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.to_string().bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn supported_compiler(signature: &Value) -> Result<()> {
    if signature["env"]["RUSTUP_TOOLCHAIN"] != "stable"
        || [
            "RUSTC",
            "CARGO_BUILD_RUSTC",
            "CARGO_BUILD_RUSTC_WRAPPER",
            "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        ]
        .iter()
        .any(|key| !signature["env"][key].is_null())
    {
        bail!("Coverage currently supports the ordinary stable compiler only; this case keeps its original compiler");
    }
    Ok(())
}

fn cargo_config(root: &Path) -> Result<Value> {
    let mut locations: Vec<_> = root
        .ancestors()
        .enumerate()
        .map(|(depth, path)| (format!("ancestor-{depth}"), path.join(".cargo")))
        .collect();
    let home = std::env::var_os("CARGO_HOME").map(PathBuf::from).or_else(|| {
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .map(|home| PathBuf::from(home).join(".cargo"))
    });
    if let Some(home) = home {
        locations.push(("cargo-home".into(), home));
    }
    let mut contents = BTreeMap::new();
    for (label, directory) in locations {
        for name in ["config", "config.toml"] {
            let data = match fs::read_to_string(directory.join(name)) {
                Ok(data) => data,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error).or_error(),
            };
            let config = data.parse::<toml_edit::DocumentMut>().or_error()?;
            if ["rustc", "rustc-wrapper", "rustc-workspace-wrapper"]
                .iter()
                .any(|key| config.get("build").and_then(|build| build.get(key)).is_some())
            {
                bail!(
                    "Coverage does not replace a compiler configured in {}",
                    directory.join(name).display()
                );
            }
            // Keep credentials and registry URLs out of a shareable index.
            contents.insert(format!("{label}/{name}"), key(&json!(data)));
        }
    }
    Ok(json!(contents))
}

fn clean_head(root: &Path) -> Result<Option<String>> {
    if !ops::output(ops::git(root)?.args(["status", "--porcelain=v1", "-z", "--untracked-files=all"]))?.is_empty() {
        return Ok(None);
    }
    Ok(Some(head(root)?))
}

fn head(root: &Path) -> Result<String> {
    Ok(
        String::from_utf8(ops::output(ops::git(root)?.args(["rev-parse", "HEAD"]))?)
            .or_error()?
            .trim()
            .into(),
    )
}

fn changed_paths(root: &Path, baseline: &str) -> Result<BTreeSet<String>> {
    if !matches!(baseline.len(), 40 | 64) || !baseline.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("Invalid coverage baseline object ID");
    }
    ops::checked(ops::git(root)?.args(["merge-base", "--is-ancestor", baseline, "HEAD"]))?;
    let mut paths = BTreeSet::new();
    for revisions in [vec![baseline, "HEAD"], vec!["--cached", "HEAD"], vec![]] {
        let output = ops::output(
            ops::git(root)?
                .args([
                    "diff",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-renames",
                    "--name-only",
                    "-z",
                ])
                .args(revisions)
                .arg("--"),
        )?;
        add_paths(&output, &mut paths)?;
    }
    add_paths(
        &ops::output(ops::git(root)?.args(["ls-files", "--others", "--exclude-standard", "-z"]))?,
        &mut paths,
    )?;
    Ok(paths)
}

fn add_paths(bytes: &[u8], paths: &mut BTreeSet<String>) -> Result<()> {
    for path in bytes.split(|byte| *byte == 0).filter(|path| !path.is_empty()) {
        paths.insert(
            std::str::from_utf8(path)
                .or_raise(|| message("Coverage paths must be UTF-8"))?
                .into(),
        );
    }
    Ok(())
}

fn wrapper_config(root: &Path, raw: &Path) -> Result<PathBuf> {
    let path = root.join(".config/nextest.toml");
    let mut config = match fs::read_to_string(path) {
        Ok(config) => config.parse::<toml_edit::DocumentMut>().or_error()?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => toml_edit::DocumentMut::new(),
        Err(error) => return Err(error).or_error(),
    };
    if config.get("scripts").is_some() || std::env::var_os("NEXTEST_PROFILE").is_some() {
        bail!("Coverage collection retains existing custom Nextest scripts/profiles without replacing them");
    }
    let experimental = config
        .entry("experimental")
        .or_insert(toml_edit::value(toml_edit::Array::new()))
        .as_array_mut()
        .ok_or_raise(|| message("Nextest experimental options must be an array"))?;
    if !experimental
        .iter()
        .any(|value| value.as_str() == Some("wrapper-scripts"))
    {
        experimental.push("wrapper-scripts");
    }
    let mut args = toml_edit::Array::new();
    for arg in [
        std::env::current_exe().or_error()?,
        "exec".into(),
        "coverage-test".into(),
        raw.into(),
    ] {
        args.push(
            arg.to_str()
                .ok_or_raise(|| message("Coverage wrapper paths must be UTF-8"))?,
        );
    }
    config["scripts"]["wrapper"]["gix-coverage"]["command"] = toml_edit::value(args);
    config["scripts"]["wrapper"]["gix-coverage"]["target-runner"] = toml_edit::value("within-wrapper");
    let mut rule = toml_edit::Table::new();
    rule["filter"] = toml_edit::value("all()");
    rule["run-wrapper"] = toml_edit::value("gix-coverage");
    let mut rules = toml_edit::ArrayOfTables::new();
    rules.push(rule);
    config["profile"]["default"]["scripts"] = toml_edit::Item::ArrayOfTables(rules);
    fs::create_dir_all(raw).or_error()?;
    let path = raw.join("nextest.toml");
    File::create(&path)
        .or_error()?
        .write_all(config.to_string().as_bytes())
        .or_error()?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn summary_tracks_execution_after_selection_and_fallback() -> gix_error::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let fixture = gix_testtools::scripted_fixture_read_only("make_qa_recording_repositories.sh")?;
        let root = fixture.join("clean");
        let temporary = gix_testtools::tempfile::tempdir()?;
        let directory = temporary.path();
        let test = json!({"kind":"test", "ignored":false, "filter-match":{"status":"matches"}});
        let mut inventory = json!({"rust-suites":{"suite":{
            "binary-id":"suite", "status":"listed", "testcases":{"a":test, "b":test, "ignored":test}
        }}});
        inventory["rust-suites"]["suite"]["testcases"]["ignored"]["ignored"] = true.into();
        fs::write(directory.join("inventory.json"), serde_json::to_vec(&inventory)?)?;
        fs::write(
            directory.join("nextest"),
            "case $1 in
list) cat inventory.json ;;
run) test ! -e fail ;;
esac
",
        )?;
        let mut command = Command::new("sh");
        command.current_dir(directory).args(["nextest", "run"]);
        let configuration = json!({"build":{"env":{"RUSTUP_TOOLCHAIN":"stable"}}});
        let index = json!({"version":1, "configuration":configuration, "baseline":head(&root)?,
        "files":["a.rs", "b.rs"], "tests":[
            {"binary":"suite", "name":"a", "files":[0]},
            {"binary":"suite", "name":"b", "files":[1]}
        ]});
        fs::create_dir(directory.join("indexes"))?;
        fs::create_dir(directory.join("work"))?;
        let index_path = directory.join("indexes").join(format!("{}.json", key(&configuration)));
        fs::write(&index_path, serde_json::to_vec(&index)?)?;
        let mut coverage = Run {
            mode: Mode::Prune,
            directory: directory.into(),
            work: directory.join("work"),
            compiler: String::new(),
            cargo_config: Value::Null,
            tools: None,
            baseline: None,
            pending: BTreeMap::new(),
            changed: BTreeSet::from(["a.rs".into()]),
            dirty: BTreeSet::new(),
            source: None,
            cases: 0,
            summary: Summary::new(Mode::Prune),
        };
        for (mode, expected_executed, expected_omitted) in [(Mode::Prune, 1, 1), (Mode::Shadow, 2, 0)] {
            coverage.mode = mode;
            coverage.summary = Summary::new(mode);
            coverage.execute(&root, configuration.clone(), &mut copy_command(&command), false, None)?;
            let summary = &coverage.summary;
            assert_eq!((summary.cases, summary.measured_cases, summary.reused), (1, 1, 1));
            assert_eq!(
                (summary.executed, summary.omitted),
                (expected_executed, expected_omitted),
                "shadow reports actual execution, while prune records only applied omissions"
            );
            assert!(summary.inventory_time > Duration::ZERO && summary.selection_time > Duration::ZERO);
        }
        fs::remove_file(index_path)?;
        coverage.mode = Mode::Prune;
        coverage.summary = Summary::new(Mode::Prune);
        coverage.execute(&root, configuration.clone(), &mut copy_command(&command), false, None)?;
        assert_eq!(
            (
                coverage.summary.executed,
                coverage.summary.omitted,
                coverage.summary.fallback
            ),
            (2, 0, 1),
            "a missing index runs and counts the complete active inventory"
        );
        fs::write(directory.join("inventory.json"), b"{}")?;
        coverage.execute(&root, configuration.clone(), &mut copy_command(&command), false, None)?;
        assert_eq!(
            (
                coverage.summary.cases,
                coverage.summary.measured_cases,
                coverage.summary.executed
            ),
            (2, 1, 2),
            "unavailable inventories do not invent a test count for successful fallback cases"
        );
        fs::write(directory.join("fail"), b"")?;
        assert!(
            coverage
                .execute(&root, configuration, &mut copy_command(&command), false, None)
                .is_err(),
            "fallback never hides test failures"
        );
        assert_eq!(
            coverage.summary.cases, 2,
            "failed executions do not count as completed cases"
        );
        command.arg("--run-ignored=all");
        assert_eq!(
            inventory_count(&inventory, &command),
            None,
            "custom ignored selection has no guessed total"
        );
        Ok(())
    }

    #[test]
    fn reverted_capture_edits_and_the_requested_base_remain_affected() -> gix_error::TestResult {
        let data = json!({"version":2, "dirty_paths":["a.rs"], "files":["a.rs","b.rs"], "tests":[
            {"binary":"suite", "name":"a", "files":[0]},
            {"binary":"suite", "name":"b", "files":[1]}
        ]});
        let test = json!({"kind":"test", "ignored":false, "filter-match":{"status":"matches"}});
        let inventory =
            json!({"rust-suites":{"suite":{"binary-id":"suite","status":"listed","testcases":{"a":test,"b":test}}}});
        let paths = affected_paths(&data, BTreeSet::new(), &BTreeSet::from(["b.rs".into()]))?;
        let selection = index::select(&data, &inventory, &paths)?;
        assert_eq!(
            selection.selected, 2,
            "the reverted capture edit and the requested QA change both keep their tests"
        );
        assert_eq!(
            recorded_dirty(&json!({"version":1}))?,
            BTreeSet::new(),
            "legacy indexes were published only for clean commits"
        );
        assert!(
            recorded_dirty(&json!({"version":2})).is_err(),
            "dirty-capable indexes require their capture inputs"
        );
        assert!(recorded_dirty(&json!({"version":2,"dirty_paths":[false]})).is_err());
        assert!(
            needs_refresh(&json!({}), 7),
            "legacy unusable indexes get one automatic refresh"
        );
        assert!(
            needs_refresh(&json!({"source_hash":6}), 7),
            "changed source permits a new collection"
        );
        assert!(
            !needs_refresh(&json!({"source_hash":7}), 7),
            "unchanged unknown/disabled files do not cause repeated collection"
        );
        Ok(())
    }

    #[test]
    fn selected_replays_replace_union_filters_and_keep_the_environment() -> gix_error::TestResult {
        let mut original = Command::new("cargo");
        original
            .args([
                "nextest",
                "run",
                "--cargo-metadata",
                "source path/cargo.json",
                "--workspace-remap",
                "source path",
                "--no-fail-fast",
                "--no-tests",
                "pass",
                "-E",
                "package(a)",
                "--filter-expr=test(foo)",
            ])
            .env("GIX_TEST_FIXTURE_HASH", "sha256")
            .env_remove("GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI");
        let selected = filtered(&original, "none()")?;
        let args: Vec<_> = selected.get_args().filter_map(OsStr::to_str).collect();
        assert_eq!(
            args.iter().filter(|arg| **arg == "-E").count(),
            1,
            "Nextest unions repeated filters"
        );
        assert!(!args.contains(&"package(a)") && !args.contains(&"--filter-expr=test(foo)"));
        assert_eq!(args.iter().filter(|arg| arg.starts_with("--no-tests")).count(), 1);
        assert!(
            args.contains(&"source path/cargo.json"),
            "paths stay separate process arguments"
        );
        assert_eq!(
            selected.get_envs().collect::<Vec<_>>(),
            original.get_envs().collect::<Vec<_>>()
        );
        let listed = replay_args(&original, true, None)?;
        assert!(
            listed.contains(&"package(a)".into()),
            "the inventory enforces original filters first"
        );
        assert!(!listed.contains(&"--no-fail-fast".into()) && !listed.contains(&"--no-tests".into()));
        assert!(replay_args(&Command::new("cargo"), true, None).is_err());
        original.arg("--run-ignored=all");
        assert!(
            filtered(&original, "none()").is_err(),
            "unsupported ignored-test selection keeps ordinary execution"
        );
        Ok(())
    }

    #[test]
    fn large_filter_files_preserve_settings_and_keep_command_arguments_small() -> gix_error::TestResult {
        let root = gix_testtools::tempfile::tempdir()?;
        fs::create_dir(root.path().join(".config"))?;
        let original_config = r#"
[profile.default]
test-threads = 3
fail-fast = false
default-filter = "not test(slow)"

[profile.default.junit]
path = "results.xml"

[[profile.default.overrides]]
filter = "test(first)"
priority = 10
"#;
        let config_path = root.path().join(".config/nextest.toml");
        fs::write(&config_path, original_config)?;
        let mut command = Command::new("cargo");
        command
            .args(["nextest", "run", "-E", "package(a)"])
            .env_remove("NEXTEST_PROFILE");
        let filter = (0..1000)
            .map(|id| format!("test(=long_module::test_{id})"))
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(filter.len() > INLINE_FILTER_LIMIT);
        assert!(
            filtered(&command, &filter).is_err(),
            "large filters cannot enter argv directly"
        );
        let directory = root.path().join("selection");
        let selected = filtered_with_file(root.path(), &directory, &command, &filter)?;
        let args: Vec<_> = selected.get_args().filter_map(OsStr::to_str).collect();
        assert!(args.iter().all(|arg| arg.len() <= INLINE_FILTER_LIMIT));
        assert!(args.windows(2).any(|args| args == ["-E", "all()"]));
        assert!(args.contains(&"--config-file") && !args.contains(&"package(a)"));
        let config = fs::read_to_string(directory.join("selection.toml"))?.parse::<toml_edit::DocumentMut>()?;
        assert_eq!(
            config["profile"]["default"]["default-filter"].as_str(),
            Some(filter.as_str())
        );
        assert_eq!(config["profile"]["default"]["test-threads"].as_integer(), Some(3));
        assert_eq!(config["profile"]["default"]["fail-fast"].as_bool(), Some(false));
        assert_eq!(
            config["profile"]["default"]["junit"]["path"].as_str(),
            Some("results.xml")
        );
        assert_eq!(
            config["profile"]["default"]["overrides"][0]["priority"].as_integer(),
            Some(10)
        );
        assert_eq!(
            fs::read_to_string(&config_path)?,
            original_config,
            "the source configuration is untouched"
        );
        let short_directory = root.path().join("short");
        filtered_with_file(root.path(), &short_directory, &command, "none()")?;
        assert!(!short_directory.exists(), "small filters need no additional file");
        for arg in [
            "--ignore-default-filter",
            "--profile=with-xml",
            "-Pwith-xml",
            "--config-file=custom.toml",
            "--tool-config-file=tool:custom.toml",
        ] {
            let mut unsupported = copy_command(&command);
            unsupported.arg(arg);
            assert!(filtered_with_file(root.path(), &directory, &unsupported, &filter).is_err());
        }
        command.env("NEXTEST_PROFILE", "with-xml");
        assert!(filtered_with_file(root.path(), &directory, &command, &filter).is_err());
        command.env_remove("NEXTEST_PROFILE");
        for config in [
            "[[profile.default.overrides]]
platform = 'cfg(unix)'
default-filter = 'none()'
",
            "[profile.default]
overrides = [{ platform = 'cfg(unix)', default-filter = 'none()' }]
",
        ] {
            fs::write(&config_path, config)?;
            assert!(
                filtered_with_file(root.path(), &directory, &command, &filter).is_err(),
                "platform default filters cannot override the generated selection"
            );
        }
        Ok(())
    }

    #[test]
    fn identities_ignore_disposable_paths_but_keep_feature_and_fixture_choices() -> gix_error::TestResult {
        let mut first = Command::new("cargo");
        first
            .args([
                "nextest",
                "run",
                "--workspace-remap",
                "snapshot-a",
                "--features",
                "sha1",
            ])
            .env("GIX_TEST_FIXTURE_SOURCE_ROOT", "snapshot-a")
            .env("GIX_TEST_FIXTURE_HASH", "sha1");
        let mut other = Command::new("cargo");
        other
            .args([
                "nextest",
                "run",
                "--workspace-remap",
                "snapshot-b",
                "--features",
                "sha1",
            ])
            .env("GIX_TEST_FIXTURE_SOURCE_ROOT", "snapshot-b")
            .env("GIX_TEST_FIXTURE_HASH", "sha1");
        assert_eq!(
            signature(&first),
            signature(&other),
            "fresh snapshots can reuse the same configuration"
        );
        other.env("GIX_TEST_FIXTURE_HASH", "sha256");
        assert_ne!(
            signature(&first),
            signature(&other),
            "runtime fixture inputs are part of identity"
        );
        other.env("GIX_TEST_FIXTURE_HASH", "sha1").arg("--no-default-features");
        assert_ne!(
            signature(&first),
            signature(&other),
            "feature configurations cannot borrow each other's coverage"
        );
        assert!(supported_compiler(&json!({"env":{"RUSTUP_TOOLCHAIN":"nightly"}})).is_err());
        assert!(supported_compiler(&json!({"env":{"RUSTUP_TOOLCHAIN":"stable","RUSTC":"custom"}})).is_err());
        supported_compiler(&json!({"env":{"RUSTUP_TOOLCHAIN":"stable"}}))?;
        Ok(())
    }

    #[test]
    fn collection_preserves_nextest_settings_and_declines_custom_scripts() -> gix_error::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = gix_testtools::Env::new().unset("NEXTEST_PROFILE");
        let root = gix_testtools::tempfile::tempdir()?;
        fs::create_dir(root.path().join(".config"))?;
        fs::write(
            root.path().join(".config/nextest.toml"),
            "[profile.default]\ntest-threads = 3\nfail-fast = false\n",
        )?;
        let path = wrapper_config(root.path(), &root.path().join("raw"))?;
        let config = fs::read_to_string(path)?.parse::<toml_edit::DocumentMut>()?;
        assert_eq!(config["profile"]["default"]["test-threads"].as_integer(), Some(3));
        assert_eq!(config["profile"]["default"]["fail-fast"].as_bool(), Some(false));
        assert_eq!(
            config["scripts"]["wrapper"]["gix-coverage"]["target-runner"].as_str(),
            Some("within-wrapper")
        );
        for profile in ["default", "with-xml"] {
            let _profile = gix_testtools::Env::new().set("NEXTEST_PROFILE", profile);
            assert!(
                wrapper_config(root.path(), &root.path().join("profile")).is_err(),
                "an explicitly selected profile is not silently replaced"
            );
        }
        fs::write(
            root.path().join(".config/nextest.toml"),
            "[scripts.wrapper.custom]\ncommand = 'required-wrapper'\n",
        )?;
        assert!(
            wrapper_config(root.path(), &root.path().join("other")).is_err(),
            "an existing wrapper is not silently replaced"
        );
        assert!(
            Run::optional(root.path(), Some(Mode::Off), true, None).is_none(),
            "disabled coverage needs no tool discovery"
        );
        assert!(Run::optional(root.path(), Some(Mode::Auto), false, None).is_none());
        Ok(())
    }
}
