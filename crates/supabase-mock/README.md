# rp-supabase-mock

[![crates.io](https://img.shields.io/crates/v/rp-supabase-mock.svg)](https://crates.io/crates/rp-supabase-mock) [![docs.rs](https://docs.rs/rp-supabase-mock/badge.svg)](https://docs.rs/rp-supabase-mock)

Test helpers for code that talks to Supabase auth. The crate starts a local
[`mockito`](https://docs.rs/mockito) server and registers the auth token
endpoints. It also creates and parses test JWTs.

Use it in tests only. The tokens are signed with a fixed test secret.

## Install

```toml
[dev-dependencies]
rp-supabase-mock = "0.8"
```

Requires Rust 1.85 or later.

## Example

Create a test JWT and read back its claims:

```rust
use core::time::Duration;

let jwt = rp_supabase_mock::make_jwt(Duration::from_secs(3600))?;
let _claims = rp_supabase_mock::parse_jwt(&jwt)?;
# Ok::<(), rp_supabase_mock::JwtParseError>(())
```

Start a mock server and register the password and refresh grants:

```rust,no_run
use core::time::Duration;
use rp_supabase_mock::{SupabaseMockServer, make_jwt};

async fn mock_auth() -> Result<url::Url, Box<dyn std::error::Error>> {
    let mut server = SupabaseMockServer::new().await;
    let jwt = make_jwt(Duration::from_secs(3600))?;
    server.register_jwt(&jwt)?;
    // Point the auth client at this URL.
    Ok(server.server_url()?)
}
```

`SupabaseMockServer` stays alive while the value exists. Keep it in scope for
the whole test.
