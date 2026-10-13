//! Release guard: let cargo-semver-checks compare APIs, then check nominal dependency identities separately.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use gix_testtools::Result;
use serde_json::Value;

fn main() {
    let result = run();
    match result {
        Ok(compatible) => std::process::exit(if compatible { 0 } else { 10 }),
        Err(err) => {
            eprintln!("API check could not complete: {err}");
            std::process::exit(2);
        }
    }
}

fn run() -> Result<bool> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "audit") {
        let path = args.get(1).ok_or("audit requires a rustdoc JSON path")?;
        let document = serde_json::from_reader(fs::File::open(path)?)?;
        let dependencies = public_dependencies(&document)?;
        println!("{}", serde_json::to_string_pretty(&dependencies)?);
        if let Some(allowed) = args.get(2) {
            let allowed = allowed.to_str().ok_or("allowlist is not UTF-8")?;
            return Ok(dependencies
                .keys()
                .all(|name| allowed.split(',').any(|value| value == name)));
        }
        return Ok(true);
    }
    let baseline_override = match args.as_slice() {
        [] => None,
        [flag, path] if flag == "--baseline-root" => Some(PathBuf::from(path).canonicalize()?),
        _ => return Err("expected no arguments, --baseline-root PATH, or audit JSON [CRATE,...]".into()),
    };
    let request: Value = serde_json::from_reader(io::stdin().lock())?;
    let package = required_str(&request, "package")?;
    let manifest = Path::new(required_str(&request, "manifest_path")?).canonicalize()?;
    let metadata = cargo_metadata(&manifest, &[])?;
    let package_metadata = find_package(&metadata, package, &manifest)?;
    let root = Path::new(required_str(&metadata, "workspace_root")?);
    let profiles = package_metadata["metadata"]["api-check"]["profiles"]
        .as_array()
        .filter(|profiles| !profiles.is_empty())
        .ok_or("package.metadata.api-check.profiles must describe the supported stable feature profiles")?;
    let unstable = string_array(&request["unstable_features"])?;
    let dependencies = request["dependencies"]
        .as_array()
        .ok_or("dependencies must be an array")?;
    let baseline_dir = tempfile::tempdir()?;
    let baseline_root = match baseline_override.as_deref() {
        Some(path) => path,
        None => {
            archive_baseline(root, required_str(&request, "baseline_revision")?, baseline_dir.path())?;
            baseline_dir.path()
        }
    };
    let baseline_manifest = baseline_root.join(manifest.strip_prefix(root)?);
    let mut compatible = true;
    for (profile_index, profile) in profiles.iter().enumerate() {
        let feature_args = feature_args(profile, &package_metadata["features"], &unstable)?;
        eprintln!("Checking {package}, features: {feature_args:?}");
        let current_metadata = cargo_metadata(&manifest, &feature_args)?;
        let baseline_metadata = cargo_metadata(&baseline_manifest, &feature_args)?;
        let current_json = baseline_dir.path().join(format!("current-{profile_index}.json"));
        let baseline_json = baseline_dir.path().join(format!("baseline-{profile_index}.json"));
        let document = rustdoc_document(&manifest, &feature_args, &current_metadata, package, &current_json)?;
        let before = rustdoc_document(
            &baseline_manifest,
            &feature_args,
            &baseline_metadata,
            package,
            &baseline_json,
        )?;
        let exposed = public_dependencies(&document)?;
        eprintln!(
            "Public dependency crates: {}",
            exposed.keys().cloned().collect::<Vec<_>>().join(", ")
        );
        for dependency in dependencies {
            let name = required_str(dependency, "name")?;
            let Some(crate_name) = planned_dependency_crate(&current_metadata, name)? else {
                continue;
            };
            if dependency["breaking"]
                .as_bool()
                .ok_or("dependency.breaking must be a boolean")?
                && let Some(paths) = exposed.get(crate_name)
            {
                eprintln!(
                    "Breaking public dependency {name}: {}",
                    paths.iter().cloned().collect::<Vec<_>>().join(", ")
                );
                compatible = false;
            }
        }
        compatible &= unchanged_identities(&exposed, &baseline_metadata, &current_metadata)?;
        compatible &= unchanged_types(&before, &document)?;
        let mut semver = Command::new("cargo");
        semver
            .args(["semver-checks", "check-release", "--current-rustdoc"])
            .arg(&current_json)
            .arg("--baseline-rustdoc")
            .arg(&baseline_json)
            .args(["--release-type", "minor", "--color", "never"]);
        let output = semver.output()?;
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
        if !output.status.success() {
            // cargo-semver-checks 0.51+ distinguishes detected breaks from tool failures.
            if output.status.code() == Some(100) {
                compatible = false;
            } else {
                return Err(format!(
                    "cargo-semver-checks failed without a completed compatibility verdict: {}",
                    output.status
                )
                .into());
            }
        }
    }
    Ok(compatible)
}

fn rustdoc_document(
    manifest: &Path,
    features: &[String],
    metadata: &Value,
    package: &str,
    output: &Path,
) -> Result<Value> {
    let rustc = Command::new("rustc").arg("-vV").output()?;
    if !rustc.status.success() {
        return Err("could not determine the rustc host target".into());
    }
    let version = String::from_utf8(rustc.stdout)?;
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or("rustc -vV has no host target")?;
    let path = Path::new(required_str(metadata, "target_directory")?)
        .join(host)
        .join("doc")
        .join(format!(
            "{}.json",
            library_name(find_package(metadata, package, manifest)?)?
        ));
    // Discard only this generated JSON, so a failed or redirected rustdoc invocation cannot reuse a stale API.
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }
    let mut rustdoc = Command::new("cargo");
    rustdoc
        .env("RUSTC_BOOTSTRAP", "1")
        .args(["rustdoc", "--lib", "--manifest-path"])
        .arg(manifest)
        .args(["--target", host])
        .args(features)
        .args(["--", "-Z", "unstable-options", "--output-format", "json"]);
    successful(rustdoc)?;
    fs::copy(path, output)?;
    Ok(serde_json::from_reader(fs::File::open(output)?)?)
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing string field {key}").into())
}

fn string_array(value: &Value) -> Result<Vec<String>> {
    value
        .as_array()
        .ok_or("expected an array of strings")?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "expected a string".into())
        })
        .collect()
}

fn feature_args(profile: &Value, features: &Value, unstable: &[String]) -> Result<Vec<String>> {
    let defaults = profile["default-features"]
        .as_bool()
        .ok_or("profile.default-features must be a boolean")?;
    let selected = string_array(&profile["features"])?;
    let mut pending = selected.clone();
    if defaults {
        pending.push("default".into());
    }
    let mut seen = BTreeSet::new();
    while let Some(feature) = pending.pop() {
        if !seen.insert(feature.clone()) {
            continue;
        }
        if unstable.contains(&feature) {
            return Err(format!("stable API profile enables excluded feature {feature}").into());
        }
        if let Some(values) = features[&feature].as_array() {
            pending.extend(
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|value| features.get(value).is_some())
                    .map(str::to_owned),
            );
        } else if feature != "default" {
            return Err(format!("unknown stable profile feature {feature}").into());
        }
    }
    let mut args = Vec::new();
    if !defaults {
        args.push("--no-default-features".into());
    }
    if !selected.is_empty() {
        args.extend(["--features".into(), selected.join(",")]);
    }
    Ok(args)
}

fn cargo_metadata(manifest: &Path, features: &[String]) -> Result<Value> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(manifest)
        .args(features)
        .output()?;
    if !output.status.success() {
        io::stderr().write_all(&output.stderr)?;
        return Err(format!("cargo metadata failed for {}", manifest.display()).into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn find_package<'a>(metadata: &'a Value, name: &str, manifest: &Path) -> Result<&'a Value> {
    let manifest = manifest.canonicalize()?;
    metadata["packages"]
        .as_array()
        .ok_or("metadata has no packages")?
        .iter()
        .find(|package| {
            package["name"] == name
                && package["manifest_path"]
                    .as_str()
                    .is_some_and(|path| Path::new(path).canonicalize().is_ok_and(|path| path == manifest))
        })
        .ok_or_else(|| format!("package {name} missing from metadata").into())
}

fn library_name(package: &Value) -> Result<&str> {
    package["targets"]
        .as_array()
        .ok_or("package has no targets")?
        .iter()
        .find(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "lib"))
        })
        .and_then(|target| target["name"].as_str())
        .ok_or_else(|| "package has no library target".into())
}

fn planned_dependency_crate<'a>(metadata: &'a Value, name: &str) -> Result<Option<&'a str>> {
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("metadata has no workspace members")?;
    let mut matches = metadata["packages"]
        .as_array()
        .ok_or("metadata has no packages")?
        .iter()
        .filter(|package| package["name"] == name && members.contains(&package["id"]));
    let package = matches
        .next()
        .ok_or_else(|| format!("planned dependency {name} is not a workspace member"))?;
    if matches.next().is_some() {
        return Err(format!("planned dependency {name} is ambiguous").into());
    }
    let targets = package["targets"].as_array().ok_or("package has no targets")?;
    let libraries: Vec<_> = targets
        .iter()
        .filter(|target| {
            target["kind"].as_array().is_some_and(|kinds| {
                kinds.iter().any(|kind| {
                    matches!(
                        kind.as_str(),
                        Some("lib" | "rlib" | "dylib" | "staticlib" | "cdylib" | "proc-macro")
                    )
                })
            })
        })
        .collect();
    match libraries.as_slice() {
        [] => Ok(None),
        [library] => Ok(Some(required_str(library, "name")?)),
        _ => Err(format!("planned dependency {name} has multiple library targets").into()),
    }
}

fn archive_baseline(root: &Path, revision: &str, destination: &Path) -> Result {
    let mut archive = gix_testtools::git_command(root)
        .args(["archive", "--format=tar", "--", revision])
        .stdout(Stdio::piped())
        .spawn()?;
    let extracted = Command::new("tar")
        .args(["-xf", "-", "-C"])
        .arg(destination)
        .stdin(archive.stdout.take().ok_or("git archive has no stdout")?)
        .status()?;
    let archived = archive.wait()?;
    if !extracted.success() || !archived.success() {
        return Err(format!("could not extract baseline {revision}; the release tag must exist locally").into());
    }
    Ok(())
}

fn successful(mut command: Command) -> Result {
    let status = command.status()?;
    if !status.success() {
        return Err(format!("{command:?} failed: {status}").into());
    }
    Ok(())
}

fn compatibility_family(version: &str) -> Result<String> {
    let parts: Vec<_> = version.split(['.', '-', '+']).take(3).collect();
    if parts.len() != 3 || parts.iter().any(|part| part.parse::<u64>().is_err()) {
        return Err(format!("invalid dependency version {version}").into());
    }
    Ok(if parts[0] != "0" {
        parts[0].to_owned()
    } else if parts[1] != "0" {
        parts[..2].join(".")
    } else {
        parts.join(".")
    })
}

fn dependency_identities(metadata: &Value, crate_name: &str) -> Result<BTreeSet<String>> {
    let candidates: Vec<_> = metadata["packages"]
        .as_array()
        .ok_or("metadata has no packages")?
        .iter()
        .filter(|package| library_name(package).is_ok_and(|name| name == crate_name))
        .collect();
    if candidates.len() > 1 {
        return Err(format!(
            "public crate {crate_name} maps to multiple Cargo packages; resolve its precise origin before releasing"
        )
        .into());
    }
    candidates
        .into_iter()
        .map(|package| {
            let family = compatibility_family(required_str(package, "version")?)?;
            let source = match package["source"].as_str() {
                Some(source) => source.to_owned(),
                None => {
                    let manifest = Path::new(required_str(package, "manifest_path")?);
                    let root = Path::new(required_str(metadata, "workspace_root")?);
                    match manifest.strip_prefix(root) {
                        Ok(relative) => format!("workspace:{}", relative.display()),
                        Err(_) => format!("path:{}", manifest.display()),
                    }
                }
            };
            Ok(format!("{source}::{family}"))
        })
        .collect()
}

fn unchanged_identities(
    exposed: &BTreeMap<String, BTreeSet<String>>,
    baseline: &Value,
    current: &Value,
) -> Result<bool> {
    let mut compatible = true;
    for name in exposed
        .keys()
        .filter(|name| !matches!(name.as_str(), "std" | "core" | "alloc"))
    {
        let before = dependency_identities(baseline, name)?;
        let after = dependency_identities(current, name)?;
        if after.is_empty() {
            return Err(format!("cannot identify public dependency crate {name} in Cargo metadata").into());
        }
        if !before.is_empty() && before != after {
            eprintln!("Public dependency identity changed for {name}: {before:?} -> {after:?}");
            compatible = false;
        }
    }
    Ok(compatible)
}

fn unchanged_types(before: &Value, after: &Value) -> Result<bool> {
    let before = type_signatures(before)?;
    let after = type_signatures(after)?;
    let mut compatible = true;
    for (path, signature) in before {
        if after.get(&path) != Some(&signature) {
            eprintln!("Public type signature changed or disappeared: {path}");
            compatible = false;
        }
    }
    Ok(compatible)
}

fn type_signatures(document: &Value) -> Result<BTreeMap<String, Value>> {
    let walk = walk_api(document)?;
    let mut names = BTreeMap::new();
    public_names(document, &document["root"], String::new(), None, &mut names)?;
    for id in &walk.seen {
        let item = &document["index"][id];
        if item["crate_id"] == 0
            && !names.contains_key(id)
            && item["inner"].as_object().is_some_and(|inner| {
                inner
                    .keys()
                    .any(|kind| matches!(kind.as_str(), "struct" | "enum" | "union" | "trait" | "type_alias"))
            })
        {
            // A returned type can expose fields and methods even when its defining module is private.
            let mut path = string_array(&document["paths"][id]["path"])?;
            let name = path.pop().ok_or("reachable type has no canonical path")?;
            public_names(
                document,
                &Value::String(id.clone()),
                path.join("::"),
                Some(&name),
                &mut names,
            )?;
        }
    }
    let mut signatures = BTreeMap::new();
    for id in &walk.seen {
        let item = &document["index"][id];
        if item["crate_id"].as_u64() != Some(0) {
            continue;
        }
        let inner = item["inner"].as_object().ok_or("item has no definition")?;
        let (kind, definition) = inner.iter().next().ok_or("item has no definition")?;
        if kind == "impl" {
            let target = normalized_type(&definition["for"], document, &names)?;
            let trait_type = normalized_type(&definition["trait"], document, &names)?;
            let path = format!("impl {trait_type} for {target}");
            let impl_signature = signature(kind, definition, document, &names)?;
            if !definition["trait"].is_null() {
                signatures.insert(path.clone(), impl_signature.clone());
            }
            for method in definition["items"].as_array().ok_or("impl has no items")? {
                let method_id = item_key(method)?;
                if !walk.seen.contains(&method_id) {
                    continue;
                }
                let method = &document["index"][method_id];
                let definition = method["inner"].as_object().ok_or("impl item has no definition")?;
                let (kind, definition) = definition.iter().next().ok_or("empty impl item")?;
                signatures.insert(
                    format!("{path}::{}", required_str(method, "name")?),
                    serde_json::json!({"impl": impl_signature, "item": signature(kind, definition, document, &names)?}),
                );
            }
        } else if let Some(path) = names.get(id)
            && !matches!(kind.as_str(), "module" | "use" | "macro" | "proc_macro")
        {
            signatures.insert(path.clone(), signature(kind, definition, document, &names)?);
        }
    }
    Ok(signatures)
}

fn signature(kind: &str, definition: &Value, document: &Value, names: &BTreeMap<String, String>) -> Result<Value> {
    let mut value = definition.clone();
    let shape = definition["kind"].as_str().or_else(|| {
        definition["kind"]
            .as_object()
            .and_then(|value| value.keys().next().map(String::as_str))
    });
    if let Some(fields) = value.as_object_mut() {
        for member in [
            "items",
            "impls",
            "implementations",
            "variants",
            "fields",
            "kind",
            "has_body",
            "provided_trait_methods",
            "blanket_impl",
        ] {
            fields.remove(member);
        }
    }
    if kind == "function"
        && let Some(inputs) = value.pointer_mut("/sig/inputs")
    {
        let types = inputs
            .as_array()
            .ok_or("function inputs must be an array")?
            .iter()
            .map(|input| {
                input
                    .as_array()
                    .and_then(|input| input.get(1))
                    .cloned()
                    .ok_or_else(|| "unknown function input shape".into())
            })
            .collect::<Result<Vec<Value>>>()?;
        *inputs = Value::Array(types);
    }
    Ok(serde_json::json!({"kind": kind, "shape": shape, "type": normalized_type(&value, document, names)?}))
}

fn normalized_type(value: &Value, document: &Value, names: &BTreeMap<String, String>) -> Result<Value> {
    match value {
        Value::Object(fields) => {
            let mut normalized = serde_json::Map::new();
            for (key, value) in fields {
                if key == "id" {
                    let id = item_key(value)?;
                    let path = match names.get(&id) {
                        Some(path) if document["index"][&id]["crate_id"] == 0 => path.clone(),
                        _ => string_array(&document["paths"][&id]["path"])
                            .map_err(|err| format!("cannot normalize type {id}: {err}"))?
                            .join("::"),
                    };
                    normalized.insert("resolved_identity".into(), Value::String(path));
                } else if key != "path" || !fields.contains_key("id") {
                    normalized.insert(key.clone(), normalized_type(value, document, names)?);
                }
            }
            Ok(Value::Object(normalized))
        }
        Value::Array(values) => values
            .iter()
            .map(|value| normalized_type(value, document, names))
            .collect::<Result<Vec<_>>>()
            .map(Value::Array),
        _ => Ok(value.clone()),
    }
}

fn item_key(id: &Value) -> Result<String> {
    match id {
        Value::Number(number) => Ok(number.to_string()),
        Value::String(value) => Ok(value.clone()),
        _ => Err("invalid rustdoc item ID".into()),
    }
}

fn public_names(
    document: &Value,
    id: &Value,
    parent: String,
    exported_name: Option<&str>,
    names: &mut BTreeMap<String, String>,
) -> Result {
    let key = item_key(id)?;
    let item = &document["index"][&key];
    if item.is_null() || item["crate_id"] != 0 {
        return Ok(());
    }
    let inner = item["inner"].as_object().ok_or("item has no definition")?;
    let (kind, definition) = inner.iter().next().ok_or("empty definition")?;
    let name = match (exported_name, kind.as_str()) {
        (Some(name), _) => name,
        (None, "use") => required_str(definition, "name")?,
        _ => required_str(item, "name")?,
    };
    let path = if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}::{name}")
    };
    if kind == "use" {
        let target = &definition["id"];
        if !target.is_null() {
            let target_id = item_key(target)?;
            if document["index"][&target_id]["crate_id"] == 0 {
                // Traverse the definition using its exported name, even when its source module is private.
                public_names(document, target, parent, Some(name), names)?;
            }
        }
        return Ok(());
    }
    if let Some(previous) = names.get_mut(&key) {
        if path.len() < previous.len() {
            *previous = path;
        }
        return Ok(());
    }
    names.insert(key, path.clone());
    let children = match kind.as_str() {
        "module" | "trait" => definition.get("items"),
        "enum" => definition.get("variants"),
        "struct" => definition
            .pointer("/kind/plain/fields")
            .or_else(|| definition.pointer("/kind/tuple")),
        "union" => definition.get("fields"),
        "variant" => definition
            .pointer("/kind/struct/fields")
            .or_else(|| definition.pointer("/kind/tuple")),
        _ => None,
    };
    if let Some(children) = children {
        for child in children.as_array().ok_or("named children must be an array")? {
            if child.is_null() {
                continue;
            }
            let child_id = item_key(child)?;
            let inherit = matches!(kind.as_str(), "enum" | "variant" | "trait");
            if inherit || document["index"][&child_id]["visibility"] == "public" {
                public_names(document, child, path.clone(), None, names)?;
            }
        }
    }
    Ok(())
}

fn public_dependencies(document: &Value) -> Result<BTreeMap<String, BTreeSet<String>>> {
    Ok(walk_api(document)?.dependencies)
}

fn walk_api(document: &Value) -> Result<ApiWalk<'_>> {
    if document["format_version"].as_u64() != Some(61) {
        return Err("the API walker requires rustdoc JSON format 61; review schema changes before updating it".into());
    }
    if document["includes_private"].as_bool() != Some(false) {
        return Err("rustdoc JSON must omit private items".into());
    }
    let mut walk = ApiWalk {
        document,
        seen: BTreeSet::new(),
        dependencies: BTreeMap::new(),
    };
    walk.visit(&document["root"], false)?;
    Ok(walk)
}

struct ApiWalk<'a> {
    document: &'a Value,
    seen: BTreeSet<String>,
    dependencies: BTreeMap<String, BTreeSet<String>>,
}

impl ApiWalk<'_> {
    fn visit(&mut self, id: &Value, public_only: bool) -> Result {
        if id.is_null() {
            return Ok(());
        }
        let key = item_key(id)?;
        let item = &self.document["index"][&key];
        if public_only && !item.is_null() && item["visibility"] != "public" {
            return Ok(());
        }
        // Rustdoc gives instantiated blanket impls the local crate ID, so use their trait's origin.
        if !item["inner"]["impl"]["blanket_impl"].is_null() {
            let trait_id = item_key(&item["inner"]["impl"]["trait"]["id"])?;
            let origin = self.document["index"][&trait_id]["crate_id"]
                .as_u64()
                .or_else(|| self.document["paths"][&trait_id]["crate_id"].as_u64())
                .ok_or("blanket trait has no crate identity")?;
            if origin != 0 {
                return Ok(());
            }
        }
        if !self.seen.insert(key.clone()) {
            return Ok(());
        }
        let summary = &self.document["paths"][&key];
        let crate_id = item["crate_id"]
            .as_u64()
            .or_else(|| summary["crate_id"].as_u64())
            .ok_or_else(|| format!("rustdoc item {key} has no crate identity"))?;
        if crate_id != 0 {
            let name = required_str(&self.document["external_crates"][crate_id.to_string()], "name")?;
            let path = summary["path"].as_array().map_or_else(
                || format!("{name}::{}", item["name"].as_str().unwrap_or("impl")),
                |parts| parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("::"),
            );
            self.dependencies.entry(name.to_owned()).or_default().insert(path);
            if item["inner"]["impl"].is_null() {
                return Ok(());
            }
        }
        let inner = item["inner"]
            .as_object()
            .ok_or_else(|| format!("rustdoc item {key} has no definition"))?;
        if inner.len() != 1 {
            return Err(format!("rustdoc item {key} has an unknown definition shape").into());
        }
        let (kind, value) = inner.iter().next().ok_or("empty item definition")?;
        match kind.as_str() {
            "module" => self.children(&value["items"], true),
            "impl" => {
                let mut signature = value.clone();
                signature.as_object_mut().ok_or("invalid impl")?.remove("items");
                self.scan(&signature)?;
                self.children(&value["items"], value["trait"].is_null())
            }
            "struct" | "union" => {
                let mut definition = value.clone();
                // Ordinary rustdoc output strips private fields; verify visibility on any fields that remain.
                if let Some(fields) = definition.pointer_mut("/kind/plain/fields") {
                    self.children(fields, true)?;
                    *fields = Value::Array(Vec::new());
                }
                if let Some(fields) = definition.get_mut("fields") {
                    self.children(fields, true)?;
                    *fields = Value::Array(Vec::new());
                }
                self.scan(&definition)
            }
            "enum" | "variant" | "function" | "type_alias" | "trait" | "trait_alias" | "struct_field"
            | "assoc_type" | "assoc_const" | "constant" | "static" | "extern_type" | "use" | "macro" | "proc_macro"
            | "primitive" | "keyword" => self.scan(value),
            _ => Err(format!("unsupported rustdoc item kind {kind}; update the API walker before releasing").into()),
        }
    }

    fn children(&mut self, ids: &Value, public_only: bool) -> Result {
        for id in ids.as_array().ok_or("rustdoc child IDs must be an array")? {
            self.visit(id, public_only)?;
        }
        Ok(())
    }

    fn scan(&mut self, value: &Value) -> Result {
        match value {
            Value::Object(fields) => {
                for (key, value) in fields {
                    match key.as_str() {
                        "id" => self.visit(value, false)?,
                        "items" | "impls" | "implementations" | "variants" | "fields" | "tuple" if value.is_array() => {
                            for child in value.as_array().ok_or("expected child IDs")? {
                                if child.is_number() || child.is_string() || child.is_null() {
                                    self.visit(child, false)?;
                                } else {
                                    self.scan(child)?;
                                }
                            }
                        }
                        _ => self.scan(value)?,
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    self.scan(value)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn public_graph_follows_associated_types_and_ignores_private_fields() -> gix_testtools::TestResult {
        let document = json!({
            "root": 0, "includes_private": false, "format_version": 61,
            "external_crates": {"1": {"name": "public_dep"}, "2": {"name": "private_dep"}},
            "paths": {"10": {"crate_id": 1, "path": ["public_dep", "Item"]}, "11": {"crate_id": 2, "path": ["private_dep", "Storage"]}},
            "index": {
                "0": {"crate_id": 0, "visibility": "public", "inner": {"module": {"items": [1]}}},
                "1": {"crate_id": 0, "visibility": "public", "inner": {"struct": {"kind": {"plain": {"fields": [2]}}, "impls": [3]}}},
                "2": {"crate_id": 0, "visibility": "default", "inner": {"struct_field": {"resolved_path": {"id": 11}}}},
                "3": {"crate_id": 0, "inner": {"impl": {"trait": {"id": 10}, "items": [4]}}},
                "4": {"crate_id": 0, "visibility": "default", "inner": {"assoc_type": {"type": {"resolved_path": {"id": 10}}}}}
            }
        });
        let exposed = public_dependencies(&document)?;
        assert!(
            exposed.contains_key("public_dep"),
            "associated types are public dependency commitments"
        );
        assert!(
            !exposed.contains_key("private_dep"),
            "private storage does not force dependency propagation"
        );
        let mut blanket = document;
        blanket["index"]["3"]["inner"]["impl"]["blanket_impl"] = json!({"generic": "T"});
        assert!(
            public_dependencies(&blanket)?.is_empty(),
            "external blanket impls do not bind the implementing crate to that trait's version"
        );
        Ok(())
    }

    fn function_api() -> Value {
        json!({
            "root": 0, "includes_private": false, "format_version": 61,
            "external_crates": {"1": {"name": "alloc"}},
            "paths": {"10": {"crate_id": 1, "path": ["alloc", "string", "String"]}, "11": {"crate_id": 1, "path": ["alloc", "vec", "Vec"]}},
            "index": {
                "0": {"name": "api", "crate_id": 0, "visibility": "public", "inner": {"module": {"items": [1]}}},
                "1": {"name": "read", "crate_id": 0, "visibility": "public", "inner": {"function": {"sig": {"inputs": [], "output": {"resolved_path": {"id": 10, "path": "String"}}}, "generics": {"params": [], "where_predicates": []}}}},
                "2": {"name": "experimental", "crate_id": 0, "visibility": "default", "inner": {"function": {"sig": {"inputs": [], "output": {"resolved_path": {"id": 11}}}}}}
            }
        })
    }

    #[test]
    fn type_changes_are_detected_even_within_one_dependency() -> gix_testtools::TestResult {
        let before = function_api();
        let mut after = before.clone();
        after["index"]["2"]["inner"]["function"]["sig"]["output"] = json!({"primitive": "bool"});
        assert!(
            unchanged_types(&before, &after)?,
            "unreachable experimental functions are outside the stable contract"
        );
        after["index"]["1"]["inner"]["function"]["sig"]["output"] = json!({"resolved_path": {"id": 11, "path": "Vec"}});
        assert!(
            !unchanged_types(&before, &after)?,
            "String to Vec is a breaking return type even though both come from alloc"
        );
        Ok(())
    }

    #[test]
    fn rustdoc_ids_and_source_aliases_do_not_define_type_identity() -> gix_testtools::TestResult {
        let before = function_api();
        let mut after = before.clone();
        after["paths"]["99"] = after["paths"]["10"].clone();
        after["index"]["1"]["inner"]["function"]["sig"]["output"] =
            json!({"resolved_path": {"id": 99, "path": "std::string::String"}});
        assert!(
            unchanged_types(&before, &after)?,
            "type identity uses resolved paths instead of session IDs or import spelling"
        );
        Ok(())
    }

    #[test]
    fn public_tuple_fields_and_bounds_are_checked() -> gix_testtools::TestResult {
        let mut before = function_api();
        before["index"]["0"]["inner"]["module"]["items"] = json!([3]);
        before["index"]["3"] = json!({"name": "Wrapper", "crate_id": 0, "visibility": "public", "inner": {"struct": {"kind": {"tuple": [4]}, "generics": {"params": [], "where_predicates": []}, "impls": []}}});
        before["index"]["4"] = json!({"name": "0", "crate_id": 0, "visibility": "public", "inner": {"struct_field": {"resolved_path": {"id": 10}}}});
        assert!(
            public_dependencies(&before)?.contains_key("alloc"),
            "tuple field types are public dependencies"
        );
        let mut after = before.clone();
        after["index"]["4"]["inner"]["struct_field"] = json!({"resolved_path": {"id": 11}});
        assert!(
            !unchanged_types(&before, &after)?,
            "changing a tuple field's type breaks its contract"
        );
        let mut after = before.clone();
        after["index"]["3"]["inner"]["struct"]["generics"]["where_predicates"] =
            json!([{"lifetime_predicate": {"lifetime": "'a", "outlives": ["'static"]}}]);
        assert!(
            !unchanged_types(&before, &after)?,
            "tightening generic bounds breaks the contract"
        );
        Ok(())
    }

    #[test]
    fn fields_of_types_returned_from_private_modules_are_checked() -> gix_testtools::TestResult {
        let mut before = function_api();
        before["paths"]["3"] = json!({"crate_id": 0, "path": ["api", "hidden", "S"]});
        before["index"]["1"]["inner"]["function"]["sig"]["output"] = json!({"resolved_path": {"id": 3}});
        before["index"]["3"] = json!({"name": "S", "crate_id": 0, "visibility": "public", "inner": {"struct": {"kind": {"plain": {"fields": [4]}}, "generics": {"params": [], "where_predicates": []}, "impls": []}}});
        before["index"]["4"] = json!({"name": "x", "crate_id": 0, "visibility": "public", "inner": {"struct_field": {"resolved_path": {"id": 10}}}});
        let mut after = before.clone();
        after["index"]["4"]["inner"]["struct_field"] = json!({"resolved_path": {"id": 11}});
        assert!(
            !unchanged_types(&before, &after)?,
            "callers can access read().x even though hidden::S has no public name"
        );
        Ok(())
    }

    #[test]
    fn profiles_cannot_enable_unstable_indirectly() {
        let features = json!({"default": ["bundle"], "bundle": ["unstable"], "unstable": []});
        let profile = json!({"default-features": true, "features": []});
        assert!(
            feature_args(&profile, &features, &["unstable".into()]).is_err(),
            "stable profiles exclude the entire unstable feature closure"
        );
    }

    #[test]
    fn ambiguous_public_dependency_origins_and_unknown_schemas_fail_closed() {
        let metadata = json!({"packages": [
            {"name": "foo", "version": "1.0.0", "targets": [{"kind": ["lib"], "name": "foo"}]},
            {"name": "foo", "version": "2.0.0", "targets": [{"kind": ["lib"], "name": "foo"}]}
        ]});
        assert!(
            dependency_identities(&metadata, "foo").is_err(),
            "identical rustdoc crate names cannot identify which major version a public type uses"
        );
        let mut document = function_api();
        document["format_version"] = json!(62);
        assert!(
            public_dependencies(&document).is_err(),
            "new schemas need review so new type-reference shapes cannot be silently skipped"
        );
    }

    #[test]
    fn requested_manifest_selects_the_package_when_names_overlap() -> gix_testtools::TestResult {
        let directory = tempfile::tempdir()?;
        let old = directory.path().join("registry.toml");
        let current = directory.path().join("workspace.toml");
        fs::write(&old, "")?;
        fs::write(&current, "")?;
        let metadata = json!({"packages": [
            {"name": "foo", "version": "1.0.0", "manifest_path": old},
            {"name": "foo", "version": "2.0.0", "manifest_path": current}
        ]});
        assert_eq!(
            find_package(&metadata, "foo", &current)?["version"],
            "2.0.0",
            "a registry package cannot supply the workspace package's stable feature profiles"
        );
        Ok(())
    }

    #[test]
    fn planned_dependencies_use_the_actual_library_target_name() -> gix_testtools::TestResult {
        let metadata = json!({"workspace_members": ["local"], "packages": [
            {"id": "registry", "name": "package-name", "targets": [{"kind": ["lib"], "name": "old_name"}]},
            {"id": "local", "name": "package-name", "targets": [{"kind": ["lib"], "name": "public_types"}]}
        ]});
        assert_eq!(
            planned_dependency_crate(&metadata, "package-name")?,
            Some("public_types"),
            "custom lib names cannot hide a planned public dependency break"
        );
        Ok(())
    }

    #[test]
    fn local_dependency_identity_preserves_origin_but_normalizes_archives() -> gix_testtools::TestResult {
        let mut before = json!({"workspace_root": "/baseline", "packages": [
            {"name": "foo", "version": "1.0.0", "source": null, "manifest_path": "/baseline/foo/Cargo.toml", "targets": [{"kind": ["lib"], "name": "foo"}]}
        ]});
        let mut after = before.clone();
        after["workspace_root"] = json!("/current");
        after["packages"][0]["manifest_path"] = json!("/current/foo/Cargo.toml");
        assert_eq!(
            dependency_identities(&before, "foo")?,
            dependency_identities(&after, "foo")?,
            "temporary baseline archive directories do not change nominal identity"
        );
        before["packages"][0]["manifest_path"] = json!("/external/old/Cargo.toml");
        after["packages"][0]["manifest_path"] = json!("/external/new/Cargo.toml");
        assert_ne!(
            dependency_identities(&before, "foo")?,
            dependency_identities(&after, "foo")?,
            "distinct external path packages must not collapse to one workspace identity"
        );
        Ok(())
    }

    #[test]
    fn dependency_identity_uses_cargos_compatibility_ranges() -> gix_testtools::TestResult {
        assert_eq!(
            compatibility_family("1.2.3")?,
            compatibility_family("1.4.0")?,
            "stable minor versions can unify"
        );
        assert_ne!(
            compatibility_family("0.2.3")?,
            compatibility_family("0.3.0")?,
            "pre-1.0 minor bumps change nominal identity"
        );
        assert_ne!(
            compatibility_family("0.0.3")?,
            compatibility_family("0.0.4")?,
            "0.0 patch versions are incompatible"
        );
        Ok(())
    }
}
