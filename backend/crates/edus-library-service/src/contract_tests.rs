use serde_json::Value;
fn validate(doc: &Value, schema: &Value, value: &Value) {
    if let Some(reference) = schema["$ref"].as_str() {
        validate(
            doc,
            &doc["components"]["schemas"][reference.rsplit('/').next().unwrap()],
            value,
        );
        return;
    }
    if let Some(alternatives) = schema["anyOf"].as_array() {
        if alternatives.iter().any(|s| s["format"] == "date")
            && alternatives.iter().any(|s| s["format"] == "date-time")
            && !value.is_null()
        {
            let text = value.as_str().expect("ISO date string");
            assert!(
                chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok()
                    || chrono::DateTime::parse_from_rfc3339(text).is_ok()
            );
            return;
        }
        if value.is_null() {
            assert!(alternatives.iter().any(|s| s["type"] == "null"));
            return;
        }
        let schema = alternatives.iter().find(|s| s["type"] != "null").unwrap();
        validate(doc, schema, value);
        return;
    }
    if let Some(choices) = schema["enum"].as_array() {
        assert!(choices.contains(value), "enum mismatch: {value}");
    }
    match schema["type"].as_str().unwrap() {
        "object" => {
            let object = value.as_object().expect("object");
            for name in schema["required"].as_array().unwrap_or(&vec![]) {
                assert!(
                    object.contains_key(name.as_str().unwrap()),
                    "missing {name}"
                );
            }
            for (key, v) in object {
                if let Some(s) = schema["properties"].get(key) {
                    validate(doc, s, v)
                } else if schema["additionalProperties"].is_object() {
                    validate(doc, &schema["additionalProperties"], v)
                } else {
                    assert_ne!(
                        schema["additionalProperties"], false,
                        "unexpected property {key}"
                    );
                }
            }
        }
        "array" => {
            for v in value.as_array().expect("array") {
                validate(doc, &schema["items"], v)
            }
        }
        "string" => {
            let text = value.as_str().expect("string");
            if schema["format"] == "date" {
                assert!(
                    chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok(),
                    "date {text}"
                );
            }
            if schema["format"] == "date-time" {
                assert!(
                    chrono::DateTime::parse_from_rfc3339(text).is_ok(),
                    "date-time {text}"
                );
            }
        }
        "integer" => assert!(value.is_i64() || value.is_u64()),
        "boolean" => assert!(value.is_boolean()),
        "null" => assert!(value.is_null()),
        other => panic!("unsupported contract type {other}"),
    }
}
pub(super) fn assert_schema(name: &str, value: &Value) {
    let doc: Value = serde_json::from_str(
        include_str!("../../../openapi/local-service-v1.openapi.json")
            .trim_start_matches('\u{feff}'),
    )
    .unwrap();
    validate(&doc, &doc["components"]["schemas"][name], value);
}
#[tokio::test]
async fn actual_router_contract_and_backend_without_frontend() {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    let (_dir, state) = tests::fixture();
    let runtime = Runtime::open_service(
        &state.data_root,
        WorkspaceConfig {
            mode: WorkspaceMode::Uat,
            cloud_url: None,
            school_id: None,
            terminal_id: None,
            device_name: "Contract UAT".into(),
        },
    )
    .unwrap();
    *state.runtime.lock().unwrap() = Some(Arc::new(runtime));
    let (cookie, csrf) = tests::auth(&state).await;
    for (path, schema) in [
        ("/api/local/v1/version", "Version"),
        ("/health", "Health"),
        ("/api/local/v1/snapshot", "Snapshot"),
        ("/api/local/v1/sync/status", "SyncStatus"),
        ("/api/local/v1/settings/ui", "UiSettings"),
    ] {
        let response = router(state.clone())
            .oneshot(
                Request::get(path)
                    .header("host", "127.0.0.1:43180")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_schema(schema, &value);
        if path == "/health" {
            assert_eq!(value["state"], "FRONTEND_MISSING");
        }
    }
    let (status, value) = tests::call(
        &state,
        "/api/local/v1/catalog/search",
        serde_json::json!({"query":"Абай","sort":"relevance"}),
        &cookie,
        &csrf,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = value.as_array().unwrap();
    assert!(!rows.is_empty());
    for row in rows {
        assert_schema("CatalogSearchResult", row);
    }
    let title = rows[0]["title"]["id"].as_str().unwrap();
    let (status, value) = tests::call(
        &state,
        "/api/local/v1/catalog/availability",
        serde_json::json!({"titleId":title}),
        &cookie,
        &csrf,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_schema("CatalogAvailability", &value);
    let (status, value) = tests::call(
        &state,
        "/api/local/v1/identify/face",
        serde_json::json!({}),
        &cookie,
        &csrf,
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_schema("Error", &value);
}
