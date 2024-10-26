# Supabase Rust utilities

A collection of Rust crates for interacting with Supabase APIs, including Authentication, Realtime, and PostgREST.

## Overview

This repository is a Cargo workspace containing multiple Rust crates that provide clients and utilities for working with Supabase services in Rust. The crates included are:

- [rp-supabase-auth](./crates/supabase-auth/README.md): A client library for Supabase’s Authentication API.
- [rp-postgrest-error](./crates/postgrest-error/README.md): Error parsing and handling for PostgREST and PostgreSQL responses.
- [rp-supabase-realtime](./crates/supabase-realtime/README.md): A client library for Supabase’s Realtime API.
- [rp-supabase-client](./crates/supabase-client/README.md): A client for Supabase’s PostgREST API with authenticated requests.

Getting Started

To use any of these crates, add them as dependencies in your Cargo.toml file:

```toml
[dependencies]
rp-supabase-auth = "0.1.0"
rp-postgrest-error = "0.1.0"
rp-supabase-realtime = "0.1.0"
rp-supabase-client = "0.1.0"
```

## Development guide

1. [Install Rust](https://rustup.rs/)
2. All tasks and actions are managed by `cargo xtask`. Try running `cargo xtask --help` to see all the available commands.
3. `cargo xtask fmt` -- format the code
4. `cargo xtask test -- my_test_name` -- run specific tests

## Supabase instance for local development

1. [Install Supabase CLI](https://supabase.com/docs/guides/cli/getting-started)
2. Run `supabase start` to run local supabase instance
