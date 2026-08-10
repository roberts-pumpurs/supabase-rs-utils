#![expect(
    clippy::assertions_on_result_states,
    clippy::indexing_slicing,
    clippy::tests_outside_test_module,
    clippy::unwrap_used,
    reason = "Panicking assertions are appropriate in integration tests"
)]

use core::error::Error as _;

use http::StatusCode;
use rp_postgrest_error::{
    Authentication, ErrorCode, ErrorKind, ErrorResponse, PostgresErrorCode, PostgrestError,
};

#[test]
fn decoded_error_preserves_observed_status_and_structured_body() {
    let body = br#"{
        "code": "23505",
        "message": "duplicate key value violates unique constraint",
        "details": "Key (id)=(1) already exists.",
        "hint": null
    }"#;

    let error = PostgrestError::from_slice(StatusCode::IM_A_TEAPOT, body).unwrap();

    assert_eq!(error.status(), StatusCode::IM_A_TEAPOT);
    assert_eq!(error.code().as_str(), "23505");
    assert_eq!(
        error.kind(),
        ErrorKind::Postgres(PostgresErrorCode::UniqueViolation)
    );
    assert_eq!(
        error.inferred_status(Authentication::Unknown),
        Some(StatusCode::CONFLICT)
    );
    assert_eq!(
        error.response().message,
        "duplicate key value violates unique constraint"
    );
    assert_eq!(
        error.response().details.as_deref(),
        Some("Key (id)=(1) already exists.")
    );
    assert_eq!(error.response().hint, None);
}

#[test]
fn display_uses_observed_status_exact_code_and_message() {
    let body = br#"{
        "code": "23505",
        "message": "duplicate key",
        "details": null,
        "hint": null
    }"#;
    let error = PostgrestError::from_slice(StatusCode::CONFLICT, body).unwrap();

    assert_eq!(
        error.to_string(),
        "PostgREST request failed with 409 Conflict [23505]: duplicate key"
    );

    let std_error: &dyn core::error::Error = &error;
    assert_eq!(
        std_error.to_string(),
        "PostgREST request failed with 409 Conflict [23505]: duplicate key"
    );
}

#[test]
fn minimal_error_body_decodes_without_optional_fields() {
    let body = br#"{
        "code": "PGRST125",
        "message": "invalid path",
        "future_field": "ignored"
    }"#;

    let error = PostgrestError::from_slice(StatusCode::NOT_FOUND, body).unwrap();

    assert_eq!(error.code().as_str(), "PGRST125");
    assert_eq!(error.response().details, None);
    assert_eq!(error.response().hint, None);
    assert_eq!(
        serde_json::to_value(error.response()).unwrap()["code"],
        "PGRST125"
    );
}

#[test]
fn malformed_body_error_retains_status_body_and_decode_source() {
    let body = b"<html>bad gateway</html>";

    let error = PostgrestError::from_slice(StatusCode::BAD_GATEWAY, body).unwrap_err();

    assert_eq!(error.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(error.body(), body);
    assert!(error.source().is_some());
    assert!(
        error
            .to_string()
            .starts_with("failed to decode PostgREST error response with status 502 Bad Gateway")
    );
}

#[test]
fn owned_malformed_body_is_retained_by_owned_decoder() {
    let body = b"<html>bad gateway</html>".to_vec();
    let body_pointer = body.as_ptr();

    let error = PostgrestError::from_vec(StatusCode::BAD_GATEWAY, body).unwrap_err();

    assert_eq!(error.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(error.body(), b"<html>bad gateway</html>");
    assert_eq!(error.body().as_ptr(), body_pointer);
}

#[test]
fn missing_required_fields_are_decode_errors_instead_of_empty_strings() {
    let missing_code = br#"{
        "message": "gateway error",
        "details": null,
        "hint": null
    }"#;
    let missing_message = br#"{
        "code": "PGRST999",
        "details": null,
        "hint": null
    }"#;

    assert!(PostgrestError::from_slice(StatusCode::BAD_GATEWAY, missing_code).is_err());
    assert!(PostgrestError::from_slice(StatusCode::BAD_GATEWAY, missing_message).is_err());
}

#[test]
fn message_sensitive_postgres_status_inference_matches_postgrest() {
    let cases = [
        (
            "21000",
            "more than one row returned by a subquery used as an expression",
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            "21000",
            "UPDATE requires a WHERE clause",
            StatusCode::BAD_REQUEST,
        ),
        (
            "22023",
            "role \"missing\" does not exist",
            StatusCode::UNAUTHORIZED,
        ),
        ("22023", "invalid parameter value", StatusCode::BAD_REQUEST),
        (
            "57P01",
            "terminating connection due to administrator command",
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            "42883",
            "function xmlagg(xml) does not exist",
            StatusCode::NOT_ACCEPTABLE,
        ),
        (
            "42883",
            "function api.missing() does not exist",
            StatusCode::NOT_FOUND,
        ),
    ];

    for (code, message, status) in cases {
        let response = ErrorResponse {
            code: ErrorCode::new(code),
            message: message.to_owned(),
            details: None,
            hint: None,
        };
        assert_eq!(
            response.inferred_status(Authentication::Unknown),
            Some(status)
        );
    }
}

#[test]
fn consuming_an_error_returns_status_and_response_together() {
    let response = ErrorResponse {
        code: ErrorCode::new("PGRST205"),
        message: "table not found".to_owned(),
        details: None,
        hint: None,
    };
    let error = PostgrestError::from_response(StatusCode::NOT_FOUND, response.clone());

    assert_eq!(error.into_parts(), (StatusCode::NOT_FOUND, response));
}
