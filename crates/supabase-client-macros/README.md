# rp-supabase-client-macros

[![crates.io](https://img.shields.io/crates/v/rp-supabase-client-macros.svg)](https://crates.io/crates/rp-supabase-client-macros) [![docs.rs](https://docs.rs/rp-supabase-client-macros/badge.svg)](https://docs.rs/rp-supabase-client-macros)

Procedural macros for [`rp-supabase-client`](https://docs.rs/rp-supabase-client).
They build constructor-checked, query-local selections and lossless schema keys.

Do not depend on this crate directly. Use the re-exports `rp_supabase_client::select`
and `rp_supabase_client::key`. The generated code refers to `rp-supabase-client`
items, so the macros do not work without it. The macro version always matches
the `rp-supabase-client` version that re-exports it.
