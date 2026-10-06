# rp-supabase-client

A Rust client for interacting with Supabase’s PostgREST API using authenticated requests.

## Overview

rp-supabase-client simplifies making authenticated requests to Supabase’s PostgREST API. It handles authentication, token refresh, and provides a straightforward API for querying data.

Features

- 	Easy authentication with Supabase.
- 	Automatic token refresh using rp-supabase-auth.
- 	Simple methods for querying and manipulating data.

```rust
use std::time::Duration;
use clap::Parser;
use futures::StreamExt;
use rp_supabase_auth::jwt_stream::SupabaseAuthConfig;
use rp_supabase_auth::types::LoginCredentials;
use rp_supabase_client::{new_authenticated, PostgerstResponse};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
struct Args {
    supabase_api_url: url::Url,
    anon_key: String,
    email: String,
    password: String,
    table: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let config = SupabaseAuthConfig {
        api_key: args.anon_key,
        url: args.supabase_api_url,
        max_reconnect_attempts: 5,
        reconnect_interval: Duration::from_secs(3),
    };

    let login_credentials = LoginCredentials::builder()
        .email(args.email)
        .password(args.password)
        .build();

    let mut client_stream = new_authenticated(config, login_credentials).unwrap();

    while let Some(client_result) = client_stream.next().await {
        if let Ok((client, _token_response)) = client_result {
            let res = client
                .from(&args.table)
                .select("*")
                .build()
                .send()
                .await
                .map(PostgerstResponse::<serde_json::Value>::new)
                .unwrap()
                .json()
                .await;

            println!("Response: {:?}", res);
        }
    }
}
```

## Generated schema bindings

Use [rp-supabase-codegen](../supabase-codegen/README.md) as a build dependency.
It generates Rust bindings from PostgreSQL catalogs or a committed offline snapshot.
No external generator CLI is required.

Generated relation modules expose `query(client)`. Queries retain the relation and response type.
`fetch()` returns decoded rows. `fetch_one()` requests exactly one row through PostgREST.
Both methods check HTTP status before decoding successful JSON.

```rust,ignore
use database::public::tables::messages;
use rp_supabase_client::projection;

projection! {
    struct MessageSummary for database::public::tables::messages {
        id,
        body,
    }
}

let rows = messages::query(client.clone())
    .select::<MessageSummary>()
    .eq(messages::columns::id, &message_id)
    .fetch()
    .await?;
```

The projection supplies both the selection and the field types. It preserves exact SQL response
keys, including renamed identifiers. Every selected field must exist, even when its value is null.
Extra response fields are ignored.

Column markers check filter ownership and scalar value types. String columns accept `&str`.
Nullable scalar comparisons take a non-null value. Use `is_null(column)` to test for SQL null.
`insert(&Insert)` and `update(&Update)` require the generated table payloads.
Only base tables support writes. A query cannot select a second mutation.

### Typed relationships

Use generated FK markers inside the same `projection!` macro:

```rust,ignore
projection! {
    struct AddressSummary for database::public::tables::addresses { id, label }
}
projection! {
    struct OrderSummary for database::public::tables::orders {
        id,
        billing: embed(database::public::tables::orders::relationships::orders_billing, AddressSummary),
        shipping: embed(database::public::tables::orders::relationships::orders_shipping, AddressSummary, inner),
    }
}

let rows = database::public::tables::orders::query(client.clone())
    .select::<OrderSummary>()
    .embedded(OrderSummary::billing, |address| {
        address.eq(database::public::tables::addresses::columns::label, "Main");
    })
    .exists(OrderSummary::billing)
    .fetch()
    .await?;
```

The field name is the response alias. Each handle checks its owning projection and target relation.
Compose selected child handles with `.then(...)` for nested filter paths.
Use `alias: empty(RelationshipMarker)` for a predicate-only embed, without a decoded field.
`exists(handle)` and `not_exists(handle)` test related-row existence, including empty embeds.

To-one embeds decode as `Option<Child>`. To-many embeds decode as `Vec<Child>`.
These types remain conservative with `inner`, because filters and RLS can hide related rows.
Normal child filters preserve parent rows. `inner` filters rows at the embed's parent level.
Embedded filters and existence predicates lock the selection. Choose `.select::<P>()` before them.
Root filters and one typed mutation remain available after locking.

For UPDATE and DELETE, use root column filters to constrain affected rows. Child filters shape returned representations.
PostgREST 16.2 rejects embed-alias existence predicates on DELETE. These remain native `QueryError::Execution` errors.
The runtime does not replace relationship predicates with FK null checks.

See the [generator relationship contracts](../supabase-codegen/README.md#typed-relationships)
for marker naming, reverse uniqueness, supported edges, and a nested example.


`into_raw()?` exposes the native PostgREST builder and drops typed guarantees.
Use it for arbitrary expressions, unsupported relationships, or array/composite comparisons without `Display`.
Generated RPC markers retain `schema::rpc::<Function>(client.clone(), &args)`.

Insert and update payloads distinguish omitted fields from explicit null.
`schema::Field<Option<T>>` represents omission, null, or a value.
`schema::Array<T>` preserves nullable elements and nested PostgreSQL arrays.
`schema::prelude` exports these helpers and the `include_schema!` macro.

Typed fetch methods return `schema::QueryError`. Its `Execution` variant retains the native
PostgREST error, HTTP status, headers, and URL. Other variants report request serialization,
successful response body reads, or selected-shape decoding failures.

Typed queries work without default features. Enable `serde_json/arbitrary_precision` when
decoding generated numeric fields through a schema-only dependency. The default `client`
feature already enables it. `PostgerstResponse::json` retains its existing nested error results.

Run the [complete build-script example](../supabase-codegen-example/README.md)
to exercise offline generation or live CRUD and RPC calls.
