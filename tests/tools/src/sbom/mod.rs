use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsString,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use gix_testtools::Result;
use serde_json::{Value, json};

#[derive(Default)]
struct Options {
    package: Option<String>,
    features: Vec<String>,
    no_default_features: bool,
    all_features: bool,
    target: Option<String>,
    output_dir: Option<PathBuf>,
}

impl Options {
    fn parse(mut args: impl Iterator<Item = OsString>) -> Result<Option<Self>> {
        let mut options = Self::default();
        while let Some(arg) = args.next() {
            let arg = arg.to_str().ok_or("option is not UTF-8")?;
            match arg {
                "--help" | "-h" => {
                    println!(
                        "Usage: jtt sbom [--package NAME] [OPTIONS]\n\
                         Without --package: all workspace members, all features, all platforms.\n\
                         With --package: default features on the host platform.\n\n\
                         --features LIST         Comma/space separated package features (repeatable)\n\
                         --no-default-features   Disable the package's default features\n\
                         --all-features          Enable every package feature\n\
                         --target TRIPLE|all     Override the platform selection\n\
                         --output-dir PATH       Default: <Cargo target directory>/sbom\n\n\
                         Writes <workspace-or-package>.cdx.json and .spdx.json.\n\
                         Includes runtime and build dependencies; excludes dev-only dependencies.\n\
                         Install the required Cargo tools with `just sbom-install`."
                    );
                    return Ok(None);
                }
                "--no-default-features" => options.no_default_features = true,
                "--all-features" => options.all_features = true,
                "--package" | "--features" | "--target" | "--output-dir" => {
                    let value = args.next().ok_or_else(|| format!("{arg} requires a value"))?;
                    if arg == "--output-dir" {
                        options.output_dir = Some(value.into());
                        continue;
                    }
                    let value = value.into_string().map_err(|_| format!("{arg} requires UTF-8"))?;
                    match arg {
                        "--package" => options.package = Some(value),
                        "--target" => options.target = Some(value),
                        _ => options.features.extend(
                            value
                                .split([',', ' '])
                                .filter(|feature| !feature.is_empty())
                                .map(str::to_owned),
                        ),
                    }
                }
                _ => return Err(format!("unknown SBOM option {arg:?}; see `jtt sbom --help`").into()),
            }
        }
        if options.package.is_none()
            && (options.no_default_features || options.all_features || !options.features.is_empty())
        {
            return Err(
                "feature selection requires --package; workspace inventories always enable all features".into(),
            );
        }
        Ok(Some(options))
    }
}

pub fn run(args: impl Iterator<Item = OsString>) -> Result {
    let Some(options) = Options::parse(args)? else {
        return Ok(());
    };
    let metadata: Value = serde_json::from_slice(&output(cargo().args([
        "metadata",
        "--locked",
        "--all-features",
        "--format-version",
        "1",
    ]))?)?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("Cargo metadata has no packages")?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("Cargo metadata has no workspace members")?;
    let selected: Vec<_> = packages
        .iter()
        .filter(|package| {
            members.contains(&package["id"]) && options.package.as_ref().is_none_or(|name| package["name"] == *name)
        })
        .collect();
    if selected.is_empty() {
        return Err(format!("unknown workspace package {:?}", options.package).into());
    }
    let output_dir = options
        .output_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from(metadata["target_directory"].as_str().unwrap_or("target")).join("sbom"));
    fs::create_dir_all(&output_dir)?;
    let temporary = gix_testtools::tempfile::Builder::new()
        .prefix(".sbom-")
        .tempdir_in(&output_dir)?;
    let mut binary_manifests = BTreeMap::new();
    let mut manifest = String::from(
        "[package]\nname = \"gitoxide-sbom-scope\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
         [workspace]\nresolver = \"2\"\n[dependencies]\n",
    );
    for package in &selected {
        let available = package["features"]
            .as_object()
            .ok_or("Cargo metadata has no feature map")?;
        for feature in &options.features {
            if !available.contains_key(feature) {
                return Err(format!("unknown feature {feature:?} for {}", package["name"]).into());
            }
        }
        let features: Vec<_> = if options.package.is_none() || options.all_features {
            available.keys().collect()
        } else {
            options.features.iter().collect()
        };
        let mut path = PathBuf::from(
            package["manifest_path"]
                .as_str()
                .ok_or("Cargo metadata has no manifest path")?,
        );
        if !has_target(package, &["lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"])? {
            // Cargo ignores binary-only path dependencies. Give them a disposable
            // library target while retaining their dependencies and feature wiring.
            path = binary_manifest(package, temporary.path())?;
            binary_manifests.insert(path.clone(), *package);
        }
        // JSON strings and arrays of strings also work as TOML basic strings and arrays.
        writeln!(
            manifest,
            "{} = {{ path = {}, default-features = {}, features = {} }}",
            package["name"],
            serde_json::to_string(&path.parent().ok_or("manifest has no parent directory")?)?,
            !options.no_default_features,
            json!(features)
        )?;
    }
    // Keep the scope beside the binary adapters so Cargo cannot auto-enroll
    // the adapters as additional workspace members and unify their features.
    let scope = temporary.path().join("scope");
    fs::create_dir(&scope)?;
    let manifest_path = scope.join("Cargo.toml");
    fs::write(&manifest_path, manifest)?;
    fs::create_dir(scope.join("src"))?;
    fs::write(scope.join("src/lib.rs"), "")?;
    let workspace = PathBuf::from(
        metadata["workspace_root"]
            .as_str()
            .ok_or("Cargo metadata has no workspace root")?,
    );
    fs::copy(workspace.join("Cargo.lock"), scope.join("Cargo.lock"))?;

    // A separate workspace prevents unrelated members (and the selected package's
    // dev-dependencies) from enabling features. Only the copied lockfile may change.
    let resolved: Value = serde_json::from_slice(&output(
        cargo()
            .args([
                "metadata",
                "--offline",
                "--format-version",
                "1",
                "--no-default-features",
                "--manifest-path",
            ])
            .arg(&manifest_path),
    )?)?;
    let locked: BTreeSet<_> = packages
        .iter()
        .map(|package| (package["id"].as_str(), package["version"].as_str()))
        .collect();
    let mut original_ids = BTreeMap::new();
    for package in resolved["packages"]
        .as_array()
        .ok_or("Cargo metadata has no packages")?
    {
        let path = Path::new(
            package["manifest_path"]
                .as_str()
                .ok_or("Cargo metadata has no manifest path")?,
        );
        if let Some(original) = binary_manifests.get(path) {
            original_ids.insert(
                package["id"].as_str().ok_or("Cargo package has no ID")?.to_owned(),
                original["id"].clone(),
            );
            continue;
        }
        if package["id"] != resolved["resolve"]["root"]
            && !locked.contains(&(package["id"].as_str(), package["version"].as_str()))
        {
            return Err(format!(
                "isolated resolution changed locked package {}; source Cargo.lock was not modified",
                package["id"]
            )
            .into());
        }
    }
    let target = match options.target.as_deref() {
        Some(target) => target.to_owned(),
        None if options.package.is_none() => "all".into(),
        None => {
            let version = output(Command::new(env::var_os("RUSTC").unwrap_or_else(|| "rustc".into())).arg("-vV"))?;
            String::from_utf8(version)?
                .lines()
                .find_map(|line| line.strip_prefix("host: "))
                .ok_or("rustc did not report its host platform")?
                .to_owned()
        }
    };
    output(
        cargo()
            .args([
                "cyclonedx",
                "--format",
                "json",
                "--spec-version",
                "1.5",
                "--override-filename",
                "bom",
                "--all",
                "--no-default-features",
                "--target",
                "all",
                "--manifest-path",
            ])
            .arg(&manifest_path),
    )?;
    let bom_path = scope.join("bom.json");
    let mut bom: Value = serde_json::from_slice(&fs::read(&bom_path)?)?;
    select_target_graph(&mut bom, &resolved, &manifest_path, &target)?;
    restore_binary_ids(&mut bom, &original_ids)?;
    set_root(&mut bom, options.package.as_ref().map(|_| selected[0]))?;
    fs::write(&bom_path, serde_json::to_vec_pretty(&bom)?)?;
    let spdx = output(
        Command::new("sbom-tools")
            .args(["convert", "--to", "spdx"])
            .arg(&bom_path),
    )?;
    serde_json::from_slice::<Value>(&spdx)?;
    let spdx_path = temporary.path().join("spdx.json");
    fs::write(&spdx_path, spdx)?;

    // Publish only after both generators succeed, leaving previous results intact on tool failure.
    let name = options.package.as_deref().unwrap_or("workspace");
    for (source, suffix) in [(bom_path, "cdx"), (spdx_path, "spdx")] {
        let destination = output_dir.join(format!("{name}.{suffix}.json"));
        fs::rename(source, &destination)?;
        println!("{}", destination.display());
    }
    Ok(())
}

fn select_target_graph(bom: &mut Value, metadata: &Value, manifest: &Path, target: &str) -> Result {
    // `cargo metadata --filter-platform` leaves features from other platforms enabled.
    // Generate a superset with CycloneDX, then use Cargo tree's resolved graph for
    // both the requested target and its host build dependencies.
    let tree = output(
        cargo()
            .args([
                "tree",
                "--locked",
                "--offline",
                "--no-default-features",
                "--edges",
                "normal,build",
                "--prefix",
                "depth",
                "--format",
                "|{p}",
                "--color",
                "never",
                "--target",
                target,
                "--manifest-path",
            ])
            .arg(manifest),
    )?;
    let mut ids: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for package in metadata["packages"]
        .as_array()
        .ok_or("Cargo metadata has no packages")?
    {
        let name = package["name"].as_str().ok_or("Cargo package has no name")?;
        let version = package["version"].as_str().ok_or("Cargo package has no version")?;
        ids.entry(format!("{name} v{version}"))
            .or_default()
            .push(package["id"].as_str().ok_or("Cargo package has no ID")?);
    }
    let mut edges: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut parents = Vec::new();
    for line in std::str::from_utf8(&tree)?.lines().filter(|line| !line.is_empty()) {
        let (depth, package) = line.split_once('|').ok_or("unexpected Cargo tree output")?;
        let depth: usize = depth.parse()?;
        let identity = package.split_once(" (").map_or(package, |(identity, _)| identity);
        let candidates = ids
            .get(identity)
            .ok_or_else(|| format!("unknown Cargo tree package {identity:?}"))?;
        // ponytail: Cargo tree has no JSON output; reject ambiguous name/version pairs until it exposes package IDs.
        let [id] = candidates.as_slice() else {
            return Err(format!("ambiguous Cargo tree package {identity:?}").into());
        };
        parents.truncate(depth);
        if parents.len() != depth {
            return Err("unexpected Cargo tree depth".into());
        }
        edges.entry(*id).or_default();
        if let Some(parent) = parents.last() {
            edges.entry(*parent).or_default().insert(*id);
        }
        parents.push(*id);
    }
    let root = bom["metadata"]["component"]["bom-ref"].clone();
    let components = bom["components"].as_array_mut().ok_or("CycloneDX has no components")?;
    components.retain(|component| component["bom-ref"].as_str().is_some_and(|id| edges.contains_key(id)));
    let references: BTreeSet<_> = components
        .iter()
        .map(|component| component["bom-ref"].as_str())
        .chain(std::iter::once(root.as_str()))
        .collect();
    if edges.is_empty() || edges.keys().any(|id| !references.contains(&Some(*id))) {
        return Err("Cargo tree contains packages absent from CycloneDX".into());
    }
    bom["dependencies"] = json!(
        edges
            .into_iter()
            .map(|(id, dependencies)| json!({"ref": id, "dependsOn": dependencies}))
            .collect::<Vec<_>>()
    );
    bom["metadata"]["properties"] = json!([{"name": "cdx:rustc:sbom:target:triple", "value": target}]);
    Ok(())
}

fn set_root(bom: &mut Value, package: Option<&Value>) -> Result {
    let temporary_ref = bom["metadata"]["component"]["bom-ref"]
        .as_str()
        .ok_or("CycloneDX root has no reference")?
        .to_owned();
    let root = if let Some(package) = package {
        let components = bom["components"].as_array_mut().ok_or("CycloneDX has no components")?;
        let index = components
            .iter()
            .position(|component| component["bom-ref"] == package["id"])
            .ok_or("selected package is absent from CycloneDX")?;
        let mut root = components.remove(index);
        let binary = has_target(package, &["bin"])?;
        root["type"] = json!(if binary { "application" } else { "library" });
        root
    } else {
        json!({"type": "application", "name": "gitoxide-workspace", "bom-ref": "gitoxide-workspace"})
    };
    let dependencies = bom["dependencies"]
        .as_array_mut()
        .ok_or("CycloneDX has no dependency graph")?;
    if package.is_some() {
        dependencies.retain(|dependency| dependency["ref"] != temporary_ref);
    } else {
        for dependency in dependencies {
            if dependency["ref"] == temporary_ref {
                dependency["ref"] = root["bom-ref"].clone();
            }
        }
    }
    bom["metadata"]["component"] = root;
    Ok(())
}

fn has_target(package: &Value, kinds: &[&str]) -> Result<bool> {
    Ok(package["targets"]
        .as_array()
        .ok_or("Cargo metadata has no targets")?
        .iter()
        .any(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|actual| kinds.iter().any(|kind| actual.contains(&json!(kind))))
        }))
}

fn binary_manifest(package: &Value, directory: &Path) -> Result<PathBuf> {
    let directory = directory.join(package["name"].as_str().ok_or("Cargo package has no name")?);
    fs::create_dir(&directory)?;
    let mut manifest = String::from("[workspace]\n[lib]\npath = \"lib.rs\"\n[package]\n");
    for key in [
        "name",
        "version",
        "edition",
        "description",
        "authors",
        "license",
        "repository",
        "homepage",
        "documentation",
    ] {
        if !package[key].is_null() {
            writeln!(manifest, "{key} = {}", package[key])?;
        }
    }
    for dependency in package["dependencies"]
        .as_array()
        .ok_or("Cargo package has no dependencies")?
    {
        // ponytail: binary adapters support path/crates.io dependencies; add other source types when used here.
        if !dependency["registry"].is_null()
            || dependency["source"]
                .as_str()
                .is_some_and(|source| source != "registry+https://github.com/rust-lang/crates.io-index")
        {
            return Err(format!(
                "unsupported source for binary package dependency {}",
                dependency["name"]
            )
            .into());
        }
        let kind = match dependency["kind"].as_str() {
            Some("build") => "build-dependencies",
            Some("dev") => "dev-dependencies",
            _ => "dependencies",
        };
        let target = if dependency["target"].is_null() {
            String::new()
        } else {
            format!("target.{}.", dependency["target"])
        };
        let name = if dependency["rename"].is_null() {
            &dependency["name"]
        } else {
            &dependency["rename"]
        };
        writeln!(
            manifest,
            "[{target}{kind}.{name}]\npackage = {}\nversion = {}\noptional = {}\ndefault-features = {}\nfeatures = {}",
            dependency["name"],
            dependency["req"],
            dependency["optional"],
            dependency["uses_default_features"],
            dependency["features"]
        )?;
        if !dependency["path"].is_null() {
            writeln!(manifest, "path = {}", dependency["path"])?;
        }
    }
    manifest.push_str("[features]\n");
    for (name, features) in package["features"].as_object().ok_or("Cargo package has no features")? {
        writeln!(manifest, "{} = {features}", json!(name))?;
    }
    fs::write(directory.join("lib.rs"), "")?;
    fs::write(directory.join("Cargo.toml"), manifest)?;
    Ok(directory.join("Cargo.toml").canonicalize()?)
}

fn restore_binary_ids(bom: &mut Value, originals: &BTreeMap<String, Value>) -> Result {
    for component in bom["components"].as_array_mut().ok_or("CycloneDX has no components")? {
        if let Some(original) = component["bom-ref"].as_str().and_then(|id| originals.get(id)) {
            component["bom-ref"] = original.clone();
            component["type"] = json!("application");
            component["purl"] = json!(format!(
                "pkg:cargo/{}@{}",
                component["name"].as_str().ok_or("component has no name")?,
                component["version"].as_str().ok_or("component has no version")?
            ));
        }
    }
    for dependency in bom["dependencies"]
        .as_array_mut()
        .ok_or("CycloneDX has no dependency graph")?
    {
        if let Some(original) = dependency["ref"].as_str().and_then(|id| originals.get(id)) {
            dependency["ref"] = original.clone();
        }
        if let Some(targets) = dependency["dependsOn"].as_array_mut() {
            for target in targets {
                if let Some(original) = target.as_str().and_then(|id| originals.get(id)) {
                    *target = original.clone();
                }
            }
        }
    }
    Ok(())
}

fn cargo() -> Command {
    Command::new(env::var_os("CARGO").unwrap_or_else(|| OsString::from(env!("CARGO"))))
}

fn output(command: &mut Command) -> Result<Vec<u8>> {
    let output = command
        .output()
        .map_err(|err| format!("could not run {command:?}: {err}; see `just sbom-install`"))?;
    if !output.status.success() {
        return Err(format!(
            "{command:?} failed ({}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    Ok(output.stdout)
}
