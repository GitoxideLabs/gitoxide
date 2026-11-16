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
    let res = gix_reqwest::get("https://hyper.rs")
        .await
        .expect("http get example");
    log(&format!("Status: {}", res.status()));

    let body = res.text().await.expect("response to utf-8 text");
    log(&format!("Body:\n\n{body}"));
}

#[wasm_bindgen_test]
async fn request_with_timeout() {
    let client = gix_reqwest::Client::new();
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
fn bearer_auth_redacts_token_in_debug() {
    let token = "sensitive-bearer-token";
    let builder = gix_reqwest::Client::new()
        .get("https://example.test/")
        .bearer_auth(token);
    let cloned_builder = builder.try_clone().unwrap();

    assert!(!format!("{builder:?}").contains(token));
    assert!(!format!("{cloned_builder:?}").contains(token));

    let request = builder.build().unwrap();
    let cloned_request = request.try_clone().unwrap();
    for request in [request, cloned_request, cloned_builder.build().unwrap()] {
        assert_eq!(
            request.headers()[gix_reqwest::header::AUTHORIZATION],
            format!("Bearer {token}")
        );
        assert!(request.headers()[gix_reqwest::header::AUTHORIZATION].is_sensitive());
        assert!(!format!("{request:?}").contains(token));
    }
}

#[wasm_bindgen_test]
fn bearer_auth_invalid_tokens_return_builder_error() {
    for token in ["token\rvalue", "token\nvalue", "token\0value"] {
        let error = gix_reqwest::Client::new()
            .get("https://example.test/")
            .bearer_auth(token)
            .build()
            .unwrap_err();

        assert!(error.is_builder());
    }
}

#[wasm_bindgen_test]
fn header_preserves_sensitive_values() {
    let secret = "existing-secret";
    let mut value = gix_reqwest::header::HeaderValue::from_static(secret);
    value.set_sensitive(true);
    let builder = gix_reqwest::Client::new()
        .get("https://example.test/")
        .header("x-sensitive", value)
        .header("x-public", "visible");

    assert!(!format!("{builder:?}").contains(secret));
    assert!(format!("{builder:?}").contains("visible"));

    let request = builder.build().unwrap();
    assert_eq!(request.headers()["x-sensitive"], secret);
    assert!(request.headers()["x-sensitive"].is_sensitive());
    assert!(!request.headers()["x-public"].is_sensitive());
    assert!(!format!("{request:?}").contains(secret));
}

#[wasm_bindgen_test]
#[cfg(feature = "json")]
fn preserve_content_type_if_set_manually() {
    use gix_reqwest::Client;
    use http::{header::CONTENT_TYPE, HeaderValue};
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
    use gix_reqwest::Client;
    use http::header::CONTENT_TYPE;
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
