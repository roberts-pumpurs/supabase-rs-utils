#![cfg(feature = "test-util")]
#![expect(
    clippy::tests_outside_test_module,
    clippy::unwrap_used,
    clippy::panic,
    reason = "Integration fixtures assert public error behavior"
)]

use rp_postgrest::{Error, reqwest::StatusCode};

#[test]
fn malformed_fixtures_retain_status_and_exact_bytes() {
    for body in [b"proxy failure\xff".as_slice(), b"{}".as_slice()] {
        let error = Error::from_response(StatusCode::BAD_GATEWAY, body).unwrap();
        assert_eq!(
            error.server_response(),
            Some((StatusCode::BAD_GATEWAY, None))
        );
        assert!(!error.is_jwt_expired());
        assert!(error.postgrest_response().is_none());
        let Error::Decode { source, metadata } = error else {
            panic!("fixture must use the malformed envelope variant");
        };
        assert_eq!(source.body(), body);
        assert_eq!(source.status(), metadata.status());
        assert!(metadata.headers().is_empty());
        assert_eq!(metadata.url().as_str(), "http://localhost/");
    }
}

#[test]
fn valid_fixtures_borrow_body_and_use_observed_status() {
    let error = Error::from_response(
        StatusCode::BAD_REQUEST,
        br#"{"code":"23505","message":"duplicate key","details":null,"hint":null}"#,
    )
    .unwrap();
    let (status, body) = error.server_response().unwrap();
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body.unwrap().sqlstate(), Some("23505"));
    assert_eq!(error.postgrest_response().unwrap().0, status);
}

#[test]
fn jwt_expiration_requires_both_code_and_expiration_message() {
    for (code, message, expired) in [
        ("PGRST301", "JWT expired", true),
        ("PGRST303", "JWT expired", true),
        ("PGRST303", "jwt expired.", true),
        ("PGRST301", "JWT invalid", false),
        ("PGRST303", "JWT signature does not match", false),
        ("PGRST302", "JWT expired", false),
        ("23505", "JWT expired", false),
        ("PGRST301", "JWT is not expired", false),
    ] {
        let body = serde_json::to_vec(&serde_json::json!({
            "code": code, "message": message, "details": null, "hint": null
        }))
        .unwrap();
        let error = Error::from_response(StatusCode::UNAUTHORIZED, &body).unwrap();
        assert_eq!(error.is_jwt_expired(), expired, "{code}: {message}");
    }
}

#[test]
fn successful_status_fixtures_are_not_errors() {
    for status in [
        StatusCode::OK,
        StatusCode::NO_CONTENT,
        StatusCode::PARTIAL_CONTENT,
    ] {
        assert!(Error::from_response(status, b"invalid JSON").is_none());
    }
}
