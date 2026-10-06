# rp-supabase-codegen

Generate Rust bindings for Supabase and PostgreSQL schemas in `build.rs`. No generator CLI is required.

The generator reads a versioned JSON snapshot or introspects PostgreSQL directly. It emits schema modules with table rows, insert and update payloads, enums, composites, and named-argument RPC bindings.

## Offline builds

Keep a schema snapshot in source control. Ordinary builds need no database or credentials.

```toml
[dependencies]
rp-supabase-client = "0.5"
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1", features = ["arbitrary_precision"] }
# Add these when your schema has UUID or temporal columns.
uuid = { version = "1", features = ["serde"] }
chrono = { version = "0.4", features = ["serde"] }

[build-dependencies]
rp-supabase-codegen = "0.5"
```

```rust
// build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    rp_supabase_codegen::Generator::new()
        .from_snapshot("schema.json")?
        .write_to_out_dir("database.rs")?;
    Ok(())
}
```

```rust
pub mod database {
    rp_supabase_client::include_schema!("database.rs");
}
```

The generator registers the snapshot with Cargo's change detection. It writes only the requested file and leaves identical output unchanged. Rust formatting uses `prettyplease`, not an external formatter.

See the [complete snapshot and executable example](../supabase-codegen-example). The snapshot format is the public `model::Snapshot` type. Version 1 rejects unknown fields and unsupported versions.

## Direct database introspection

Enable the `database` build-dependency feature. Database and TLS dependencies do not enter the application's runtime dependency graph.

```toml
[build-dependencies]
rp-supabase-codegen = { version = "0.5", features = ["database"] }
```

```rust
// build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=supabase/migrations");
    rp_supabase_codegen::Generator::new()
        .schemas(["public", "api"])
        .from_database_env("SUPABASE_CODEGEN_DATABASE_URL")?
        .write_to_out_dir("database.rs")?;
    Ok(())
}
```

Use a PostgreSQL connection string, not an anon key or service-role API key. Prefer a direct or session-pooler connection. Use a schema-owning role for complete metadata. Introspection reads catalogs in a read-only repeatable-read transaction. It does not read application rows or function bodies.

TLS verifies certificates and hostnames. The driver upgrades `sslmode=prefer` to `require`, preventing plaintext fallback. Use `sslmode=disable` only for a trusted local database. Connection errors do not print the connection string or raw server diagnostics.

Cargo cannot detect remote DDL. Track your migration directory, or add a refresh environment variable in your build script and change it after remote migrations. Do not assume every `cargo build` introspects again.

Live failures never fall back to cached metadata. Choose offline or live input explicitly.

### Creating a snapshot

Call the same library outside `build.rs` to refresh the committed snapshot:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    rp_supabase_codegen::Generator::new()
        .from_database_env("SUPABASE_CODEGEN_DATABASE_URL")?
        .write_snapshot("schema.json")?;
    Ok(())
}
```

Run this Rust code in a small host-side example or maintenance task. Do not write committed snapshots from an ordinary build script. `Bindings::snapshot()` also exposes typed metadata for your own tooling.

## Generated contracts

For `public.messages`, the generator emits:

- `public::tables::messages::Row`, with every readable field.
- `public::tables::messages::Insert`, with required fields and omittable default or nullable fields.
- `public::tables::messages::Update`, with omittable writable fields and `Default`.

Rows implement `schema::Relation`. The `schema::from::<Row>(client)` helper selects the correct schema and relation. It consumes the client, so cloning remains explicit. It returns the existing `rp_postgrest::Builder`.

```rust,ignore
use rp_supabase_client::{PostgerstResponse, schema};
use database::public::tables::messages::Row;

let response = schema::from::<Row>(client.clone())
    .select("*")
    .execute()
    .await?;
let rows = PostgerstResponse::<Vec<Row>>::new(response).json().await??;
```

Serialize an `Insert` or `Update` with `serde_json::to_string` before passing it to `.insert` or `.update`. The client retains its existing nested transport and PostgREST error results.

### Omission and null

`schema::Field<T>` has `Omit` and `Value(T)`. Nullable writes use `Field<Option<T>>`:

| Rust value | Request |
| --- | --- |
| `Field::Omit` | No key. |
| `Field::Value(None)` | Explicit JSON null. |
| `Field::Value(Some(value))` | Explicit value. |

Non-null writes use `Field<T>`, so they cannot send null. Generated serializers omit `Omit` fields. Serializing `Omit` without the containing-field skip attribute returns an error.

Generated expression columns and ALWAYS identity columns appear only in rows. BY DEFAULT identities remain optional on insert and writable on update. Domain defaults and not-null constraints participate in insert requirements.

Bulk inserts with different omitted keys require care. PostgREST's `Prefer: missing=default` controls missing-key defaults in bulk requests. Generated omission alone does not change server preferences.

### Types

Integer widths match PostgreSQL. UUID and temporal columns use `uuid` and `chrono`. Timestamps with time zones use `DateTime<FixedOffset>`. JSON columns use `serde_json::Value`.

Numeric columns use `serde_json::Number`. Enable `arbitrary_precision` when decoding outside the client. `PostgerstResponse::json` enables this support through the client's dependency and preserves numeric precision. Non-finite numeric values need a custom mapping because they are not ordinary JSON numbers.

Bytea, network, interval, range, geometric, and text-search columns use their JSON string representation. SQL bytea does not map to a JSON byte array.

`schema::Array<T>` preserves null elements and variable rank with `Elements(Vec<Option<T>>)` and `Nested(Vec<Array<T>>)`. Use another outer `Option` for a nullable column. PostgreSQL does not enforce declared array dimensions. For JSON-valued array elements, JSON itself does not distinguish nested SQL arrays from JSON arrays stored as elements.

Enums preserve exact database labels with Serde renames. Standalone and relation-row composites have nullable members. They remain distinct from a full table row's constraints. Domains preserve their qualified identity for overrides and otherwise resolve to their base type. Unknown types fail generation and require `type_override`.

### Functions

Named-object RPCs emit `public::functions::<name>::Args`, `Returns`, and `Function`. Overloads use deterministic numbered modules, such as `lookup_0` and `lookup_1`, while retaining the original RPC name.

Required arguments use `Option<T>` because PostgreSQL functions can accept null. Default arguments use `Field<Option<T>>`, distinguishing omission from null. Scalar results are nullable. Set results use vectors. Non-set composite and OUT results use a single struct. OUT and TABLE fields have a generated `Record` type, including singleton OUT and INOUT results.

```rust,ignore
let request = schema::rpc::<database::public::functions::echo_message::Function>(
    client.clone(),
    &database::public::functions::echo_message::Args {
        message: Some("hello".to_owned()),
    },
)?;
```

PostgREST cannot distinguish every SQL overload by its JSON argument names. Generated overload types do not change that server limitation.

## Custom derives, attributes, and preludes

```rust
let generator = rp_supabase_codegen::Generator::new()
    .prelude("use ::std::string::String as DatabaseText;")
    .type_override("pg_catalog.text", "DatabaseText")
    .derive("PartialEq")
    .attribute("#[allow(dead_code)]")
    .type_attribute(
        "public.tables.messages.Insert",
        "#[derive(::typed_builder::TypedBuilder)]",
    );
```

Global derives and attributes apply to generated data structs and enums. Per-type attributes apply only to their named target. Use per-type attributes for macros that cannot operate on enums. The executable example compiles and uses `TypedBuilder` on its insert payload.

Target paths use generated Rust names. Keyword modules use raw spelling, such as `public.tables.r#type.Insert`. Record-result targets end in `.Record`. Unknown targets and invalid Rust syntax fail before output is written.

Prelude items appear at the generated root. Nested modules import their parent, so custom types remain available. `schema::prelude` exports runtime helpers and `include_schema!`. Use the generated schema modules explicitly to avoid cross-schema name collisions.

`runtime_path("::renamed_client::schema")` supports a renamed client dependency. Generated Serde derives and helper attributes require the normal `serde` dependency name.

## Scope and safety

View and materialized-view bindings are read-only, even when PostgreSQL permits writes to a view. Their row fields are conservatively nullable.

The named-object RPC generator excludes unnamed input arguments, trigger functions, polymorphic pseudotypes, and dynamic records without named output fields. Those need different request bodies or explicit application-specific contracts.

Bindings do not validate CHECK constraints, enforce RLS, grant permissions, or type-check arbitrary PostgREST projections and joins. Decode custom projections into your own structs. Additional schema modules can contain referenced types even when their schema is not exposed through PostgREST.

## Verification example

```sh
cargo run -p rp-supabase-codegen-example
```

This compiles a real `build.rs` consumer and exercises generated serialization and decoding offline. For live verification, apply its `smoke.sql` to a disposable database, expose `public` through PostgREST, and set `SUPABASE_CODEGEN_DATABASE_URL` and `SUPABASE_CODEGEN_API_URL`.

The [research note](../../docs/schema-generation-research.md) records source comparisons and design decisions.
