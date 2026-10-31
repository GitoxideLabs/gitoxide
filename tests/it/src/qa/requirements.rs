use std::collections::BTreeSet;

use gix_error::{OptionExt, Result, bail, message};
use serde_json::{Value, json};

use super::run::Command;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Requirements {
    pub tools: BTreeSet<String>,
    pub rust_components: BTreeSet<(String, String)>,
    pub rust_toolchains: BTreeSet<String>,
    pub rust_targets: BTreeSet<(String, String)>,
    pub history: bool,
}

impl Requirements {
    fn add(&mut self, requirement: &str, toolchain: &str) {
        if let Some(version) = requirement.strip_prefix("toolchain:") {
            self.rust_toolchains.insert(version.into());
        } else if let Some(target) = requirement.strip_prefix("target:") {
            self.rust_targets.insert((toolchain.into(), target.into()));
        } else if matches!(requirement, "clippy" | "rustfmt") {
            self.rust_components.insert((toolchain.into(), requirement.into()));
        } else if requirement == "history" {
            self.history = true;
        } else {
            self.tools.insert(requirement.into());
        }
    }

    pub fn commands(commands: &[Command]) -> Self {
        let mut requirements = Self::default();
        for command in commands {
            let mut args = command.args.iter();
            let mut subcommand = args.next().map(String::as_str);
            let mut toolchain = command.env.get("RUSTUP_TOOLCHAIN").map_or("stable", String::as_str);
            if command.program == "cargo"
                && let Some(version) = subcommand.and_then(|arg| arg.strip_prefix('+'))
            {
                toolchain = version;
                subcommand = args.next().map(String::as_str);
            }
            if toolchain != "stable" {
                requirements.rust_toolchains.insert(toolchain.into());
            }
            for extra in &command.requires {
                requirements.add(extra, toolchain);
            }
            match command.program.as_str() {
                "just" => requirements.add("just", toolchain),
                "cargo" => {
                    match subcommand {
                        Some("fmt") => requirements.add("rustfmt", toolchain),
                        Some("clippy") => requirements.add("clippy", toolchain),
                        Some(name @ ("nextest" | "deny" | "machete" | "auditable" | "semver-checks" | "diet")) => {
                            requirements.add(&format!("cargo-{name}"), toolchain);
                        }
                        _ => {}
                    }
                    for (index, arg) in command
                        .args
                        .iter()
                        .enumerate()
                        .take_while(|(_, arg)| arg.as_str() != "--")
                    {
                        if arg == "--target" {
                            if let Some(target) = command.args.get(index + 1) {
                                requirements.rust_targets.insert((toolchain.into(), target.clone()));
                            }
                        } else if let Some(target) = arg.strip_prefix("--target=") {
                            requirements.rust_targets.insert((toolchain.into(), target.into()));
                        }
                    }
                }
                _ => {}
            }
        }
        requirements
    }

    pub fn merge(&mut self, other: Self) {
        self.tools.extend(other.tools);
        self.rust_components.extend(other.rust_components);
        self.rust_toolchains.extend(other.rust_toolchains);
        self.rust_targets.extend(other.rust_targets);
        self.history |= other.history;
    }

    pub fn json(&self, metadata: &Value) -> Result<Value> {
        let special = ["just", "yq", "cargo-diet", "xz", "docker"];
        let mut tools = Vec::new();
        for tool in &self.tools {
            if !special.contains(&tool.as_str()) {
                tools.push(format!("{tool}@{}", version(metadata, tool)?));
            }
        }
        let special_version = |name| -> Result<&str> {
            if self.tools.contains(name) {
                version(metadata, name)
            } else {
                Ok("")
            }
        };
        Ok(json!({
            "tools": tools,
            "rust_components": self.rust_components.iter().map(|(toolchain, component)| json!({"toolchain":toolchain,"component":component})).collect::<Vec<_>>(),
            "rust_toolchains": self.rust_toolchains,
            "rust_targets": self.rust_targets.iter().map(|(toolchain, target)| json!({"toolchain":toolchain,"target":target})).collect::<Vec<_>>(),
            "history": self.history,
            "just": self.tools.contains("just"), "just_version": special_version("just")?,
            "yq": self.tools.contains("yq"), "yq_version": special_version("yq")?,
            "diet": self.tools.contains("cargo-diet"), "diet_version": special_version("cargo-diet")?,
            "xz": self.tools.contains("xz"), "docker": self.tools.contains("docker"),
        }))
    }
}

fn version<'a>(metadata: &'a Value, tool: &str) -> Result<&'a str> {
    let version = metadata["metadata"]["qa"]["tool-versions"][tool]
        .as_str()
        .ok_or_raise(|| message!("Declare {tool} in workspace.metadata.qa.tool-versions"))?;
    if version.is_empty()
        || !version
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'+'))
    {
        bail!("Invalid pinned version for {tool}: {version:?}");
    }
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirements_follow_commands_instead_of_task_names() -> gix_error::TestResult {
        let mut command = Command::new("cargo", &["nextest", "run", "--target", "wasm32-wasip1"]).requiring(&["xz"]);
        command.env.insert("RUSTUP_TOOLCHAIN".into(), "1.99.0".into());
        let mut clippy = Command::new("cargo", &["+nightly", "clippy", "--target=wasm32-wasip2"]);
        clippy.env.insert("RUSTUP_TOOLCHAIN".into(), "1.98.0".into());
        let requirements = Requirements::commands(&[command.clone(), command, Command::new("cargo", &["fmt"]), clippy]);
        let metadata = json!({"metadata":{"qa":{"tool-versions":{"cargo-nextest":"0.9.122"}}}});
        let actual = requirements.json(&metadata)?;
        assert_eq!(
            actual["tools"],
            json!(["cargo-nextest@0.9.122"]),
            "shared tools are installed once"
        );
        assert_eq!(
            actual["rust_targets"],
            json!([
                {"toolchain":"1.99.0", "target":"wasm32-wasip1"},
                {"toolchain":"nightly", "target":"wasm32-wasip2"}
            ]),
            "targets belong to the toolchain that builds them"
        );
        assert_eq!(
            actual["rust_toolchains"],
            json!(["1.99.0", "nightly"]),
            "cargo +toolchain overrides RUSTUP_TOOLCHAIN"
        );
        assert_eq!(
            actual["rust_components"],
            json!([
                {"toolchain":"nightly", "component":"clippy"},
                {"toolchain":"stable", "component":"rustfmt"}
            ])
        );
        assert_eq!(actual["xz"], true);
        assert!(
            requirements.json(&json!({})).is_err(),
            "missing pins cannot silently select another tool version"
        );
        Ok(())
    }
}
