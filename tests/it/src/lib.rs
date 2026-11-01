//! Shared parsing for the internal CLI tools and lightweight QA runner.

use gix_error::{Result, ResultExt, bail, message};

/// Convert one YAML document to JSON, rejecting duplicate keys and non-string mapping keys.
pub fn yaml_to_json(input: &[u8]) -> Result<serde_json::Value> {
    // Parsing a YAML value first rejects duplicate keys instead of silently overwriting them.
    let document: yaml_serde::Value = yaml_serde::from_slice(input).or_raise(|| message("Parse YAML"))?;
    let mut pending = vec![&document];
    while let Some(value) = pending.pop() {
        match value {
            // JSON's value deserializer would silently turn these numbers into null.
            yaml_serde::Value::Number(number) if number.as_f64().is_some_and(|number| !number.is_finite()) => {
                bail!(message("Non-finite YAML numbers cannot be represented in JSON"));
            }
            yaml_serde::Value::Sequence(values) => pending.extend(values),
            yaml_serde::Value::Mapping(values) => pending.extend(values.values()),
            _ => {}
        }
    }
    yaml_serde::from_value(document).or_raise(|| message("Convert YAML to JSON"))
}
