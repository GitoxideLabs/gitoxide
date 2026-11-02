use std::collections::BTreeSet;

use gix_error::{OptionExt, Result, bail, message};
use serde_json::{Value, json};

fn strings<'a>(value: &'a Value, context: &str) -> Result<Vec<&'a str>> {
    value
        .as_array()
        .ok_or_raise(|| message!("{context} must be an array"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.contains('\0'))
                .ok_or_raise(|| message!("{context} must contain strings without NUL bytes"))
        })
        .collect()
}

/// Expand explicit checks and an opt-in baseline plus one declared feature at a time.
pub fn checks(package: &Value, settings: &Value) -> Result<Vec<Value>> {
    let name = package["name"].as_str().unwrap_or("package");
    if !settings.is_null() && !settings.is_object() {
        bail!("{name}: QA settings must be a table");
    }
    let mut checks = match settings.get("checks") {
        Some(checks) => checks
            .as_array()
            .ok_or_raise(|| message!("{name}: qa.checks must be an array"))?
            .clone(),
        None => Vec::new(),
    };
    for case in &checks {
        let fields = case
            .as_object()
            .ok_or_raise(|| message!("{name}: QA checks must be objects"))?;
        for key in fields.keys() {
            if !["args", "env", "packages", "requires"].contains(&key.as_str()) {
                bail!("{name}: unknown QA check field '{key}'");
            }
        }
        strings(&case["args"], &format!("{name}: QA check args"))?;
    }
    if let Some(policy) = settings.get("each-feature") {
        let fields = policy
            .as_object()
            .ok_or_raise(|| message!("{name}: qa.each-feature must be a table"))?;
        for key in fields.keys() {
            if !["args", "base", "exclude"].contains(&key.as_str()) {
                bail!("{name}: unknown each-feature field '{key}'");
            }
        }
        let empty = json!([]);
        let args = strings(policy.get("args").unwrap_or(&empty), "each-feature args")?;
        if args.iter().any(|arg| {
            matches!(*arg, "--features" | "--all-features") || arg.starts_with("-F") || arg.starts_with("--features=")
        }) {
            bail!("{name}: each-feature args cannot select features; declare them in base instead");
        }
        let base: BTreeSet<_> = strings(policy.get("base").unwrap_or(&empty), "each-feature base")?
            .into_iter()
            .collect();
        let exclude: BTreeSet<_> = strings(policy.get("exclude").unwrap_or(&empty), "each-feature exclude")?
            .into_iter()
            .collect();
        let declared = package["features"]
            .as_object()
            .ok_or_raise(|| message!("{name}: Cargo metadata is missing features"))?;
        for feature in base.union(&exclude) {
            if !declared.contains_key(*feature) {
                bail!("{name}: each-feature names undeclared feature '{feature}'");
            }
        }
        for addition in std::iter::once(None).chain(
            declared
                .keys()
                .map(String::as_str)
                .filter(|feature| !base.contains(feature) && !exclude.contains(feature))
                .map(Some),
        ) {
            let mut features = base.clone();
            features.extend(addition);
            let mut args: Vec<_> = args.iter().map(|arg| (*arg).to_owned()).collect();
            if !features.is_empty() {
                args.extend(["--features".into(), features.into_iter().collect::<Vec<_>>().join(",")]);
            }
            checks.push(json!({"args": args}));
        }
    }
    let mut unique = Vec::new();
    for case in checks {
        if !unique.contains(&case) {
            unique.push(case);
        }
    }
    Ok(unique)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package() -> Value {
        json!({"name": "sample", "features": {
            "default": ["serde"], "sha1": [], "serde": [], "parallel": [], "backend": []
        }})
    }

    #[test]
    fn adds_one_feature_at_a_time_without_cross_products() -> gix_error::TestResult {
        let settings = json!({"each-feature": {
            "args": ["--no-default-features", "--tests"], "base": ["sha1"], "exclude": ["default", "backend"]
        }});
        assert_eq!(
            checks(&package(), &settings)?,
            json!([
                {"args": ["--no-default-features", "--tests", "--features", "sha1"]},
                {"args": ["--no-default-features", "--tests", "--features", "parallel,sha1"]},
                {"args": ["--no-default-features", "--tests", "--features", "serde,sha1"]}
            ])
            .as_array()
            .expect("expected checks are an array")
            .clone(),
            "orthogonal features are added independently to the declared baseline"
        );
        Ok(())
    }

    #[test]
    fn explicit_cases_are_preserved_and_exact_repetitions_are_removed() -> gix_error::TestResult {
        let case = json!({"args": ["--all-features"], "requires": ["xz"], "env": {"HASH": "sha256"}});
        assert_eq!(
            checks(&package(), &json!({"checks": [case.clone(), case.clone()]}))?,
            [case],
            "explicit combinations and execution requirements survive expansion"
        );
        assert!(
            checks(&package(), &json!({}))?.is_empty(),
            "feature enumeration is opt-in"
        );
        Ok(())
    }

    #[test]
    fn malformed_feature_policies_fail_in_the_planner() -> gix_error::TestResult {
        for policy in [
            json!({"exclude": ["typo"]}),
            json!({"base": ["typo"]}),
            json!({"excludes": ["backend"]}),
            json!({"args": ["--all-features"]}),
            json!({"args": ["-Fserde"]}),
            json!({"args": [false]}),
        ] {
            assert!(
                checks(&package(), &json!({"each-feature": policy})).is_err(),
                "invalid policy must fail: {policy}"
            );
        }
        assert!(
            checks(&package(), &json!({"checks": [{"args": [], "require": ["xz"]}]})).is_err(),
            "misspelled case requirements cannot silently weaken provisioning"
        );
        Ok(())
    }
}
