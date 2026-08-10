#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests are already isolated in a test crate"
)]

use http::StatusCode;
use rp_postgrest_error::{
    Authentication, ErrorCode, ErrorKind, PostgresErrorCode, PostgrestErrorCode,
};

#[test]
fn pt_code_is_a_custom_http_status() {
    let code = ErrorCode::new("PT402");

    assert_eq!(code.as_str(), "PT402");
    assert_eq!(
        code.kind(),
        ErrorKind::CustomStatus(StatusCode::PAYMENT_REQUIRED)
    );
    assert_eq!(
        code.inferred_status(Authentication::Unknown),
        Some(StatusCode::PAYMENT_REQUIRED)
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The table mirrors PostgREST's complete emitted-code catalogue"
)]
fn current_postgrest_codes_have_typed_meanings_and_server_statuses() {
    let cases = [
        (
            "PGRST000",
            PostgrestErrorCode::CouldNotConnectDatabase,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            "PGRST001",
            PostgrestErrorCode::InternalConnectionError,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            "PGRST002",
            PostgrestErrorCode::CouldNotConnectSchemaCache,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            "PGRST003",
            PostgrestErrorCode::RequestTimedOut,
            StatusCode::GATEWAY_TIMEOUT,
        ),
        (
            "PGRST100",
            PostgrestErrorCode::ParsingErrorQueryParameter,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST101",
            PostgrestErrorCode::FunctionOnlySupportsGetOrPost,
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        (
            "PGRST102",
            PostgrestErrorCode::InvalidRequestBody,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST103",
            PostgrestErrorCode::InvalidRange,
            StatusCode::RANGE_NOT_SATISFIABLE,
        ),
        (
            "PGRST105",
            PostgrestErrorCode::InvalidPutRequest,
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        (
            "PGRST106",
            PostgrestErrorCode::SchemaNotInConfig,
            StatusCode::NOT_ACCEPTABLE,
        ),
        (
            "PGRST107",
            PostgrestErrorCode::InvalidContentType,
            StatusCode::NOT_ACCEPTABLE,
        ),
        (
            "PGRST108",
            PostgrestErrorCode::FilterOnMissingEmbeddedResource,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST111",
            PostgrestErrorCode::InvalidResponseHeaders,
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            "PGRST112",
            PostgrestErrorCode::InvalidStatusCode,
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            "PGRST114",
            PostgrestErrorCode::UpsertPutWithLimitsOffsets,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST115",
            PostgrestErrorCode::UpsertPutPrimaryKeyMismatch,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST116",
            PostgrestErrorCode::InvalidSingularResponse,
            StatusCode::NOT_ACCEPTABLE,
        ),
        (
            "PGRST117",
            PostgrestErrorCode::UnsupportedHttpVerb,
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        (
            "PGRST118",
            PostgrestErrorCode::CannotOrderByRelatedTable,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST120",
            PostgrestErrorCode::InvalidEmbeddedResourceFilter,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST121",
            PostgrestErrorCode::InvalidRaiseErrorJson,
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            "PGRST122",
            PostgrestErrorCode::InvalidPreferHeader,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST123",
            PostgrestErrorCode::AggregatesDisabled,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST124",
            PostgrestErrorCode::MaxAffectedRowsExceeded,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST125",
            PostgrestErrorCode::InvalidPath,
            StatusCode::NOT_FOUND,
        ),
        (
            "PGRST126",
            PostgrestErrorCode::OpenApiDisabled,
            StatusCode::NOT_FOUND,
        ),
        (
            "PGRST127",
            PostgrestErrorCode::FeatureNotImplemented,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST128",
            PostgrestErrorCode::MaxAffectedRpcExceeded,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST200",
            PostgrestErrorCode::RelationshipNotFound,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST201",
            PostgrestErrorCode::AmbiguousEmbedding,
            StatusCode::MULTIPLE_CHOICES,
        ),
        (
            "PGRST202",
            PostgrestErrorCode::FunctionNotFound,
            StatusCode::NOT_FOUND,
        ),
        (
            "PGRST203",
            PostgrestErrorCode::OverloadedFunctionAmbiguous,
            StatusCode::MULTIPLE_CHOICES,
        ),
        (
            "PGRST204",
            PostgrestErrorCode::ColumnNotFound,
            StatusCode::BAD_REQUEST,
        ),
        (
            "PGRST205",
            PostgrestErrorCode::TableNotFound,
            StatusCode::NOT_FOUND,
        ),
        (
            "PGRST300",
            PostgrestErrorCode::JwtSecretMissing,
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            "PGRST301",
            PostgrestErrorCode::JwtInvalid,
            StatusCode::UNAUTHORIZED,
        ),
        (
            "PGRST302",
            PostgrestErrorCode::AnonymousRoleDisabled,
            StatusCode::UNAUTHORIZED,
        ),
        (
            "PGRST303",
            PostgrestErrorCode::JwtClaimsInvalid,
            StatusCode::UNAUTHORIZED,
        ),
        (
            "PGRSTX00",
            PostgrestErrorCode::InternalLibraryError,
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
    ];

    for (raw, known, status) in cases {
        let code = ErrorCode::new(raw);
        assert_eq!(code.as_str(), raw);
        assert_eq!(code.kind(), ErrorKind::Postgrest(known));
        assert_eq!(code.inferred_status(Authentication::Unknown), Some(status));
    }
}

#[test]
fn future_postgrest_code_is_lossless_without_an_invented_status() {
    let code = ErrorCode::new("PGRST999");

    assert_eq!(code.as_str(), "PGRST999");
    assert_eq!(
        code.kind(),
        ErrorKind::Postgrest(PostgrestErrorCode::Unknown)
    );
    assert_eq!(code.inferred_status(Authentication::Unknown), None);
}

#[test]
fn postgres_codes_keep_the_exact_sqlstate_and_classify_by_meaning() {
    let connection = ErrorCode::new("08006");
    assert_eq!(connection.as_str(), "08006");
    assert_eq!(
        connection.kind(),
        ErrorKind::Postgres(PostgresErrorCode::ConnectionException)
    );
    assert_eq!(
        connection.inferred_status(Authentication::Unknown),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );

    let unique = ErrorCode::new("23505");
    assert_eq!(
        unique.kind(),
        ErrorKind::Postgres(PostgresErrorCode::UniqueViolation)
    );
    assert_eq!(
        unique.inferred_status(Authentication::Unknown),
        Some(StatusCode::CONFLICT)
    );
}

#[test]
fn insufficient_privilege_requires_authentication_context_for_inference() {
    let code = ErrorCode::new("42501");

    assert_eq!(
        code.inferred_status(Authentication::Authenticated),
        Some(StatusCode::FORBIDDEN)
    );
    assert_eq!(
        code.inferred_status(Authentication::Anonymous),
        Some(StatusCode::UNAUTHORIZED)
    );
    assert_eq!(code.inferred_status(Authentication::Unknown), None);
}

#[test]
fn nonstandard_code_is_custom_and_has_no_inferred_status() {
    let code = ErrorCode::new("CUSTOM123");

    assert_eq!(code.kind(), ErrorKind::Custom);
    assert_eq!(code.inferred_status(Authentication::Unknown), None);
}

#[test]
fn unknown_sqlstate_uses_postgrest_default_status_without_losing_its_code() {
    let code = ErrorCode::new("99999");

    assert_eq!(code.as_str(), "99999");
    assert_eq!(code.kind(), ErrorKind::Postgres(PostgresErrorCode::Unknown));
    assert_eq!(
        code.inferred_status(Authentication::Unknown),
        Some(StatusCode::BAD_REQUEST)
    );
}

#[test]
fn postgrest_prefix_requires_the_documented_code_shape() {
    let sqlstate = ErrorCode::new("PGRST");
    assert_eq!(
        sqlstate.kind(),
        ErrorKind::Postgres(PostgresErrorCode::Unknown)
    );

    for malformed in ["PGRSTfoo", "PGRSTABC", "PGRST1A2"] {
        assert_eq!(ErrorCode::new(malformed).kind(), ErrorKind::Custom);
    }

    let internal = ErrorCode::new("PGRSTX00");
    assert_eq!(
        internal.kind(),
        ErrorKind::Postgrest(PostgrestErrorCode::InternalLibraryError)
    );
}

#[test]
fn retired_postgrest_codes_are_lossless_unknowns_without_invented_statuses() {
    for raw in ["PGRST109", "PGRST110", "PGRST119"] {
        let code = ErrorCode::new(raw);
        assert_eq!(code.as_str(), raw);
        assert_eq!(
            code.kind(),
            ErrorKind::Postgrest(PostgrestErrorCode::Unknown)
        );
        assert_eq!(code.inferred_status(Authentication::Unknown), None);
    }
}
