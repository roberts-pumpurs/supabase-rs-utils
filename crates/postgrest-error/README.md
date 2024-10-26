# rp-postgrest-error

A Rust crate for parsing and handling error responses from PostgREST and PostgreSQL, providing structured error types and utility functions.

## Features

- Structured Error Types: Differentiate between PostgreSQL errors, PostgREST errors, and custom errors.
- HTTP Status Mapping: Map error codes to corresponding HTTP status codes for API responses.
- Detailed Error Information: Access error messages, details, and hints for better error handling.
- Trait Implementations: Implements std::error::Error and Display traits for seamless integration with Rust’s error handling ecosystem.

## Installation

Add the following to your Cargo.toml:

```toml
[dependencies]
rp-postgrest-error = "0.1.0"
```

Replace "0.1.0" with the latest version of the crate.

## Usage

Parsing Error Responses

The crate provides an Error enum that represents different types of errors that can occur when interacting with PostgREST:

- 	Error::PostgresError for PostgreSQL errors.
- 	Error::PostgrestError for PostgREST-specific errors.
- 	Error::CustomError for any other errors.

To parse an error response from PostgREST, you can use the from_error_response method:

```rust
use rp_postgrest_error::{Error, ErrorResponse};

// Example error response from PostgREST
let error_response = ErrorResponse {
    message: "duplicate key value violates unique constraint".to_owned(),
    code: "23505".to_owned(),
    details: Some("Key (id)=(1) already exists.".to_owned()),
    hint: None,
};

// Parse the error response
let error = Error::from_error_response(error_response);

match error {
    Error::PostgresError(pg_error) => {
        println!("PostgreSQL Error: {}", pg_error.message);
    }
    Error::PostgrestError(pgrst_error) => {
        println!("PostgREST Error: {}", pgrst_error.message);
    }
    Error::CustomError(custom_error) => {
        println!("Custom Error: {}", custom_error.message);
    }
}
```
