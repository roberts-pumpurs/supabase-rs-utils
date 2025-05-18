extern crate alloc;

use alloc::fmt;

use serde::{Deserialize, Serialize};

/// Represents the error response returned by `PostgREST`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Deserialize, Serialize)]
pub struct ErrorResponse {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub code: String,
    pub details: Option<String>,
    pub hint: Option<String>,
}

/// Enum representing the different types of errors that can occur.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum PostgrestUtilError {
    Postgres(PostgresError),
    Postgrest(PostgrestError),
    Custom(CustomError),
}

impl PostgrestUtilError {
    /// Creates an error from an `ErrorResponse`.
    #[must_use]
    pub fn from_error_response(resp: ErrorResponse) -> Self {
        if resp.code.starts_with("PGRST") {
            Self::Postgrest(PostgrestError::from_response(resp))
        } else if resp.code.len() == 5 || resp.code.starts_with("XX") {
            Self::Postgres(PostgresError::from_response(resp))
        } else {
            Self::Custom(CustomError::from_response(resp))
        }
    }

    /// Returns the corresponding HTTP status code for the error.
    #[must_use]
    pub const fn http_status_code(&self, is_authenticated: bool) -> u16 {
        match self {
            Self::Postgres(err) => err.http_status_code(is_authenticated),
            Self::Postgrest(err) => err.http_status_code(),
            Self::Custom(_) => 400, // Default to 400 for custom errors
        }
    }
}

impl core::fmt::Display for PostgrestUtilError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Postgres(err) => {
                write!(fmt, "Postgres [{}]: {}", err.code, err.message)
            }
            Self::Postgrest(err) => {
                write!(fmt, "Postgrest [{}]: {}", err.code, err.message)
            }
            Self::Custom(err) => write!(fmt, "Custom [{}]: {}", err.code, err.message),
        }
    }
}

impl core::error::Error for PostgrestUtilError {}

/// Represents an error returned by `PostgreSQL`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct PostgresError {
    pub code: PostgresErrorCode,
    pub message: String,
    pub details: Option<String>,
    pub hint: Option<String>,
}

impl PostgresError {
    #[must_use]
    pub fn from_response(resp: ErrorResponse) -> Self {
        let code = PostgresErrorCode::from_code(&resp.code);
        Self {
            code,
            message: resp.message,
            details: resp.details,
            hint: resp.hint,
        }
    }

    #[must_use]
    pub const fn http_status_code(&self, is_authenticated: bool) -> u16 {
        self.code.http_status_code(is_authenticated)
    }
}

/// Enum representing `PostgreSQL` error codes.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum PostgresErrorCode {
    // Specific codes
    NotNullViolation,       // 23502
    ForeignKeyViolation,    // 23503
    UniqueViolation,        // 23505
    ReadOnlySqlTransaction, // 25006
    UndefinedFunction,      // 42883
    UndefinedTable,         // 42P01
    InfiniteRecursion,      // 42P17
    InsufficientPrivilege,  // 42501
    ConfigLimitExceeded,    // 53400
    RaiseException,         // P0001
    // Patterns
    ConnectionException,                // 08*
    TriggeredActionException,           // 09*
    InvalidGrantor,                     // 0L*
    InvalidRoleSpecification,           // 0P*
    InvalidTransactionState,            // 25*
    InvalidAuthorizationSpecification,  // 28*
    InvalidTransactionTermination,      // 2D*
    ExternalRoutineException,           // 38*
    ExternalRoutineInvocationException, // 39*
    SavepointException,                 // 3B*
    TransactionRollback,                // 40*
    InsufficientResources,              // 53*
    ProgramLimitExceeded,               // 54*
    ObjectNotInPrerequisiteState,       // 55*
    OperatorIntervention,               // 57*
    SystemError,                        // 58*
    ConfigFileError,                    // F0*
    FdwError,                           // HV*
    PlpgsqlError,                       // P0*
    InternalError,                      // XX*
    // Other errors
    Other(String), // Any other code
}

impl PostgresErrorCode {
    #[must_use]
    pub fn from_code(code: &str) -> Self {
        match code {
            // Specific codes
            "23502" => Self::NotNullViolation,
            "23503" => Self::ForeignKeyViolation,
            "23505" => Self::UniqueViolation,
            "25006" => Self::ReadOnlySqlTransaction,
            "42883" => Self::UndefinedFunction,
            "42P01" => Self::UndefinedTable,
            "42P17" => Self::InfiniteRecursion,
            "42501" => Self::InsufficientPrivilege,
            "53400" => Self::ConfigLimitExceeded,
            "P0001" => Self::RaiseException,
            _ => {
                // Check for patterns
                if code.starts_with("08") {
                    Self::ConnectionException
                } else if code.starts_with("09") {
                    Self::TriggeredActionException
                } else if code.starts_with("0L") {
                    Self::InvalidGrantor
                } else if code.starts_with("0P") {
                    Self::InvalidRoleSpecification
                } else if code.starts_with("25") {
                    Self::InvalidTransactionState
                } else if code.starts_with("28") {
                    Self::InvalidAuthorizationSpecification
                } else if code.starts_with("2D") {
                    Self::InvalidTransactionTermination
                } else if code.starts_with("38") {
                    Self::ExternalRoutineException
                } else if code.starts_with("39") {
                    Self::ExternalRoutineInvocationException
                } else if code.starts_with("3B") {
                    Self::SavepointException
                } else if code.starts_with("40") {
                    Self::TransactionRollback
                } else if code.starts_with("53") {
                    Self::InsufficientResources
                } else if code.starts_with("54") {
                    Self::ProgramLimitExceeded
                } else if code.starts_with("55") {
                    Self::ObjectNotInPrerequisiteState
                } else if code.starts_with("57") {
                    Self::OperatorIntervention
                } else if code.starts_with("58") {
                    Self::SystemError
                } else if code.starts_with("F0") {
                    Self::ConfigFileError
                } else if code.starts_with("HV") {
                    Self::FdwError
                } else if code.starts_with("P0") {
                    Self::PlpgsqlError
                } else if code.starts_with("XX") {
                    Self::InternalError
                } else {
                    Self::Other(code.to_owned())
                }
            }
        }
    }

    #[must_use]
    pub const fn http_status_code(&self, is_authenticated: bool) -> u16 {
        match self {
            // 500 status codes
            Self::TriggeredActionException
            | Self::InvalidTransactionState
            | Self::InvalidTransactionTermination
            | Self::ExternalRoutineException
            | Self::ExternalRoutineInvocationException
            | Self::SavepointException
            | Self::TransactionRollback
            | Self::ProgramLimitExceeded
            | Self::ObjectNotInPrerequisiteState
            | Self::OperatorIntervention
            | Self::SystemError
            | Self::ConfigFileError
            | Self::FdwError
            | Self::PlpgsqlError
            | Self::InternalError
            | Self::ConfigLimitExceeded
            | Self::InfiniteRecursion => 500,

            // 503 status codes
            Self::ConnectionException | Self::InsufficientResources => 503,

            // 403 status codes
            Self::InvalidGrantor
            | Self::InvalidRoleSpecification
            | Self::InvalidAuthorizationSpecification => 403,

            // 404 status codes
            Self::UndefinedFunction | Self::UndefinedTable => 404,

            // 400 status codes
            Self::NotNullViolation | Self::RaiseException | Self::Other(_) => 400,

            // 409 status codes
            Self::ForeignKeyViolation | Self::UniqueViolation => 409,

            // 405 status code
            Self::ReadOnlySqlTransaction => 405,

            // Conditional status code
            Self::InsufficientPrivilege => {
                if is_authenticated {
                    403
                } else {
                    401
                }
            }
        }
    }
}

impl core::fmt::Display for PostgresErrorCode {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotNullViolation => write!(fmt, "23502"),
            Self::ForeignKeyViolation => write!(fmt, "23503"),
            Self::UniqueViolation => write!(fmt, "23505"),
            Self::ReadOnlySqlTransaction => write!(fmt, "25006"),
            Self::UndefinedFunction => write!(fmt, "42883"),
            Self::UndefinedTable => write!(fmt, "42P01"),
            Self::InfiniteRecursion => write!(fmt, "42P17"),
            Self::InsufficientPrivilege => write!(fmt, "42501"),
            Self::ConfigLimitExceeded => write!(fmt, "53400"),
            Self::RaiseException => write!(fmt, "P0001"),
            Self::ConnectionException => write!(fmt, "08*"),
            Self::TriggeredActionException => write!(fmt, "09*"),
            Self::InvalidGrantor => write!(fmt, "0L*"),
            Self::InvalidRoleSpecification => write!(fmt, "0P*"),
            Self::InvalidTransactionState => write!(fmt, "25*"),
            Self::InvalidAuthorizationSpecification => write!(fmt, "28*"),
            Self::InvalidTransactionTermination => write!(fmt, "2D*"),
            Self::ExternalRoutineException => write!(fmt, "38*"),
            Self::ExternalRoutineInvocationException => write!(fmt, "39*"),
            Self::SavepointException => write!(fmt, "3B*"),
            Self::TransactionRollback => write!(fmt, "40*"),
            Self::InsufficientResources => write!(fmt, "53*"),
            Self::ProgramLimitExceeded => write!(fmt, "54*"),
            Self::ObjectNotInPrerequisiteState => write!(fmt, "55*"),
            Self::OperatorIntervention => write!(fmt, "57*"),
            Self::SystemError => write!(fmt, "58*"),
            Self::ConfigFileError => write!(fmt, "F0*"),
            Self::FdwError => write!(fmt, "HV*"),
            Self::PlpgsqlError => write!(fmt, "P0*"),
            Self::InternalError => write!(fmt, "XX*"),
            Self::Other(code) => write!(fmt, "{code}"),
        }
    }
}

/// Represents an error returned by `PostgREST`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct PostgrestError {
    pub code: PostgrestErrorCode,
    pub message: String,
    pub details: Option<String>,
    pub hint: Option<String>,
}

impl PostgrestError {
    #[must_use]
    pub fn from_response(resp: ErrorResponse) -> Self {
        let code = PostgrestErrorCode::from_code(&resp.code);
        Self {
            code,
            message: resp.message,
            details: resp.details,
            hint: resp.hint,
        }
    }

    #[must_use]
    pub const fn http_status_code(&self) -> u16 {
        self.code.http_status_code()
    }
}

/// Enum representing `PostgREST` error codes.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum PostgrestErrorCode {
    // Group 0 - Connection
    CouldNotConnectDatabase,    // PGRST000
    InternalConnectionError,    // PGRST001
    CouldNotConnectSchemaCache, // PGRST002
    RequestTimedOut,            // PGRST003

    // Group 1 - API Request
    ParsingErrorQueryParameter,         // PGRST100
    FunctionOnlySupportsGetOrPost,      // PGRST101
    InvalidRequestBody,                 // PGRST102
    InvalidRange,                       // PGRST103
    InvalidPutRequest,                  // PGRST105
    SchemaNotInConfig,                  // PGRST106
    InvalidContentType,                 // PGRST107
    FilterOnMissingEmbeddedResource,    // PGRST108
    LimitedUpdateDeleteWithoutOrdering, // PGRST109
    LimitedUpdateDeleteExceededMaxRows, // PGRST110
    InvalidResponseHeaders,             // PGRST111
    InvalidStatusCode,                  // PGRST112
    UpsertPutWithLimitsOffsets,         // PGRST114
    UpsertPutPrimaryKeyMismatch,        // PGRST115
    InvalidSingularResponse,            // PGRST116
    UnsupportedHttpVerb,                // PGRST117
    CannotOrderByRelatedTable,          // PGRST118
    CannotSpreadRelatedTable,           // PGRST119
    InvalidEmbeddedResourceFilter,      // PGRST120
    InvalidRaiseErrorJson,              // PGRST121
    InvalidPreferHeader,                // PGRST122

    // Group 2 - Schema Cache
    RelationshipNotFound,        // PGRST200
    AmbiguousEmbedding,          // PGRST201
    FunctionNotFound,            // PGRST202
    OverloadedFunctionAmbiguous, // PGRST203
    ColumnNotFound,              // PGRST204

    // Group 3 - JWT
    JwtSecretMissing,      // PGRST300
    JwtInvalid,            // PGRST301
    AnonymousRoleDisabled, // PGRST302

    // Group X - Internal
    InternalLibraryError, // PGRSTX00

    // Other errors
    Other(String), // Any other code
}

impl PostgrestErrorCode {
    #[must_use]
    pub fn from_code(code: &str) -> Self {
        match code {
            "PGRST000" => Self::CouldNotConnectDatabase,
            "PGRST001" => Self::InternalConnectionError,
            "PGRST002" => Self::CouldNotConnectSchemaCache,
            "PGRST003" => Self::RequestTimedOut,
            "PGRST100" => Self::ParsingErrorQueryParameter,
            "PGRST101" => Self::FunctionOnlySupportsGetOrPost,
            "PGRST102" => Self::InvalidRequestBody,
            "PGRST103" => Self::InvalidRange,
            "PGRST105" => Self::InvalidPutRequest,
            "PGRST106" => Self::SchemaNotInConfig,
            "PGRST107" => Self::InvalidContentType,
            "PGRST108" => Self::FilterOnMissingEmbeddedResource,
            "PGRST109" => Self::LimitedUpdateDeleteWithoutOrdering,
            "PGRST110" => Self::LimitedUpdateDeleteExceededMaxRows,
            "PGRST111" => Self::InvalidResponseHeaders,
            "PGRST112" => Self::InvalidStatusCode,
            "PGRST114" => Self::UpsertPutWithLimitsOffsets,
            "PGRST115" => Self::UpsertPutPrimaryKeyMismatch,
            "PGRST116" => Self::InvalidSingularResponse,
            "PGRST117" => Self::UnsupportedHttpVerb,
            "PGRST118" => Self::CannotOrderByRelatedTable,
            "PGRST119" => Self::CannotSpreadRelatedTable,
            "PGRST120" => Self::InvalidEmbeddedResourceFilter,
            "PGRST121" => Self::InvalidRaiseErrorJson,
            "PGRST122" => Self::InvalidPreferHeader,
            "PGRST200" => Self::RelationshipNotFound,
            "PGRST201" => Self::AmbiguousEmbedding,
            "PGRST202" => Self::FunctionNotFound,
            "PGRST203" => Self::OverloadedFunctionAmbiguous,
            "PGRST204" => Self::ColumnNotFound,
            "PGRST300" => Self::JwtSecretMissing,
            "PGRST301" => Self::JwtInvalid,
            "PGRST302" => Self::AnonymousRoleDisabled,
            "PGRSTX00" => Self::InternalLibraryError,
            _ => Self::Other(code.to_owned()),
        }
    }

    #[must_use]
    pub const fn http_status_code(&self) -> u16 {
        match self {
            // 500 status codes
            Self::InternalConnectionError
            | Self::CouldNotConnectSchemaCache
            | Self::InvalidResponseHeaders
            | Self::InvalidStatusCode
            | Self::InvalidRaiseErrorJson
            | Self::JwtSecretMissing
            | Self::InternalLibraryError
            | Self::Other(_) => 500,
            // 503
            Self::CouldNotConnectDatabase => 503,
            // 504
            Self::RequestTimedOut => 504,
            // 400
            Self::ParsingErrorQueryParameter
            | Self::InvalidRequestBody
            | Self::FilterOnMissingEmbeddedResource
            | Self::LimitedUpdateDeleteWithoutOrdering
            | Self::LimitedUpdateDeleteExceededMaxRows
            | Self::UpsertPutWithLimitsOffsets
            | Self::UpsertPutPrimaryKeyMismatch
            | Self::CannotOrderByRelatedTable
            | Self::CannotSpreadRelatedTable
            | Self::InvalidEmbeddedResourceFilter
            | Self::InvalidPreferHeader
            | Self::RelationshipNotFound
            | Self::ColumnNotFound => 400,
            // 405
            Self::FunctionOnlySupportsGetOrPost
            | Self::InvalidPutRequest
            | Self::UnsupportedHttpVerb => 405,
            // 416
            Self::InvalidRange => 416,
            // 406
            Self::SchemaNotInConfig | Self::InvalidSingularResponse => 406,
            // 415
            Self::InvalidContentType => 415,
            // 300
            Self::AmbiguousEmbedding | Self::OverloadedFunctionAmbiguous => 300,
            // 404
            Self::FunctionNotFound => 404,
            // 401
            Self::JwtInvalid | Self::AnonymousRoleDisabled => 401,
        }
    }
}

impl core::fmt::Display for PostgrestErrorCode {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CouldNotConnectDatabase => write!(fmt, "PGRST000"),
            Self::InternalConnectionError => write!(fmt, "PGRST001"),
            Self::CouldNotConnectSchemaCache => write!(fmt, "PGRST002"),
            Self::RequestTimedOut => write!(fmt, "PGRST003"),
            Self::ParsingErrorQueryParameter => write!(fmt, "PGRST100"),
            Self::FunctionOnlySupportsGetOrPost => write!(fmt, "PGRST101"),
            Self::InvalidRequestBody => write!(fmt, "PGRST102"),
            Self::InvalidRange => write!(fmt, "PGRST103"),
            Self::InvalidPutRequest => write!(fmt, "PGRST105"),
            Self::SchemaNotInConfig => write!(fmt, "PGRST106"),
            Self::InvalidContentType => write!(fmt, "PGRST107"),
            Self::FilterOnMissingEmbeddedResource => write!(fmt, "PGRST108"),
            Self::LimitedUpdateDeleteWithoutOrdering => write!(fmt, "PGRST109"),
            Self::LimitedUpdateDeleteExceededMaxRows => write!(fmt, "PGRST110"),
            Self::InvalidResponseHeaders => write!(fmt, "PGRST111"),
            Self::InvalidStatusCode => write!(fmt, "PGRST112"),
            Self::UpsertPutWithLimitsOffsets => write!(fmt, "PGRST114"),
            Self::UpsertPutPrimaryKeyMismatch => write!(fmt, "PGRST115"),
            Self::InvalidSingularResponse => write!(fmt, "PGRST116"),
            Self::UnsupportedHttpVerb => write!(fmt, "PGRST117"),
            Self::CannotOrderByRelatedTable => write!(fmt, "PGRST118"),
            Self::CannotSpreadRelatedTable => write!(fmt, "PGRST119"),
            Self::InvalidEmbeddedResourceFilter => write!(fmt, "PGRST120"),
            Self::InvalidRaiseErrorJson => write!(fmt, "PGRST121"),
            Self::InvalidPreferHeader => write!(fmt, "PGRST122"),
            Self::RelationshipNotFound => write!(fmt, "PGRST200"),
            Self::AmbiguousEmbedding => write!(fmt, "PGRST201"),
            Self::FunctionNotFound => write!(fmt, "PGRST202"),
            Self::OverloadedFunctionAmbiguous => write!(fmt, "PGRST203"),
            Self::ColumnNotFound => write!(fmt, "PGRST204"),
            Self::JwtSecretMissing => write!(fmt, "PGRST300"),
            Self::JwtInvalid => write!(fmt, "PGRST301"),
            Self::AnonymousRoleDisabled => write!(fmt, "PGRST302"),
            Self::InternalLibraryError => write!(fmt, "PGRSTX00"),
            Self::Other(code) => write!(fmt, "{code}"),
        }
    }
}

/// Represents a custom error.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct CustomError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
    pub hint: Option<String>,
}

impl CustomError {
    #[must_use]
    pub fn from_response(resp: ErrorResponse) -> Self {
        Self {
            code: resp.code,
            message: resp.message,
            details: resp.details,
            hint: resp.hint,
        }
    }
}

#[cfg(test)]
#[expect(clippy::panic, reason = "Allowed in test code for simplicity")]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "Allowed in test code for simplicity"
)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_error_transformation() {
        // Test a specific PostgreSQL error code: 23505 - Unique Violation
        let error_response = ErrorResponse {
            message: "duplicate key value violates unique constraint".to_owned(),
            code: "23505".to_owned(),
            details: Some("Key (id)=(1) already exists.".to_owned()),
            hint: None,
        };
        let is_authenticated = true;
        let error = PostgrestUtilError::from_error_response(error_response);

        match error {
            PostgrestUtilError::Postgres(pg_error) => {
                assert_eq!(pg_error.code, PostgresErrorCode::UniqueViolation);
                assert_eq!(pg_error.http_status_code(is_authenticated), 409);
                assert_eq!(
                    pg_error.message,
                    "duplicate key value violates unique constraint"
                );
                assert_eq!(
                    pg_error.details,
                    Some("Key (id)=(1) already exists.".to_owned())
                );
            }
            _ => panic!("Expected PostgresError"),
        }
    }

    #[test]
    fn test_postgrest_error_transformation() {
        // Test a PostgREST error code: PGRST116 - Invalid Singular Response
        let error_response = ErrorResponse {
            message: "More than one item found".to_owned(),
            code: "PGRST116".to_owned(),
            details: None,
            hint: Some("Use limit to restrict the number of results.".to_owned()),
        };
        let error = PostgrestUtilError::from_error_response(error_response);

        match error {
            PostgrestUtilError::Postgrest(pgrst_error) => {
                assert_eq!(
                    pgrst_error.code,
                    PostgrestErrorCode::InvalidSingularResponse
                );
                assert_eq!(pgrst_error.http_status_code(), 406);
                assert_eq!(pgrst_error.message, "More than one item found");
                assert_eq!(
                    pgrst_error.hint,
                    Some("Use limit to restrict the number of results.".to_owned())
                );
            }
            _ => panic!("Expected PostgrestError"),
        }
    }

    #[test]
    fn test_custom_error_transformation() {
        // Test a custom error code not matching any known codes
        let error_response = ErrorResponse {
            message: "Custom error message".to_owned(),
            code: "CUSTOM123".to_owned(),
            details: Some("Some custom details.".to_owned()),
            hint: Some("Some custom hint.".to_owned()),
        };
        let error = PostgrestUtilError::from_error_response(error_response);

        match error {
            PostgrestUtilError::Custom(custom_error) => {
                assert_eq!(custom_error.code, "CUSTOM123");
                assert_eq!(custom_error.message, "Custom error message");
                assert_eq!(
                    custom_error.details,
                    Some("Some custom details.".to_owned())
                );
                assert_eq!(custom_error.hint, Some("Some custom hint.".to_owned()));
            }
            _ => panic!("Expected CustomError"),
        }
    }

    #[test]
    fn test_insufficient_privilege_error_authenticated() {
        // Test error code 42501 - Insufficient Privilege when authenticated
        let error_response = ErrorResponse {
            message: "permission denied for relation".to_owned(),
            code: "42501".to_owned(),
            details: None,
            hint: None,
        };
        let is_authenticated = true;
        let error = PostgrestUtilError::from_error_response(error_response);

        match error {
            PostgrestUtilError::Postgres(pg_error) => {
                assert_eq!(pg_error.code, PostgresErrorCode::InsufficientPrivilege);
                assert_eq!(pg_error.http_status_code(is_authenticated), 403);
            }
            _ => panic!("Expected PostgresError"),
        }
    }

    #[test]
    fn test_insufficient_privilege_error_unauthenticated() {
        // Test error code 42501 - Insufficient Privilege when not authenticated
        let error_response = ErrorResponse {
            message: "permission denied for relation".to_owned(),
            code: "42501".to_owned(),
            details: None,
            hint: None,
        };
        let is_authenticated = false;
        let error = PostgrestUtilError::from_error_response(error_response);

        match error {
            PostgrestUtilError::Postgres(pg_error) => {
                assert_eq!(pg_error.code, PostgresErrorCode::InsufficientPrivilege);
                assert_eq!(pg_error.http_status_code(is_authenticated), 401);
            }
            _ => panic!("Expected PostgresError"),
        }
    }

    #[test]
    fn test_pattern_error_transformation() {
        // Test an error code that matches a pattern: 08006 - Connection Exception
        let error_response = ErrorResponse {
            message: "An error occurred while connecting to the database".to_owned(),
            code: "08006".to_owned(),
            details: None,
            hint: None,
        };
        let is_authenticated = true;
        let error = PostgrestUtilError::from_error_response(error_response);

        match error {
            PostgrestUtilError::Postgres(pg_error) => {
                assert_eq!(pg_error.code, PostgresErrorCode::ConnectionException);
                assert_eq!(pg_error.http_status_code(is_authenticated), 503);
            }
            _ => panic!("Expected PostgresError"),
        }
    }

    #[test]
    fn test_postgrest_internal_error() {
        // Test PostgREST internal error code: PGRSTX00
        let error_response = ErrorResponse {
            message: "Internal server error".to_owned(),
            code: "PGRSTX00".to_owned(),
            details: Some("An unexpected error occurred.".to_owned()),
            hint: None,
        };
        let error = PostgrestUtilError::from_error_response(error_response);

        match error {
            PostgrestUtilError::Postgrest(pgrst_error) => {
                assert_eq!(pgrst_error.code, PostgrestErrorCode::InternalLibraryError);
                assert_eq!(pgrst_error.http_status_code(), 500);
            }
            _ => panic!("Expected PostgrestError"),
        }
    }

    #[test]
    fn test_unknown_postgres_error_code() {
        // Test an unknown PostgreSQL error code
        let error_response = ErrorResponse {
            message: "Unknown error".to_owned(),
            code: "99999".to_owned(),
            details: None,
            hint: None,
        };
        let is_authenticated = true;
        let error = PostgrestUtilError::from_error_response(error_response);

        match &error {
            PostgrestUtilError::Postgres(pg_error) => {
                match &pg_error.code {
                    PostgresErrorCode::Other(code) => assert_eq!(code, "99999"),
                    _ => panic!("Expected Other variant"),
                }
                assert_eq!(pg_error.http_status_code(is_authenticated), 400);
            }
            _ => panic!("Expected PostgresError"),
        }
    }

    #[test]
    fn test_unknown_postgrest_error_code() {
        // Test an unknown PostgREST error code
        let error_response = ErrorResponse {
            message: "Unknown PostgREST error".to_owned(),
            code: "PGRST999".to_owned(),
            details: None,
            hint: None,
        };
        let error = PostgrestUtilError::from_error_response(error_response);

        match &error {
            PostgrestUtilError::Postgrest(pgrst_error) => {
                match &pgrst_error.code {
                    PostgrestErrorCode::Other(code) => assert_eq!(code, "PGRST999"),
                    _ => panic!("Expected Other variant"),
                }
                assert_eq!(pgrst_error.http_status_code(), 500);
            }
            _ => panic!("Expected PostgrestError"),
        }
    }

    #[test]
    fn test_raise_exception_error() {
        // Test error code P0001 - Raise Exception
        let error_response = ErrorResponse {
            message: "I refuse!".to_owned(),
            code: "P0001".to_owned(),
            details: Some("Pretty simple".to_owned()),
            hint: Some("There is nothing you can do.".to_owned()),
        };
        let is_authenticated = true;
        let error = PostgrestUtilError::from_error_response(error_response);

        match error {
            PostgrestUtilError::Postgres(pg_error) => {
                assert_eq!(pg_error.code, PostgresErrorCode::RaiseException);
                assert_eq!(pg_error.http_status_code(is_authenticated), 400);
                assert_eq!(pg_error.message, "I refuse!");
                assert_eq!(pg_error.details, Some("Pretty simple".to_owned()));
                assert_eq!(
                    pg_error.hint,
                    Some("There is nothing you can do.".to_owned())
                );
            }
            _ => panic!("Expected PostgresError"),
        }
    }

    #[test]
    fn test_custom_status_code_in_raise() {
        // Test a custom error using RAISE with PTxyz SQLSTATE
        let error_response = ErrorResponse {
            message: "Payment Required".to_owned(),
            code: "PT402".to_owned(),
            details: Some("Quota exceeded".to_owned()),
            hint: Some("Upgrade your plan".to_owned()),
        };
        let error = PostgrestUtilError::Custom(CustomError::from_response(error_response));

        match error {
            PostgrestUtilError::Custom(custom_error) => {
                assert_eq!(custom_error.code, "PT402");
                assert_eq!(custom_error.message, "Payment Required");
                assert_eq!(custom_error.details, Some("Quota exceeded".to_owned()));
                assert_eq!(custom_error.hint, Some("Upgrade your plan".to_owned()));
            }
            _ => panic!("Expected CustomError"),
        }
    }

    #[test]
    fn test_error_display_trait() {
        // Test that the Display trait is implemented correctly
        let error_response = ErrorResponse {
            message: "Not null violation".to_owned(),
            code: "23502".to_owned(),
            details: None,
            hint: None,
        };
        let error = PostgrestUtilError::from_error_response(error_response);

        assert_eq!(format!("{error}"), "Postgres [23502]: Not null violation");
    }

    #[test]
    fn test_error_trait() {
        // Test that the Error trait is implemented
        let error_response = ErrorResponse {
            message: "Some error".to_owned(),
            code: "23502".to_owned(),
            details: None,
            hint: None,
        };
        let error = PostgrestUtilError::from_error_response(error_response);

        let std_error: &dyn core::error::Error = &error;
        assert_eq!(std_error.to_string(), "Postgres [23502]: Some error");
    }

    #[test]
    fn non_standard_error() {
        let error_response = ErrorResponse {
            message: "no Route matched with those values".to_owned(),
            code: String::new(),
            details: None,
            hint: None,
        };
        let error = PostgrestUtilError::from_error_response(error_response);
        let std_error: &dyn core::error::Error = &error;
        assert_eq!(
            std_error.to_string(),
            "Custom []: no Route matched with those values"
        );
    }
}
