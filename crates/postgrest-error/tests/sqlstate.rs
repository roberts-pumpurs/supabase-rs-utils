#![expect(
    clippy::tests_outside_test_module,
    reason = "This integration test exercises the public error body interface"
)]

use rp_postgrest_error::{ErrorCode, ErrorResponse};

#[test]
fn sqlstate_accepts_only_canonical_database_codes() {
    for (code, expected) in [
        ("23505", Some("23505")),
        ("42P01", Some("42P01")),
        ("ZZ999", Some("ZZ999")),
        ("42p01", None),
        ("PGRST301", None),
        ("PGRST", None),
        ("PT402", None),
        ("2350", None),
        ("235050", None),
        ("23-05", None),
        ("\u{e9}000", None),
    ] {
        let response = ErrorResponse {
            code: ErrorCode::new(code),
            message: String::new(),
            details: None,
            hint: None,
        };
        assert_eq!(response.sqlstate(), expected, "{code}");
    }
}
