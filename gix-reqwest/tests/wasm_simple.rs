#![cfg(target_arch = "wasm32")]
use std::time::Duration;

use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen_test]
async fn simple_example() {
    let res = reqwest::get("https://hyper.rs")
        .await
        .expect("http get example");
    log(&format!("Status: {}", res.status()));

    let body = res.text().await.expect("response to utf-8 text");
    log(&format!("Body:\n\n{body}"));
}

#[wasm_bindgen_test]
async fn request_with_timeout() {
    let client = reqwest::Client::new();
    let err = client
        .get("https://hyper.rs/not-cached")
        .timeout(Duration::from_millis(1))
        .send()
        .await
        .expect_err("Expected error from aborted request");

    assert!(err.is_request());
    assert!(err.is_timeout());
}

#[wasm_bindgen_test]
#[cfg(feature = "json")]
fn preserve_content_type_if_set_manually() {
    use http::{header::CONTENT_TYPE, HeaderValue};
    use reqwest::Client;
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert("body", "json");
    let content_type = HeaderValue::from_static("application/vnd.api+json");
    let req = Client::new()
        .post("https://google.com/")
        .header(CONTENT_TYPE, &content_type)
        .json(&map)
        .build()
        .expect("request is not valid");

    assert_eq!(content_type, req.headers().get(CONTENT_TYPE).unwrap());
}

#[wasm_bindgen_test]
#[cfg(feature = "json")]
fn add_default_json_content_type_if_not_set_manually() {
    use http::header::CONTENT_TYPE;
    use reqwest::Client;
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert("body", "json");
    let req = Client::new()
        .post("https://google.com/")
        .json(&map)
        .build()
        .expect("request is not valid");

    assert_eq!("application/json", req.headers().get(CONTENT_TYPE).unwrap());
}
