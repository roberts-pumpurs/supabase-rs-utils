# Comparison with other Rust Supabase clients

Checked in 2026-10. Projects change often; read their current docs before you decide.

This page compares this workspace (`rp-supabase` and its crates) with three other crates:

- [supabase_rs](https://github.com/xylex-group/supabase_rs) 0.8.0 ([README](https://raw.githubusercontent.com/xylex-group/supabase_rs/main/README.md), [Cargo.toml](https://raw.githubusercontent.com/xylex-group/supabase_rs/main/Cargo.toml), [crates.io](https://crates.io/crates/supabase_rs)).
- [postgrest](https://github.com/supabase-community/postgrest-rs) 1.6.0, last released 2023-07 ([README](https://raw.githubusercontent.com/supabase-community/postgrest-rs/master/README.md), [crates.io](https://crates.io/crates/postgrest)).
- [rust_supabase_sdk](https://github.com/Lenard-0/Rust-Supabase-SDK) 0.4.4 ([README](https://raw.githubusercontent.com/Lenard-0/Rust-Supabase-SDK/main/README.md), [crates.io](https://crates.io/crates/rust_supabase_sdk)).

"Unknown" means the primary sources above do not say.

| Topic | rp-supabase (this workspace) | supabase_rs | postgrest | rust_supabase_sdk |
|-------|------------------------------|-------------|-----------|-------------------|
| Typed results | `fetch::<T>()` decodes into your serde types | Unknown | No. Returns the raw HTTP response; you decode the body. | Yes. `Vec<T>` from the string builder and typed builder. |
| Compile-time column checks | Yes, with generated types (`select!`, typed columns) | Unknown | No | Yes, with generated `Column<R, V>` constants |
| Codegen | `build.rs` generator from a snapshot or a live database | `typegen` feature | No | `cargo supabase gen types` CLI |
| Auth: user sessions | Password, OTP, sign up, user update, sign out | Not listed | No | Email, phone, OTP, OAuth, anonymous; pluggable session stores |
| Auth: refresh | `refresh_session` and a refresh stream | Not listed | No | Unknown |
| Auth: MFA and admin | Request types for MFA factors and admin endpoints; no convenience methods | Not listed | No | Admin user management. MFA unknown. |
| Realtime | Database changes (typed), broadcast, presence. No websocket reconnect. | Not listed | No | `postgres_changes`, broadcast, presence (opt-in feature) |
| Storage | Buckets, objects, signed and public URLs. No TUS, no image transforms. | `storage` feature | No | Buckets, objects, signed URLs, image transforms |
| Edge functions | Invoke with JSON or raw bodies | Not listed | No | Invoke, with streamed responses |
| GraphQL | No | Yes, experimental (`graphql` feature) | No | Not listed |
| Error model | `thiserror` enums; PostgREST errors parsed into typed codes (`rp-postgrest-error`) | `Error` enum with HTTP status and provider details | Unknown | Unknown |
| TLS options | rustls only | `rustls` or `native_tls` feature | Unknown for 1.6.0 | `rustls` (default) or `native-tls` |
| Retries | Not built in for HTTP requests | Unknown | No | Exponential backoff on 429 and 5xx |
| MSRV | 1.85 | 1.85 | Not declared | 1.75 |

## Where others are ahead

- supabase_rs has GraphQL support (experimental) and a short string-based API for simple CRUD.
- postgrest is small and has one job. Use it if you want a minimal PostgREST builder and decode the body yourself.
- rust_supabase_sdk covers more of the Supabase surface in one crate: OAuth and anonymous sign-in, admin user methods, storage image transforms, streamed function responses, and automatic retries. It also supports native TLS and an older MSRV.

## Where this workspace differs

- Generated types come from `build.rs`, not a separate CLI. A committed snapshot lets builds run offline.
- PostgREST errors decode into typed PostgreSQL and PostgREST error codes.
- Each service is a separate crate. You can depend on one service without the others.
