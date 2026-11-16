use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// NOTE: This test is a clone of https://github.com/rustwasm/wasm-bindgen/blob/master/examples/fetch/src/lib.rs
// but uses Reqwest instead of the web_sys fetch api directly

/// A struct to hold some data from the GitHub Branch API.
///
/// Note how we don't have to define every member -- serde will ignore extra
/// data when deserializing
#[derive(Debug, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub commit: Commit,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub commit: CommitDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitDetails {
    pub author: Signature,
    pub committer: Signature,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Signature {
    pub name: String,
    pub email: String,
}

#[wasm_bindgen]
pub async fn run() -> Result<JsValue, JsValue> {
    let res = reqwest::Client::new()
        .get("https://api.github.com/repos/rustwasm/wasm-bindgen/branches/master")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;

    let text = res.text().await?;
    let branch_info: Branch = serde_json::from_str(&text).unwrap();

    Ok(JsValue::from_serde(&branch_info).unwrap())
}
