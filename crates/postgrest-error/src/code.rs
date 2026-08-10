use alloc::{borrow::ToOwned as _, string::String};
use core::fmt;

use http::StatusCode;
use serde::{Deserialize, Serialize};

/// Authentication state used when a `PostgreSQL` error maps differently for
/// anonymous and authenticated requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Authentication {
    Authenticated,
    Anonymous,
    Unknown,
}

/// Semantic classification of an error code returned by `PostgREST`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorKind {
    Postgres(PostgresErrorCode),
    Postgrest(PostgrestErrorCode),
    /// A `PTxyz` code that directly encodes an HTTP status.
    CustomStatus(StatusCode),
    /// A non-standard application or gateway code.
    Custom,
}

/// Semantic categories for `PostgreSQL` SQLSTATE values used by `PostgREST`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PostgresErrorCode {
    CardinalityViolation,
    InvalidParameterValue,
    NotNullViolation,
    ForeignKeyViolation,
    UniqueViolation,
    ReadOnlySqlTransaction,
    UndefinedFunction,
    UndefinedTable,
    InfiniteRecursion,
    InsufficientPrivilege,
    ConfigLimitExceeded,
    RaiseException,
    ConnectionException,
    TriggeredActionException,
    InvalidGrantor,
    InvalidRoleSpecification,
    InvalidTransactionState,
    InvalidAuthorizationSpecification,
    InvalidTransactionTermination,
    ExternalRoutineException,
    ExternalRoutineInvocationException,
    SavepointException,
    TransactionRollback,
    InsufficientResources,
    ProgramLimitExceeded,
    ObjectNotInPrerequisiteState,
    AdminShutdown,
    OperatorIntervention,
    SystemError,
    ConfigFileError,
    FdwError,
    PlpgsqlError,
    InternalError,
    Unknown,
}

/// `PostgREST` error codes documented by supported `PostgREST` versions.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PostgrestErrorCode {
    CouldNotConnectDatabase,
    InternalConnectionError,
    CouldNotConnectSchemaCache,
    RequestTimedOut,
    ParsingErrorQueryParameter,
    FunctionOnlySupportsGetOrPost,
    InvalidRequestBody,
    InvalidRange,
    InvalidPutRequest,
    SchemaNotInConfig,
    InvalidContentType,
    FilterOnMissingEmbeddedResource,
    InvalidResponseHeaders,
    InvalidStatusCode,
    UpsertPutWithLimitsOffsets,
    UpsertPutPrimaryKeyMismatch,
    InvalidSingularResponse,
    UnsupportedHttpVerb,
    CannotOrderByRelatedTable,
    InvalidEmbeddedResourceFilter,
    InvalidRaiseErrorJson,
    InvalidPreferHeader,
    AggregatesDisabled,
    MaxAffectedRowsExceeded,
    InvalidPath,
    OpenApiDisabled,
    FeatureNotImplemented,
    MaxAffectedRpcExceeded,
    RelationshipNotFound,
    AmbiguousEmbedding,
    FunctionNotFound,
    OverloadedFunctionAmbiguous,
    ColumnNotFound,
    TableNotFound,
    JwtSecretMissing,
    JwtInvalid,
    AnonymousRoleDisabled,
    JwtClaimsInvalid,
    InternalLibraryError,
    Unknown,
}

/// Lossless error code returned by `PostgREST`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct ErrorCode(String);

impl ErrorCode {
    /// Creates an error code while preserving its exact wire value.
    #[must_use]
    pub fn new<S: AsRef<str>>(code: S) -> Self {
        Self(code.as_ref().to_owned())
    }

    /// Returns the exact code received from `PostgREST`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Classifies the code without discarding its exact value.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        if is_postgrest_code(&self.0) {
            ErrorKind::Postgrest(postgrest_code(&self.0))
        } else if let Some(status) = custom_status(&self.0) {
            ErrorKind::CustomStatus(status)
        } else if is_sqlstate(&self.0) {
            ErrorKind::Postgres(postgres_code(&self.0))
        } else {
            ErrorKind::Custom
        }
    }

    /// Returns the HTTP status implied by the code when one is known.
    ///
    /// This is only a fallback for body-only use cases. When an HTTP response
    /// is available, its observed status is authoritative.
    #[must_use]
    pub fn inferred_status(&self, authentication: Authentication) -> Option<StatusCode> {
        match self.kind() {
            ErrorKind::Postgres(code) => postgres_status(code, authentication),
            ErrorKind::Postgrest(code) => postgrest_status(code),
            ErrorKind::CustomStatus(status) => Some(status),
            ErrorKind::Custom => None,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<&str> for ErrorCode {
    fn from(code: &str) -> Self {
        Self::new(code)
    }
}

impl From<String> for ErrorCode {
    fn from(code: String) -> Self {
        Self(code)
    }
}

impl AsRef<str> for ErrorCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

fn custom_status(code: &str) -> Option<StatusCode> {
    if code.len() != 5 {
        return None;
    }

    let status = code.strip_prefix("PT")?.parse::<u16>().ok()?;
    StatusCode::from_u16(status).ok()
}

fn is_postgrest_code(code: &str) -> bool {
    if code.len() != 8 || !code.starts_with("PGRST") {
        return false;
    }

    let mut suffix = code.bytes().skip(5);
    matches!(suffix.next(), Some(group) if group.is_ascii_digit() || group == b'X')
        && suffix.all(|byte| byte.is_ascii_digit())
}

fn is_sqlstate(code: &str) -> bool {
    code.len() == 5
        && code
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
}

fn postgres_code(code: &str) -> PostgresErrorCode {
    match code {
        "21000" => PostgresErrorCode::CardinalityViolation,
        "22023" => PostgresErrorCode::InvalidParameterValue,
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
        "57P01" => PostgresErrorCode::AdminShutdown,
        _ if code.starts_with("08") => PostgresErrorCode::ConnectionException,
        _ if code.starts_with("09") => PostgresErrorCode::TriggeredActionException,
        _ if code.starts_with("0L") => PostgresErrorCode::InvalidGrantor,
        _ if code.starts_with("0P") => PostgresErrorCode::InvalidRoleSpecification,
        _ if code.starts_with("25") => PostgresErrorCode::InvalidTransactionState,
        _ if code.starts_with("28") => PostgresErrorCode::InvalidAuthorizationSpecification,
        _ if code.starts_with("2D") => PostgresErrorCode::InvalidTransactionTermination,
        _ if code.starts_with("38") => PostgresErrorCode::ExternalRoutineException,
        _ if code.starts_with("39") => PostgresErrorCode::ExternalRoutineInvocationException,
        _ if code.starts_with("3B") => PostgresErrorCode::SavepointException,
        _ if code.starts_with("40") => PostgresErrorCode::TransactionRollback,
        _ if code.starts_with("53") => PostgresErrorCode::InsufficientResources,
        _ if code.starts_with("54") => PostgresErrorCode::ProgramLimitExceeded,
        _ if code.starts_with("55") => PostgresErrorCode::ObjectNotInPrerequisiteState,
        _ if code.starts_with("57") => PostgresErrorCode::OperatorIntervention,
        _ if code.starts_with("58") => PostgresErrorCode::SystemError,
        _ if code.starts_with("F0") => PostgresErrorCode::ConfigFileError,
        _ if code.starts_with("HV") => PostgresErrorCode::FdwError,
        _ if code.starts_with("P0") => PostgresErrorCode::PlpgsqlError,
        _ if code.starts_with("XX") => PostgresErrorCode::InternalError,
        _ => PostgresErrorCode::Unknown,
    }
}

const fn postgres_status(
    code: PostgresErrorCode,
    authentication: Authentication,
) -> Option<StatusCode> {
    match code {
        PostgresErrorCode::TriggeredActionException
        | PostgresErrorCode::InvalidTransactionState
        | PostgresErrorCode::InvalidTransactionTermination
        | PostgresErrorCode::ExternalRoutineException
        | PostgresErrorCode::ExternalRoutineInvocationException
        | PostgresErrorCode::SavepointException
        | PostgresErrorCode::TransactionRollback
        | PostgresErrorCode::ProgramLimitExceeded
        | PostgresErrorCode::ObjectNotInPrerequisiteState
        | PostgresErrorCode::OperatorIntervention
        | PostgresErrorCode::SystemError
        | PostgresErrorCode::ConfigFileError
        | PostgresErrorCode::FdwError
        | PostgresErrorCode::PlpgsqlError
        | PostgresErrorCode::InternalError
        | PostgresErrorCode::ConfigLimitExceeded
        | PostgresErrorCode::InfiniteRecursion => Some(StatusCode::INTERNAL_SERVER_ERROR),
        PostgresErrorCode::ConnectionException | PostgresErrorCode::InsufficientResources => {
            Some(StatusCode::SERVICE_UNAVAILABLE)
        }
        PostgresErrorCode::InvalidGrantor
        | PostgresErrorCode::InvalidRoleSpecification
        | PostgresErrorCode::InvalidAuthorizationSpecification => Some(StatusCode::FORBIDDEN),
        PostgresErrorCode::UndefinedTable => Some(StatusCode::NOT_FOUND),
        PostgresErrorCode::NotNullViolation | PostgresErrorCode::RaiseException => {
            Some(StatusCode::BAD_REQUEST)
        }
        PostgresErrorCode::ForeignKeyViolation | PostgresErrorCode::UniqueViolation => {
            Some(StatusCode::CONFLICT)
        }
        PostgresErrorCode::AdminShutdown => Some(StatusCode::SERVICE_UNAVAILABLE),
        PostgresErrorCode::ReadOnlySqlTransaction => Some(StatusCode::METHOD_NOT_ALLOWED),
        PostgresErrorCode::InsufficientPrivilege => match authentication {
            Authentication::Authenticated => Some(StatusCode::FORBIDDEN),
            Authentication::Anonymous => Some(StatusCode::UNAUTHORIZED),
            Authentication::Unknown => None,
        },
        PostgresErrorCode::CardinalityViolation
        | PostgresErrorCode::InvalidParameterValue
        | PostgresErrorCode::UndefinedFunction => None,
        PostgresErrorCode::Unknown => Some(StatusCode::BAD_REQUEST),
    }
}

fn postgrest_code(code: &str) -> PostgrestErrorCode {
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
        "PGRST111" => PostgrestErrorCode::InvalidResponseHeaders,
        "PGRST112" => PostgrestErrorCode::InvalidStatusCode,
        "PGRST114" => PostgrestErrorCode::UpsertPutWithLimitsOffsets,
        "PGRST115" => PostgrestErrorCode::UpsertPutPrimaryKeyMismatch,
        "PGRST116" => PostgrestErrorCode::InvalidSingularResponse,
        "PGRST117" => PostgrestErrorCode::UnsupportedHttpVerb,
        "PGRST118" => PostgrestErrorCode::CannotOrderByRelatedTable,
        "PGRST120" => PostgrestErrorCode::InvalidEmbeddedResourceFilter,
        "PGRST121" => PostgrestErrorCode::InvalidRaiseErrorJson,
        "PGRST122" => PostgrestErrorCode::InvalidPreferHeader,
        "PGRST123" => PostgrestErrorCode::AggregatesDisabled,
        "PGRST124" => PostgrestErrorCode::MaxAffectedRowsExceeded,
        "PGRST125" => PostgrestErrorCode::InvalidPath,
        "PGRST126" => PostgrestErrorCode::OpenApiDisabled,
        "PGRST127" => PostgrestErrorCode::FeatureNotImplemented,
        "PGRST128" => PostgrestErrorCode::MaxAffectedRpcExceeded,
        "PGRST200" => PostgrestErrorCode::RelationshipNotFound,
        "PGRST201" => PostgrestErrorCode::AmbiguousEmbedding,
        "PGRST202" => PostgrestErrorCode::FunctionNotFound,
        "PGRST203" => PostgrestErrorCode::OverloadedFunctionAmbiguous,
        "PGRST204" => PostgrestErrorCode::ColumnNotFound,
        "PGRST205" => PostgrestErrorCode::TableNotFound,
        "PGRST300" => PostgrestErrorCode::JwtSecretMissing,
        "PGRST301" => PostgrestErrorCode::JwtInvalid,
        "PGRST302" => PostgrestErrorCode::AnonymousRoleDisabled,
        "PGRST303" => PostgrestErrorCode::JwtClaimsInvalid,
        "PGRSTX00" => PostgrestErrorCode::InternalLibraryError,
        _ => PostgrestErrorCode::Unknown,
    }
}

fn postgrest_status(code: PostgrestErrorCode) -> Option<StatusCode> {
    match code {
        PostgrestErrorCode::CouldNotConnectDatabase
        | PostgrestErrorCode::InternalConnectionError
        | PostgrestErrorCode::CouldNotConnectSchemaCache => Some(StatusCode::SERVICE_UNAVAILABLE),
        PostgrestErrorCode::RequestTimedOut => Some(StatusCode::GATEWAY_TIMEOUT),
        PostgrestErrorCode::ParsingErrorQueryParameter
        | PostgrestErrorCode::InvalidRequestBody
        | PostgrestErrorCode::FilterOnMissingEmbeddedResource
        | PostgrestErrorCode::UpsertPutWithLimitsOffsets
        | PostgrestErrorCode::UpsertPutPrimaryKeyMismatch
        | PostgrestErrorCode::CannotOrderByRelatedTable
        | PostgrestErrorCode::InvalidEmbeddedResourceFilter
        | PostgrestErrorCode::InvalidPreferHeader
        | PostgrestErrorCode::AggregatesDisabled
        | PostgrestErrorCode::MaxAffectedRowsExceeded
        | PostgrestErrorCode::FeatureNotImplemented
        | PostgrestErrorCode::MaxAffectedRpcExceeded
        | PostgrestErrorCode::RelationshipNotFound
        | PostgrestErrorCode::ColumnNotFound => Some(StatusCode::BAD_REQUEST),
        PostgrestErrorCode::FunctionOnlySupportsGetOrPost
        | PostgrestErrorCode::InvalidPutRequest
        | PostgrestErrorCode::UnsupportedHttpVerb => Some(StatusCode::METHOD_NOT_ALLOWED),
        PostgrestErrorCode::InvalidRange => Some(StatusCode::RANGE_NOT_SATISFIABLE),
        PostgrestErrorCode::SchemaNotInConfig
        | PostgrestErrorCode::InvalidContentType
        | PostgrestErrorCode::InvalidSingularResponse => Some(StatusCode::NOT_ACCEPTABLE),
        PostgrestErrorCode::InvalidResponseHeaders
        | PostgrestErrorCode::InvalidStatusCode
        | PostgrestErrorCode::InvalidRaiseErrorJson
        | PostgrestErrorCode::JwtSecretMissing
        | PostgrestErrorCode::InternalLibraryError => Some(StatusCode::INTERNAL_SERVER_ERROR),
        PostgrestErrorCode::AmbiguousEmbedding
        | PostgrestErrorCode::OverloadedFunctionAmbiguous => StatusCode::from_u16(300).ok(),
        PostgrestErrorCode::FunctionNotFound
        | PostgrestErrorCode::InvalidPath
        | PostgrestErrorCode::OpenApiDisabled
        | PostgrestErrorCode::TableNotFound => Some(StatusCode::NOT_FOUND),
        PostgrestErrorCode::JwtInvalid
        | PostgrestErrorCode::AnonymousRoleDisabled
        | PostgrestErrorCode::JwtClaimsInvalid => Some(StatusCode::UNAUTHORIZED),
        PostgrestErrorCode::Unknown => None,
    }
}
