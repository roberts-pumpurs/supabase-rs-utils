# Supabase Rust utilities

[![Tests](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/test.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/test.yaml) [![Checks](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/check.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/check.yaml) [![Audit](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/audit.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/audit.yaml) [![Deny](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/deny.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/deny.yaml) [![Docs](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/doc.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/doc.yaml) [![Unused Dependencies](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/unused-deps.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/unused-deps.yaml) [![Conventional PR](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/conventional-pr.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/conventional-pr.yaml) [![Release](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/release-plz.yml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/release-plz.yml)

A collection of Rust crates for interacting with Supabase APIs, including Authentication, Realtime, and PostgREST.

## Overview

This repository is a Cargo workspace containing multiple Rust crates that provide clients and utilities for working with Supabase services in Rust. The crates included are:

- [rp-supabase-auth](./crates/supabase-auth/README.md): A client library for Supabase's Authentication API.
- [rp-postgrest-error](./crates/postgrest-error/README.md): Error parsing and handling for PostgREST and PostgreSQL responses.
- [rp-supabase-realtime](./crates/supabase-realtime/README.md): A client library for Supabase's Realtime API.
- [rp-supabase-client](./crates/supabase-client/README.md): A client for Supabase's PostgREST API with authenticated requests.

## Getting Started

To use any of these crates, add them as dependencies in your Cargo.toml file:

```toml
[dependencies]
rp-supabase-auth = "0.1.0"
rp-postgrest-error = "0.1.0"
rp-supabase-realtime = "0.1.0"
rp-supabase-client = "0.1.0"
```

## Examples

Check out our [examples directory](./examples/README.md) for complete working examples of how to use each crate. The examples cover:

- Authentication and user management
- Real-time database updates
- Broadcast messaging
- Presence tracking
- JWT token management
- Database operations

Each example is self-contained and includes detailed documentation about its use case and how to run it.

## Development guide

1. [Install Rust](https://rustup.rs/)
2. All tasks and actions are managed by `cargo xtask`. Try running `cargo xtask --help` to see all the available commands.
3. `cargo xtask fmt` -- format the code
4. `cargo xtask test -- my_test_name` -- run specific tests

## Supabase instance for local development

1. [Install Supabase CLI](https://supabase.com/docs/guides/cli/getting-started)
2. Run `supabase start` to run local supabase instance
