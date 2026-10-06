# Supabase Rust utilities

[![Tests](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/test.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/test.yaml) [![Checks](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/check.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/check.yaml) [![Audit](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/audit.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/audit.yaml) [![Deny](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/deny.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/deny.yaml) [![Docs](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/doc.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/doc.yaml) [![Unused Dependencies](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/unused-deps.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/unused-deps.yaml) [![Conventional PR](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/conventional-pr.yaml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/conventional-pr.yaml) [![Release](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/release-plz.yml/badge.svg)](https://github.com/roberts-pumpurs/supabase-rs-utils/actions/workflows/release-plz.yml)

A collection of Rust crates for interacting with Supabase APIs, including Authentication, Realtime, and PostgREST.

## Overview

This repository is a Cargo workspace containing multiple Rust crates that provide clients and utilities for working with Supabase services in Rust. The crates included are:

- [rp-postgrest](./crates/postgrest/README.md): The workspace-owned raw PostgREST HTTP client, with checked execution and typed decoding.
- [rp-supabase-auth](./crates/supabase-auth/README.md): A client library for Supabase's Authentication API.
- [rp-postgrest-error](./crates/postgrest-error/README.md): Error parsing and handling for PostgREST and PostgreSQL responses.
- [rp-supabase-realtime](./crates/supabase-realtime/README.md): A client library for Supabase's Realtime API.
- [rp-supabase-client](./crates/supabase-client/README.md): A client for Supabase's PostgREST API with authenticated requests.
- [rp-supabase-codegen](./crates/supabase-codegen/README.md): Generate Rust schema bindings in `build.rs`, with offline snapshots and custom derives, attributes, and preludes.

## Getting started

The workspace crates require Rust 1.85 or later. Add the crates you need to `Cargo.toml`:

```toml
[dependencies]
rp-postgrest = "3.0"
rp-supabase-auth = "0.8"
rp-postgrest-error = "0.8"
rp-supabase-realtime = "0.8"
rp-supabase-client = "0.9"
rp-supabase-codegen = "0.9"
```

`rp-postgrest` is implemented in this workspace. `rp-supabase-client` builds typed queries on it and re-exports it as `rp_postgrest`. Both use the canonical `rp-postgrest-error` response model. See the [typed client documentation](./crates/supabase-client/README.md) for query-first selections, shared DTOs, typed ordering and filters, pagination, counts, minimal writes, and typed RPCs. Raw builders support checked decoding through `fetch::<T>()`.

## Examples

Check out our [examples directory](./examples/README.md) for complete working examples of how to use each crate. The examples cover:

- Authentication and user management
- Real-time database updates
- Broadcast messaging
- Presence tracking
- JWT token management
- Database operations
- [Build-script schema bindings](./crates/supabase-codegen-example/README.md), including offline generation and live PostgREST CRUD/RPC verification.

Each example is self-contained and includes detailed documentation about its use case and how to run it.

## Development guide

1. Install the pinned Rust 1.99.0 toolchain with `rustup show`. Development and CI use this stable release. The published crates retain Rust 1.85 support.
2. Run `cargo fmt --all --check`.
3. Run `cargo clippy --workspace --all-features --all-targets --locked -- -D warnings`.
4. Run `cargo test --workspace --all-features --all-targets --locked`.
5. Run `cargo test --workspace --all-features --doc --locked` for documentation examples.

`cargo xtask check` runs Clippy and formatting checks. `cargo xtask fmt` also applies compiler and Clippy fixes.

## Supabase instance for local development

1. [Install Supabase CLI](https://supabase.com/docs/guides/cli/getting-started)
2. Run `supabase start` to run local supabase instance
