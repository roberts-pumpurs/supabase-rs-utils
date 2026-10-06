# rp-supabase-client

Supabase authentication and query-first typed PostgreSQL queries. Version 0.9 uses the workspace-owned [rp-postgrest 3.0](../postgrest/README.md). Raw and typed requests share its checked execution, JSON decoder, and flat error type.

```toml
[dependencies]
rp-supabase-client = "0.9"
```

Use [rp-supabase-codegen](../supabase-codegen/README.md) in a build script to generate relation, column, relationship, payload, and function markers. The snippets below assume the generated `database` module from the [complete example](../supabase-codegen-example/README.md). Substitute your own generated names and value types.

## Query-first selections

Regenerate bindings with codegen 0.9. Describe the fields at the query instead of declaring a DTO for every local result:

```rust,ignore
use rp_supabase_client::{key, select};
use rp_supabase_client::schema::Selection;

let selected = {
    use database::public::tables::orders::Row as LocalOrder;
    select!(LocalOrder => {
        id, label,
        billing: orders_billing {
            label, country: address_country { name },
        },
        shipping: orders_shipping { label },
    })
};
let country = selected.billing.then(selected.billing.child.country);
let rows = selected.query(client.clone())
    .embedded(selected.billing, |billing| {
        billing.eq(selected.billing.column(key!(id)), &10);
    })
    .embedded(country, |child| {
        child.eq(country.column(key!(name)), "UK");
    })
    .fetch().await?;
```

The FK determines each child relation. The result has ordinary named Rust fields with owned values. Filtering `billing.id` does not add that column to the response selection. `selected.billing.child.country` is relative to billing; `.then` composes a path from the root.

Function-local imports, concrete aliases, and bounded generic root/DTO types work. `key!(type id)` supplies the lookup-name type for a generic `ColumnByKey` bound. Keys encode the complete Rust identifier without hashing. Lookup uses the exact generated row type, including its schema, and rendering retains the exact SQL name.

Use `billing: inner(orders_billing) { label }` for an inner embed. `matching: empty(orders_billing)` creates a predicate-only handle without a decoded field. Named child reuse uses `billing: embed(orders_billing, AddressSummary)`, optionally with `inner` as the third argument.

For a renamed Cargo dependency, pass `runtime = renamed_client;` before the root type. Use the same prefix in `key!`. Generated bindings use `Generator::runtime_path("::renamed_client::schema")`.

Selections and inline handles are copyable zero-byte values. Each macro expansion has a distinct owner. Shared named child descendants use `selected.billing.then(AddressSummary::country)` or `AddressSummary::country` inside the billing filter scope. They do not expose `.child.country`. Paths containing named DTO handles retain their stored alias strings.

Missing selected fields fail decoding, including nullable fields. Explicit null is accepted where the schema permits it. Duplicate selected keys fail; extra keys are ignored. Named `projection!` and query-local `select!` compile through the same streaming typed decoder and exact-capacity field/embed renderer, without an intermediate JSON map. Neither local selection nor named child reuse requires `Debug` or `Serialize`. Generated record implementations for those traits depend on the selected values.

Keep named DTOs for reusable children and stable public return types. Move local result fields into an application DTO when needed; no JSON conversion is required.

## Shared projections and typed reads

A projection defines both the selection and response field types. `Projection<R>` is parameterized by the relation, so one DTO can implement several relation contracts:

```rust,ignore
use database::public::tables::{adapters, skills};
use rp_supabase_client::projection;
use rp_supabase_client::schema::{Count, Nulls, Order, named};

projection! {
    struct Artifact for [database::public::tables::skills, database::public::tables::adapters] {
        id,
        name,
        owner_id,
    }
}

let names = ["search", "storage"];
let page = skills::query(client.clone())
    .select(named::<_, Artifact>())
    .in_(skills::columns::name, names)
    .order_with_nulls(skills::columns::name, Order::Asc, Nulls::Last)
    .order(skills::columns::id, Order::Desc)
    .json_text_eq(skills::columns::manifest, &["fingerprint"], "abc123")?
    .range(0, 24)
    .fetch_with_count(Count::Exact)
    .await?;
let same_dto = adapters::query(client.clone())
    .select(named::<_, Artifact>())
    .limit(10)
    .fetch()
    .await?;
println!("{} rows, {} total", page.data.len(), page.count);
```

The shared macro emits one DTO and a `Projection<R>` implementation for each relation. Every selected field must exist on each relation with exactly the same Rust value type and SQL response key. Unselected fields need not match. Selection preserves exact SQL response keys, including renamed identifiers. Missing selected fields are decoding errors even when their type allows null; extra fields are ignored. Named projections retain their attributes, visibility, and DTO-owned relationship handles. `projection!` resolves its runtime through `$crate`, including renamed dependencies; the hidden proc-macro support is version-owned implementation, not additional caller syntax.

Column markers enforce relation ownership and scalar filter types. String columns accept borrowed `str`; nullable comparisons take a non-null value. `is_null(column)` requires a nullable column. Typed `in_` accepts borrowed scalar values and quotes/escapes them in list context. `order` composes multiple terms; `order_with_nulls` accepts `Nulls::First` or `Last`. `json_text_eq` requires a JSON/JSONB column and a nonempty key path. It escapes path identifiers and leaves the scalar value literal; an empty path returns a configuration error.

`limit` and inclusive `range` transition a read into `Paged`. Paged queries retain selection, filters, ordering, fetch, and counts, but have no insert/update/delete methods. This prevents response pagination from being mistaken for a safe mutation limiter. Ordering alone does not prohibit writes. `limit(0)` requests zero rows.

`fetch()` returns `Vec<P>`. `fetch_one()` returns `P` and requests PostgREST's single-object cardinality semantics, not the first row of an array. Read and Paged queries expose `.count(Count::Exact).await?` for a body-free total. `fetch_with_count` returns `Counted<Vec<P>>` with `.data` and `.count`. Planned and estimated count modes are also available. Missing or invalid server totals are errors, not zero. Counts preserve requested pagination.

## Minimal writes and raw decoding

```rust,ignore
use database::public::tables::skills;
use rp_supabase_client::schema::{Count, named};

// `patch` is this generated table's Update payload.
let affected = skills::query(client.clone())
    .eq(skills::columns::name, "search")
    .update(&patch)
    .execute_with_count(Count::Exact)
    .await?;

skills::query(client.clone())
    .eq(skills::columns::name, "obsolete")
    .delete()
    .execute()
    .await?;

let rows = skills::query(client.clone())
    .select(named::<_, Artifact>())
    .into_raw()
    .fetch::<Vec<Artifact>>()
    .await?;
```

Write `.execute()` requests minimal return and does not decode a row body. `.execute_with_count(Count)` returns the affected-row total without fetching identifiers. Write `.fetch()` still requests and decodes representations. Generated insert/update payloads retain exact table ownership. Only base tables support typed writes; views support typed reads. A query can choose only one mutation.

`into_raw()` returns the owned `rp_postgrest::Builder` directly, applies the final projection, and drops typed guarantees. Use its public `fetch::<Vec<P>>()` for checked raw decoding; no separate raw decoder is needed. Use the raw builder for arbitrary expressions, unsupported relationships, or array/composite comparisons without `Display`. Raw `in_` takes grammar fragments; raw `in_values` takes literal list elements. Raw pagination does not promise safely limited mutations.

`schema::Field<Option<T>>` distinguishes omission, explicit null, and a value. `schema::Array<T>` preserves nullable elements and nested PostgreSQL arrays. The default `client` feature enables `serde_json/arbitrary_precision` for exact generated PostgreSQL numerics. With default features disabled, explicitly enable `serde_json/arbitrary_precision` when decoding numeric fields. Do not convert exact numeric fields through `f64` if their precision matters.

## Pure parameters for another HTTP client

`schema::params` needs no `Postgrest` instance. It produces unencoded `QueryPair` values with the same typed ownership and grammar as query methods:

```rust,ignore
use database::public::tables::skills;
use rp_supabase_client::schema::{Order, params};

let pairs = [
    params::projection::<skills::Row, Artifact>(),
    params::eq(skills::columns::name, "search"),
    params::in_(skills::columns::name, ["search", "storage"]),
    params::order(skills::columns::name, Order::Asc),
    params::json_text_eq(skills::columns::manifest, &["fingerprint"], "abc123")?,
    params::limit(100),
    params::offset(0),
];
let response = http.get("https://example.supabase.co/rest/v1/skills")
    .query(&pairs)
    .send()
    .await?;
```

This example uses another Reqwest request directly; callers can pass pairs to another HTTP serializer. Do not percent-encode them first. The module also provides `neq`, `gt`, `gte`, `lt`, `lte`, nullable `is_null`, and `order_with_nulls`. Separate order pairs remain separate pairs; combine their terms into one order value if your transport/server requires multiple ordering terms. These helpers do not send requests, add auth/schema headers, or check responses.

For a table chosen at runtime, use `select` and literal-column `order_by` without generated relation markers:

```rust,ignore
let table = "audit events";
let mut url = reqwest::Url::parse("https://example.supabase.co/rest/v1/")?;
url.path_segments_mut().unwrap().push(table);
let mut pairs = vec![
    params::select("*"),
    params::order_by("created_at", Order::Desc),
];
pairs.extend(params::range(100, 199)?);
let response = http.get(url).query(&pairs).send().await?;
```

`range(low, high)` returns offset and limit pairs for an inclusive range. It rejects reversed bounds and row-count overflow with `params::RangeError`. `limit(0)` requests zero rows. `order_by` escapes a literal column name; `select` accepts selection grammar. `order_by_with_nulls` adds explicit null placement.

Use `params::scope(&["tasks"], params::limit(5))` for a runtime embedded relation name or alias. It emits `tasks.limit=5`. Nested scopes take separate path segments. This limits selected child rows, not the root table, and does not add the embed itself.

## Typed RPC returns

```rust,ignore
use database::public::functions::echo_message;
use rp_supabase_client::schema::rpc;

let args = echo_message::Args { message: Some("rpc example".into()) };
let result = rpc::<echo_message::Function>(client.clone(), &args).fetch().await?;
```

`args` is the generated function's `Args`; `result` is inferred as its `Returns`. Generated scalar, set-returning, composite, and void contracts decode their server representations. RPC does not require a relation projection or automatically add `select` or `single`. Serialization errors are deferred to execution.

Use `fetch_as::<Outcome>()` to decode JSON/JSONB directly into a caller-defined type. Use `single()` for server-enforced single-object mode:

```rust,ignore
let outcome = rpc::<delete_lifecycle::Function>(client.clone(), &args)
    .single()
    .fetch_as::<Outcome>()
    .await?;

rpc::<delete_lifecycle::Function>(client.clone(), &args)
    .execute()
    .await?;
```

Replace the function marker, arguments, and `Outcome` with your generated function and response type. `single()` sets the object Accept header but retains the generated return type. A set-returning function therefore needs `single().fetch_as::<Row>()`, not `single().fetch()`, to decode one row.

`execute()` checks status without reading or decoding a successful body. It preserves structured server errors and accepts empty or non-JSON success bodies. `fetch()` and `fetch_as()` still require the requested JSON shape. HTTP 204 can decode `()`; an empty HTTP 200 remains a JSON decoding error. `Rpc::into_raw()` is still available for raw protocol composition.

## Configured authentication

The default `anonymous_client` and `new_authenticated` constructors build a fallible default transport. Their `_with_client` variants reuse a caller's pool and policies. The authenticated stream shares that transport for REST, login, and every refresh.

```rust
use std::time::Duration;
use rp_supabase_client::{anonymous_client_with_client, new_authenticated_with_client};
use rp_supabase_client::rp_postgrest::reqwest;
use rp_supabase_client::rp_supabase_auth::{
    jwt_stream::SupabaseAuthConfig, types::LoginCredentials, url::Url,
};

fn configured() -> Result<(), Box<dyn std::error::Error>> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;
    let url = Url::parse("https://example.supabase.co/")?;
    let _anonymous = anonymous_client_with_client("public-key".into(), &url, http.clone())?;
    let config = SupabaseAuthConfig {
        api_key: "public-key".into(), url,
        max_reconnect_attempts: 5,
        reconnect_interval: Duration::from_secs(3),
    };
    let credentials = LoginCredentials::builder()
        .email("user@example.com".to_owned())
        .password("password".to_owned())
        .build();
    let _stream = new_authenticated_with_client(config, credentials, http)?;
    Ok(())
}
```

Each authenticated stream item is a `Result<(Postgrest, AccessTokenResponseSchema), SupabaseClientError>`. Consume it with `futures::StreamExt` to receive refreshed clients. API-key, JSON, and bearer headers coexist with caller default headers. Auth credentials apply per request rather than replacing shared bearer defaults. `SupabaseAuthConfig` has no HTTP-client field.

The crate reexports `rp_postgrest`, `Postgrest`, and `Error`. The default `client` feature includes authentication; `default-features = false` retains the generated-schema/query runtime without auth helpers. That is not a no-HTTP or `no_std` mode. See the raw crate's [TLS policy](../postgrest/README.md#install-and-configure) for transport feature configuration.

## Relationships, RLS, and errors

Single-relation `projection!` continues to support `alias: embed(RelationshipMarker, Child)` and its `inner` form. `alias: empty(RelationshipMarker)` selects a predicate-only embed. Generated direct, reverse, unique, composite, and nested FK relationships preserve source/target ownership. Selected handles compose with `.then(...)` only when the selected child and relation chain match. A shared DTO does not loosen these checks.

`embedded(handle, |child| { ... })` scopes filters to the selected relationship. `exists` and `not_exists` test related-row existence, including empty embeds. Embedded predicates lock the selection, so choose `.select(selection)` first. Root filters and one mutation remain available on non-paged locked queries.

To-one embeds decode as `Option<Child>` and to-many embeds as `Vec<Child>`. These types remain conservative with `inner`, because RLS and filters can hide rows. Ordinary child filters preserve parent rows; `inner` filters at the embed's parent level. For UPDATE and DELETE, constrain affected rows with root filters. Child filters shape returned representations. PostgREST 16.2 rejects embed-alias existence predicates on DELETE; the client reports the canonical server error and does not rewrite them into FK null checks. See [generated relationship contracts](../supabase-codegen/README.md#typed-relationships) for edge and naming rules.

Typed fetch, RPC, raw fetch, and count methods return the same flat `rp_postgrest::Error`. `error.postgrest_body()` exposes decoded code/message/details/hint; `postgrest_error()` exposes the canonical structured source. `status()`, `url()`, and `response_metadata()` retain observed HTTP metadata, including on successful-response decoding failures. HTTP 300 is an error for checked execution. Ambiguous embedding PGRST201 details can be a typed array of relationship descriptions, not only text. Malformed error-envelope bytes remain in the Decode source. See the [raw error guide](../postgrest/README.md#one-error-result).

## Migration from 0.8

- Regenerate Rust bindings with codegen 0.9. Snapshot version 2 remains supported.
- Replace `.select::<Dto>()` with `.select(named::<_, Dto>())`. Import `schema::named`. The old type-only method is removed.
- Prefer `select!(Row => { ... })` for local queries and use the returned selection's handles.
- Keep `projection!` for named/shared DTOs. Existing DTO-owned relationship constants still compose with local selections.

## Migration from 0.7

- Replace the old `postgrest` reexport with `rp_postgrest`, or use the direct `Postgrest`/`Error` exports. The owned dependency is rp-postgrest 3.0, with the normal `rp_postgrest` library name.
- Remove `PostgerstResponse`, `ResponseError`, `schema::QueryError`, and manual response-wrapper/nested-result decoding. Fetch now returns `Result<T, Error>` directly. Replace old execution-wrapper pattern matches with canonical `Error` handling and its getters. No old exports or aliases remain.
- Replace `query.into_raw()?` with `query.into_raw()`. Use `builder.fetch::<Vec<P>>().await?` for raw typed decoding instead of the removed raw-result/response helpers. Raw `execute` deliberately remains unchecked.
- Add `?` to direct `Postgrest::new`, client auth/header setters, and raw `.build()`. JSON serialization failures now live in the builder and emerge at build/execution.
- Implement custom projections as `Projection<R>` instead of using an associated `Relation`. Preserve `SELECT_LEN`, `write_selection`, and exact response-key decoding. Regenerate bindings with codegen 0.9. Custom embed paths must supply their `Source` relation.
- Use `schema::rpc::<Function>(client, &args).fetch().await?`; the generated `Returns` is inferred, and no relation-specific projection/single or separate response decode is required.
- Count preferences no longer force a one-row range. Add pagination explicitly or use typed `.count(Count)`/raw `.execute_count(Count)` for body-free totals.
- A paged typed read cannot become a write. Start a separate root-filtered mutation rather than treating read pagination as a mutation limit.
- Builders are not `Clone`; recreate queries from `Postgrest::clone()`. Pass literal resource names, not percent-encoded strings. Dot-only resources return an explicit configuration error.

The [raw 2.1 migration guide](../postgrest/README.md#migration-from-21) covers the remaining protocol changes.
