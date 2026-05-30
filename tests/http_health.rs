use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::collections::BTreeMap;
use tower::ServiceExt;

#[tokio::test]
async fn healthz_returns_go_compatible_status_payload() {
    let response = blogweb::app::router()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload, serde_json::json!({ "status": "ok" }));
}

#[tokio::test]
async fn healthz_matches_go_golden_headers_and_cookie_contract() {
    let response = blogweb::app::router()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let golden: Value =
        serde_json::from_str(include_str!("../tests/golden/http/healthz.json")).unwrap();
    let expected_headers = golden["headers"].as_object().unwrap();

    for (name, expected) in expected_headers {
        let actual = response
            .headers()
            .get(name)
            .unwrap_or_else(|| panic!("missing header {name}"))
            .to_str()
            .unwrap();
        assert_eq!(actual, expected[0].as_str().unwrap(), "header {name}");
    }

    let mut cookies = BTreeMap::new();
    for value in response.headers().get_all("set-cookie") {
        let text = value.to_str().unwrap();
        let name = text.split('=').next().unwrap();
        cookies.insert(name.to_string(), text.to_string());
    }
    let anonymous = cookies
        .get("anonymous_id")
        .expect("anonymous_id cookie should be set");
    assert!(anonymous.contains("Path=/"));
    assert!(anonymous.contains("Max-Age=31536000"));
    assert!(anonymous.contains("HttpOnly"));
}
