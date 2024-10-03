use std::fmt;

use serde::{Deserialize, Serialize};

/// Represents the error response returned by PostgREST.
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
pub enum Error {
    PostgresError(PostgresError),
    PostgrestError(PostgrestError),
    CustomError(CustomError),
}

impl Error {
    /// Creates an `Error` from an `ErrorResponse`.
    pub fn from_error_response(resp: ErrorResponse) -> Self {
        if resp.code.starts_with("PGRST") {
            Error::PostgrestError(PostgrestError::from_response(resp))
        } else if resp.code.len() == 5 || resp.code.starts_with("XX") {
            Error::PostgresError(PostgresError::from_response(resp))
        } else {
            Error::CustomError(CustomError::from_response(resp))
        }
    }

    /// Returns the corresponding HTTP status code for the error.
    pub fn http_status_code(&self, is_authenticated: bool) -> u16 {
        match self {
            Error::PostgresError(err) => err.http_status_code(is_authenticated),
            Error::PostgrestError(err) => err.http_status_code(),
            Error::CustomError(_) => 400, // Default to 400 for custom errors
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::PostgresError(err) => {
                write!(f, "PostgresError [{:?}]: {}", err.code, err.message)
            }
            Error::PostgrestError(err) => {
                write!(f, "PostgrestError [{:?}]: {}", err.code, err.message)
            }
            Error::CustomError(err) => write!(f, "CustomError [{}]: {}", err.code, err.message),
        }
    }
}

impl std::error::Error for Error {}

/// Represents an error returned by PostgreSQL.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct PostgresError {
    pub code: PostgresErrorCode,
    pub message: String,
    pub details: Option<String>,
    pub hint: Option<String>,
}

impl PostgresError {
    pub fn from_response(resp: ErrorResponse) -> Self {
        let code = PostgresErrorCode::from_code(&resp.code);
        PostgresError {
            code,
            message: resp.message,
            details: resp.details,
            hint: resp.hint,
        }
    }

    pub fn http_status_code(&self, is_authenticated: bool) -> u16 {
        self.code.http_status_code(is_authenticated)
    }
}

/// Enum representing PostgreSQL error codes.
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
    pub fn from_code(code: &str) -> Self {
        match code {
            // Specific codes
            "23502" => PostgresErrorCode::NotNullViolation,
            "23503" => PostgresErrorCode::ForeignKeyViolation,
            "23505" => PostgresErrorCode::UniqueViolation,
            "25006" => PostgresErrorCode::ReadOnlySqlTransaction,
            "42883" => PostgresErrorCode::UndefinedFunction,
            "42P01" => PostgresErrorCode::UndefinedTable,
            "42P17" => PostgresErrorCode::InfiniteRecursion,
            "42501" => PostgresErrorCode::InsufficientPrivilege,
            "53400" => PostgresErrorCode::ConfigLimitExceeded,
            "P0001" => PostgresErrorCode::RaiseException,
            _ => {
                // Check for patterns
                if code.starts_with("08") {
                    PostgresErrorCode::ConnectionException
                } else if code.starts_with("09") {
                    PostgresErrorCode::TriggeredActionException
                } else if code.starts_with("0L") {
                    PostgresErrorCode::InvalidGrantor
                } else if code.starts_with("0P") {
                    PostgresErrorCode::InvalidRoleSpecification
                } else if code.starts_with("25") {
                    PostgresErrorCode::InvalidTransactionState
                } else if code.starts_with("28") {
                    PostgresErrorCode::InvalidAuthorizationSpecification
                } else if code.starts_with("2D") {
                    PostgresErrorCode::InvalidTransactionTermination
                } else if code.starts_with("38") {
                    PostgresErrorCode::ExternalRoutineException
                } else if code.starts_with("39") {
                    PostgresErrorCode::ExternalRoutineInvocationException
                } else if code.starts_with("3B") {
                    PostgresErrorCode::SavepointException
                } else if code.starts_with("40") {
                    PostgresErrorCode::TransactionRollback
                } else if code.starts_with("53") {
                    PostgresErrorCode::InsufficientResources
                } else if code.starts_with("54") {
                    PostgresErrorCode::ProgramLimitExceeded
                } else if code.starts_with("55") {
                    PostgresErrorCode::ObjectNotInPrerequisiteState
                } else if code.starts_with("57") {
                    PostgresErrorCode::OperatorIntervention
                } else if code.starts_with("58") {
                    PostgresErrorCode::SystemError
                } else if code.starts_with("F0") {
                    PostgresErrorCode::ConfigFileError
                } else if code.starts_with("HV") {
                    PostgresErrorCode::FdwError
                } else if code.starts_with("P0") {
                    PostgresErrorCode::PlpgsqlError
                } else if code.starts_with("XX") {
                    PostgresErrorCode::InternalError
                } else {
                    PostgresErrorCode::Other(code.to_string())
                }
            }
        }
    }

    pub fn http_status_code(&self, is_authenticated: bool) -> u16 {
        match self {
            // Patterns
            PostgresErrorCode::ConnectionException => 503,
            PostgresErrorCode::TriggeredActionException => 500,
            PostgresErrorCode::InvalidGrantor => 403,
            PostgresErrorCode::InvalidRoleSpecification => 403,
            PostgresErrorCode::InvalidTransactionState => 500,
            PostgresErrorCode::InvalidAuthorizationSpecification => 403,
            PostgresErrorCode::InvalidTransactionTermination => 500,
            PostgresErrorCode::ExternalRoutineException => 500,
            PostgresErrorCode::ExternalRoutineInvocationException => 500,
            PostgresErrorCode::SavepointException => 500,
            PostgresErrorCode::TransactionRollback => 500,
            PostgresErrorCode::InsufficientResources => 503,
            PostgresErrorCode::ProgramLimitExceeded => 500,
            PostgresErrorCode::ObjectNotInPrerequisiteState => 500,
            PostgresErrorCode::OperatorIntervention => 500,
            PostgresErrorCode::SystemError => 500,
            PostgresErrorCode::ConfigFileError => 500,
            PostgresErrorCode::FdwError => 500,
            PostgresErrorCode::PlpgsqlError => 500,
            PostgresErrorCode::InternalError => 500,
            // Specific codes
            PostgresErrorCode::NotNullViolation => 400,
            PostgresErrorCode::ForeignKeyViolation => 409,
            PostgresErrorCode::UniqueViolation => 409,
            PostgresErrorCode::ReadOnlySqlTransaction => 405,
            PostgresErrorCode::ConfigLimitExceeded => 500,
            PostgresErrorCode::RaiseException => 400,
            PostgresErrorCode::UndefinedFunction => 404,
            PostgresErrorCode::UndefinedTable => 404,
            PostgresErrorCode::InfiniteRecursion => 500,
            PostgresErrorCode::InsufficientPrivilege => {
                if is_authenticated {
                    403
                } else {
                    401
                }
            }
            // Other errors default to 400
            PostgresErrorCode::Other(_) => 400,
        }
    }
}

/// Represents an error returned by PostgREST.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct PostgrestError {
    pub code: PostgrestErrorCode,
    pub message: String,
    pub details: Option<String>,
    pub hint: Option<String>,
}

impl PostgrestError {
    pub fn from_response(resp: ErrorResponse) -> Self {
        let code = PostgrestErrorCode::from_code(&resp.code);
        PostgrestError {
            code,
            message: resp.message,
            details: resp.details,
            hint: resp.hint,
        }
    }

    pub fn http_status_code(&self) -> u16 {
        self.code.http_status_code()
    }
}

/// Enum representing PostgREST error codes.
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
    pub fn from_code(code: &str) -> Self {
        match code {
            "PGRST000" => PostgrestErrorCode::CouldNotConnectDatabase,
            "PGRST001" => PostgrestErrorCode::InternalConnectionError,
            "PGRST002" => PostgrestErrorCode::CouldNotConnectSchemaCache,
            "PGRST003" => PostgrestErrorCode::RequestTimedOut,
            "PGRST100" => PostgrestErrorCode::ParsingErrorQueryParameter,
            "PGRST101" => PostgrestErrorCode::FunctionOnlySupportsGetOrPost,
            "PGRST102" => PostgrestErrorCode::InvalidRequestBody,
            "PGRST103" => PostgrestErrorCode::InvalidRange,
            "PGRST105" => PostgrestErrorCode::InvalidPutRequest,
            "PGRST106" => PostgrestErrorCode::SchemaNotInConfig,
            "PGRST107" => PostgrestErrorCode::InvalidContentType,
            "PGRST108" => PostgrestErrorCode::FilterOnMissingEmbeddedResource,
            "PGRST109" => PostgrestErrorCode::LimitedUpdateDeleteWithoutOrdering,
            "PGRST110" => PostgrestErrorCode::LimitedUpdateDeleteExceededMaxRows,
            "PGRST111" => PostgrestErrorCode::InvalidResponseHeaders,
            "PGRST112" => PostgrestErrorCode::InvalidStatusCode,
            "PGRST114" => PostgrestErrorCode::UpsertPutWithLimitsOffsets,
            "PGRST115" => PostgrestErrorCode::UpsertPutPrimaryKeyMismatch,
            "PGRST116" => PostgrestErrorCode::InvalidSingularResponse,
            "PGRST117" => PostgrestErrorCode::UnsupportedHttpVerb,
            "PGRST118" => PostgrestErrorCode::CannotOrderByRelatedTable,
            "PGRST119" => PostgrestErrorCode::CannotSpreadRelatedTable,
            "PGRST120" => PostgrestErrorCode::InvalidEmbeddedResourceFilter,
            "PGRST121" => PostgrestErrorCode::InvalidRaiseErrorJson,
            "PGRST122" => PostgrestErrorCode::InvalidPreferHeader,
            "PGRST200" => PostgrestErrorCode::RelationshipNotFound,
            "PGRST201" => PostgrestErrorCode::AmbiguousEmbedding,
            "PGRST202" => PostgrestErrorCode::FunctionNotFound,
            "PGRST203" => PostgrestErrorCode::OverloadedFunctionAmbiguous,
            "PGRST204" => PostgrestErrorCode::ColumnNotFound,
            "PGRST300" => PostgrestErrorCode::JwtSecretMissing,
            "PGRST301" => PostgrestErrorCode::JwtInvalid,
            "PGRST302" => PostgrestErrorCode::AnonymousRoleDisabled,
            "PGRSTX00" => PostgrestErrorCode::InternalLibraryError,
            _ => PostgrestErrorCode::Other(code.to_string()),
        }
    }

    pub fn http_status_code(&self) -> u16 {
        match self {
            // Group 0 - Connection
            PostgrestErrorCode::CouldNotConnectDatabase |
            PostgrestErrorCode::InternalConnectionError |
            PostgrestErrorCode::CouldNotConnectSchemaCache => 503,
            PostgrestErrorCode::RequestTimedOut => 504,

            // Group 1 - API Request
            PostgrestErrorCode::ParsingErrorQueryParameter => 400,
            PostgrestErrorCode::FunctionOnlySupportsGetOrPost => 405,
            PostgrestErrorCode::InvalidRequestBody => 400,
            PostgrestErrorCode::InvalidRange => 416,
            PostgrestErrorCode::InvalidPutRequest => 405,
            PostgrestErrorCode::SchemaNotInConfig => 406,
            PostgrestErrorCode::InvalidContentType => 415,
            PostgrestErrorCode::FilterOnMissingEmbeddedResource => 400,
            PostgrestErrorCode::LimitedUpdateDeleteWithoutOrdering => 400,
            PostgrestErrorCode::LimitedUpdateDeleteExceededMaxRows => 400,
            PostgrestErrorCode::InvalidResponseHeaders => 500,
            PostgrestErrorCode::InvalidStatusCode => 500,
            PostgrestErrorCode::UpsertPutWithLimitsOffsets => 400,
            PostgrestErrorCode::UpsertPutPrimaryKeyMismatch => 400,
            PostgrestErrorCode::InvalidSingularResponse => 406,
            PostgrestErrorCode::UnsupportedHttpVerb => 405,
            PostgrestErrorCode::CannotOrderByRelatedTable => 400,
            PostgrestErrorCode::CannotSpreadRelatedTable => 400,
            PostgrestErrorCode::InvalidEmbeddedResourceFilter => 400,
            PostgrestErrorCode::InvalidRaiseErrorJson => 500,
            PostgrestErrorCode::InvalidPreferHeader => 400,

            // Group 2 - Schema Cache
            PostgrestErrorCode::RelationshipNotFound => 400,
            PostgrestErrorCode::AmbiguousEmbedding => 300,
            PostgrestErrorCode::FunctionNotFound => 404,
            PostgrestErrorCode::OverloadedFunctionAmbiguous => 300,
            PostgrestErrorCode::ColumnNotFound => 400,

            // Group 3 - JWT
            PostgrestErrorCode::JwtSecretMissing => 500,
            PostgrestErrorCode::JwtInvalid => 401,
            PostgrestErrorCode::AnonymousRoleDisabled => 401,

            // Group X - Internal
            PostgrestErrorCode::InternalLibraryError => 500,

            // Other errors
            PostgrestErrorCode::Other(_) => 500,
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
    pub fn from_response(resp: ErrorResponse) -> Self {
        CustomError {
            code: resp.code,
            message: resp.message,
            details: resp.details,
            hint: resp.hint,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_error_transformation() {
        // Test a specific PostgreSQL error code: 23505 - Unique Violation
        let error_response = ErrorResponse {
            message: "duplicate key value violates unique constraint".to_string(),
            code: "23505".to_string(),
            details: Some("Key (id)=(1) already exists.".to_string()),
            hint: None,
        };
        let is_authenticated = true;
        let error = Error::from_error_response(error_response);

        match error {
            Error::PostgresError(pg_error) => {
                assert_eq!(pg_error.code, PostgresErrorCode::UniqueViolation);
                assert_eq!(pg_error.http_status_code(is_authenticated), 409);
                assert_eq!(
                    pg_error.message,
                    "duplicate key value violates unique constraint"
                );
                assert_eq!(
                    pg_error.details,
                    Some("Key (id)=(1) already exists.".to_string())
                );
            }
            _ => panic!("Expected PostgresError"),
        }
    }

    #[test]
    fn test_postgrest_error_transformation() {
        // Test a PostgREST error code: PGRST116 - Invalid Singular Response
        let error_response = ErrorResponse {
            message: "More than one item found".to_string(),
            code: "PGRST116".to_string(),
            details: None,
            hint: Some("Use limit to restrict the number of results.".to_string()),
        };
        let error = Error::from_error_response(error_response);

        match error {
            Error::PostgrestError(pgrst_error) => {
                assert_eq!(
                    pgrst_error.code,
                    PostgrestErrorCode::InvalidSingularResponse
                );
                assert_eq!(pgrst_error.http_status_code(), 406);
                assert_eq!(pgrst_error.message, "More than one item found");
                assert_eq!(
                    pgrst_error.hint,
                    Some("Use limit to restrict the number of results.".to_string())
                );
            }
            _ => panic!("Expected PostgrestError"),
        }
    }

    #[test]
    fn test_custom_error_transformation() {
        // Test a custom error code not matching any known codes
        let error_response = ErrorResponse {
            message: "Custom error message".to_string(),
            code: "CUSTOM123".to_string(),
            details: Some("Some custom details.".to_string()),
            hint: Some("Some custom hint.".to_string()),
        };
        let error = Error::from_error_response(error_response);

        match error {
            Error::CustomError(custom_error) => {
                assert_eq!(custom_error.code, "CUSTOM123");
                assert_eq!(custom_error.message, "Custom error message");
                assert_eq!(
                    custom_error.details,
                    Some("Some custom details.".to_string())
                );
                assert_eq!(custom_error.hint, Some("Some custom hint.".to_string()));
            }
            _ => panic!("Expected CustomError"),
        }
    }

    #[test]
    fn test_insufficient_privilege_error_authenticated() {
        // Test error code 42501 - Insufficient Privilege when authenticated
        let error_response = ErrorResponse {
            message: "permission denied for relation".to_string(),
            code: "42501".to_string(),
            details: None,
            hint: None,
        };
        let is_authenticated = true;
        let error = Error::from_error_response(error_response);

        match error {
            Error::PostgresError(pg_error) => {
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
            message: "permission denied for relation".to_string(),
            code: "42501".to_string(),
            details: None,
            hint: None,
        };
        let is_authenticated = false;
        let error = Error::from_error_response(error_response);

        match error {
            Error::PostgresError(pg_error) => {
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
            message: "An error occurred while connecting to the database".to_string(),
            code: "08006".to_string(),
            details: None,
            hint: None,
        };
        let is_authenticated = true;
        let error = Error::from_error_response(error_response);

        match error {
            Error::PostgresError(pg_error) => {
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
            message: "Internal server error".to_string(),
            code: "PGRSTX00".to_string(),
            details: Some("An unexpected error occurred.".to_string()),
            hint: None,
        };
        let error = Error::from_error_response(error_response);

        match error {
            Error::PostgrestError(pgrst_error) => {
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
            message: "Unknown error".to_string(),
            code: "99999".to_string(),
            details: None,
            hint: None,
        };
        let is_authenticated = true;
        let error = Error::from_error_response(error_response);

        match &error {
            Error::PostgresError(pg_error) => {
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
            message: "Unknown PostgREST error".to_string(),
            code: "PGRST999".to_string(),
            details: None,
            hint: None,
        };
        let error = Error::from_error_response(error_response);

        match &error {
            Error::PostgrestError(pgrst_error) => {
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
            message: "I refuse!".to_string(),
            code: "P0001".to_string(),
            details: Some("Pretty simple".to_string()),
            hint: Some("There is nothing you can do.".to_string()),
        };
        let is_authenticated = true;
        let error = Error::from_error_response(error_response);

        match error {
            Error::PostgresError(pg_error) => {
                assert_eq!(pg_error.code, PostgresErrorCode::RaiseException);
                assert_eq!(pg_error.http_status_code(is_authenticated), 400);
                assert_eq!(pg_error.message, "I refuse!");
                assert_eq!(pg_error.details, Some("Pretty simple".to_string()));
                assert_eq!(
                    pg_error.hint,
                    Some("There is nothing you can do.".to_string())
                );
            }
            _ => panic!("Expected PostgresError"),
        }
    }

    #[test]
    fn test_custom_status_code_in_raise() {
        // Test a custom error using RAISE with PTxyz SQLSTATE
        let error_response = ErrorResponse {
            message: "Payment Required".to_string(),
            code: "PT402".to_string(),
            details: Some("Quota exceeded".to_string()),
            hint: Some("Upgrade your plan".to_string()),
        };
        let error = Error::CustomError(CustomError::from_response(error_response));

        match error {
            Error::CustomError(custom_error) => {
                assert_eq!(custom_error.code, "PT402");
                assert_eq!(custom_error.message, "Payment Required");
                assert_eq!(custom_error.details, Some("Quota exceeded".to_string()));
                assert_eq!(custom_error.hint, Some("Upgrade your plan".to_string()));
            }
            _ => panic!("Expected CustomError"),
        }
    }

    #[test]
    fn test_error_display_trait() {
        // Test that the Display trait is implemented correctly
        let error_response = ErrorResponse {
            message: "Not null violation".to_string(),
            code: "23502".to_string(),
            details: None,
            hint: None,
        };
        let error = Error::from_error_response(error_response);

        assert_eq!(
            format!("{}", error),
            "PostgresError [NotNullViolation]: Not null violation"
        );
    }

    #[test]
    fn test_error_trait() {
        // Test that the Error trait is implemented
        let error_response = ErrorResponse {
            message: "Some error".to_string(),
            code: "23502".to_string(),
            details: None,
            hint: None,
        };
        let error = Error::from_error_response(error_response);

        let std_error: &dyn std::error::Error = &error;
        assert_eq!(
            std_error.to_string(),
            "PostgresError [NotNullViolation]: Some error"
        );
    }

    #[test]
    fn non_standard_error() {
        let error_response = ErrorResponse {
            message: "no Route matched with those values".to_string(),
            code: "".to_string(),
            details: None,
            hint: None,
        };
        let error = Error::from_error_response(error_response);
        let std_error: &dyn std::error::Error = &error;
        assert_eq!(
            std_error.to_string(),
            "CustomError []: no Route matched with those values"
        );
    }
}
