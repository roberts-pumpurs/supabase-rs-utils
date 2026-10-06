# Schema generation research

Research date: 2026-10-05. Three subagents inspected generator alternatives, Supabase metadata extraction, and this workspace's client.

## Decision

Implement `rp-supabase-codegen` as a build dependency. Read PostgreSQL catalogs directly or load a versioned snapshot. Generate one Rust file under `OUT_DIR`. Keep the existing `rp-postgrest` query builder and response errors.

Committed snapshots make ordinary builds reproducible. Live introspection is explicit. A failed connection must fail the build, not select a stale snapshot. Cargo cannot detect remote DDL, so live builds must track migrations or an explicit refresh input.

## Existing generator UX

[supabase-rust-gen 0.1.0](https://docs.rs/crate/supabase-rust-gen/0.1.0) has a CLI and a callable library. The CLI accepts URL, anon key, output path, query generation, and RLS generation options. It fetches the PostgREST OpenAPI document.

Its [configuration](https://docs.rs/crate/supabase-rust-gen/0.1.0/source/src/generate/mod.rs) has three fields: `with_rls`, `with_queries`, and `output_path`. It has no prelude, custom derive, custom attribute, or injected type mapper. Calling it from `build.rs` is possible, but callers must supply Cargo change tracking and an async runtime for fetching.

The [parser](https://docs.rs/crate/supabase-rust-gen/0.1.0/source/src/schema/parse.rs) discards SQL type details. Its [emitter](https://docs.rs/crate/supabase-rust-gen/0.1.0/source/src/generate/structs.rs) generates one row model, not separate row, insert, and update models. The [mapper](https://docs.rs/crate/supabase-rust-gen/0.1.0/source/src/types/mapping.rs) uses `f64` for numeric and strings for unknown types. These choices cannot preserve PostgreSQL mutation semantics or decimal precision.

The advertised mappings do not prove the fetch-to-code path. [PostgREST's OpenAPI emitter](https://github.com/PostgREST/postgrest/blob/v14.0/src/PostgREST/Response/OpenAPI.hs) uses PostgreSQL format strings and omits JSON `type` for JSON columns. The inspected parser does not preserve those cases. Its [RLS helpers](https://github.com/Dominion77/supabase-rust-gen/blob/main/src/generate/rls.rs) return `Ok(true)` without checking policies. We do not copy them.

The crate declares MIT OR Apache-2.0. This implementation does not copy its source.

## Other Rust tools

| Tool | Useful precedent | Why it is not the implementation |
| --- | --- | --- |
| [Typify](https://docs.rs/typify/0.8.0/typify/) | [Real build.rs example](https://github.com/oxidecomputer/typify/blob/main/example-build/build.rs), custom derives, attributes, and type replacement | JSON Schema lacks PostgreSQL defaults, identity, domain, and RPC semantics. |
| [SeaORM codegen](https://www.sea-ql.org/SeaORM/docs/generate-entity/sea-orm-cli/) | Generated preludes and extra model attributes | Primarily generates ORM entities. The inspected 2.0.4 release requires Rust 1.88. |
| [SQLx-gen](https://docs.rs/crate/sqlx-gen/0.5.9) | Direct introspection, qualified custom types, derive and type overrides | Generated contracts target SQLx, not PostgREST JSON. |
| [Cornucopia](https://docs.rs/cornucopia/1.0.1/cornucopia/) | Consuming configuration builder and per-type customization | Generates prepared-query bindings and a separate crate, not REST schema models. |

## Supabase's metadata source

Current [Supabase CLI generation](https://github.com/supabase/cli/blob/develop/apps/cli/src/commands/gen/types/types.generator.layer.ts) opens a database connection and calls the first-party introspector. [postgres-meta](https://github.com/supabase/postgres-meta/blob/master/src/lib/generators.ts) delegates to the same package.

The [first-party introspection package](https://github.com/supabase/sdk/tree/main/packages/postgrest-typegen/src/introspection) reads catalog metadata for tables, views, columns, keys, functions, and types. Its [published 0.4.0 package](https://unpkg.com/@supabase/postgrest-typegen@0.4.0/package.json) declares MIT. Branch links above describe the inspected upstream state, not an immutable release contract. Our SQL is independently written against PostgreSQL catalogs.

A Rust PostgreSQL driver can perform these reads without the Supabase CLI, Node, or a metadata server. It needs a database connection string, not an anon or service-role API key. Use a direct or session-pooler connection from the [Supabase connection guide](https://supabase.com/docs/guides/database/connecting-to-postgres). Remote connections need TLS and certificate verification. Keep passwords out of generated files, snapshots, and Cargo logs.

Use one read-only repeatable-read transaction. Parameterize schema names. Read schema metadata only, not application rows or function bodies. Use a migration or schema-owning role for complete metadata. A generated binding does not prove runtime permissions or RLS access.

## Rust contracts

[PostgreSQL attributes](https://www.postgresql.org/docs/current/catalog-pg-attribute.html) distinguish nullability, defaults, identity, and generated expressions. A row includes readable columns. An insert requires writable non-null columns without defaults. An update can omit every writable column. Generated columns and ALWAYS identity columns have no normal write field.

An optional nullable write needs three states: omission, SQL null, and a value. `Field<Option<T>>` represents them without another allocation. `Field<T>` prevents explicit null for non-null writes. Do not use a single `Option<T>` for both omission and null.

[PostgreSQL arrays](https://www.postgresql.org/docs/current/arrays.html) can contain null elements and have variable rank. Array bindings must preserve nested JSON shape rather than assume the declared dimensions are enforced.

[Domains](https://www.postgresql.org/docs/current/catalog-pg-type.html) can supply defaults and not-null constraints. Resolve their base types recursively while preserving qualified domain names for custom Rust mappings. Referenced enums and composites can belong to schemas outside the exposed selection.

[Function metadata](https://www.postgresql.org/docs/current/catalog-pg-proc.html) has separate input and output modes. Defaults apply to the last input arguments, not the last positions of the combined argument list. SQL arguments and scalar returns can be null. [PostgREST's response encoder](https://github.com/PostgREST/postgrest/blob/v12.2.12/src/PostgREST/Query/SqlFragment.hs) returns a JSON object for non-set composite and OUT results, and an array for set results. A live PostgreSQL 17 and PostgREST 12.2.12 check confirms the non-set composite object. Classify output modes as well as the declared return type.

Unknown SQL types must require an explicit mapping. JSON columns intentionally use `serde_json::Value`. Numeric values need exact JSON-number handling, not `f64` or a decimal serializer that emits strings.

## Customization and integration

Use global derives and attributes for shared behavior. Use a generated-type path for attributes that apply only to one struct. This lets consumers use existing derive and attribute macros without a new procedural-macro framework. Validate Rust syntax and reject unknown customization targets.

Generate schema modules rather than flatten names across schemas. Preserve SQL spelling with Serde renames. Reject normalized-name collisions. Allow a prelude to supply imports for custom mappings, and supply an `include_schema!` macro for `OUT_DIR` inclusion.

The client returns `rp_postgrest::Postgrest` today. Its builder already implements schema profiles, filters, RPC requests, and HTTP transport. Typed relation and function selection should return that builder. Arbitrary projections and joins still need caller-defined response types.

Follow [Cargo's build-script contract](https://doc.rust-lang.org/cargo/reference/build-scripts.html): write generated source into `OUT_DIR`, register input paths and environment variables, and never rewrite committed snapshots during ordinary builds.

## Deliberate limits

View bindings are read-only, including views PostgreSQL marks writable. Their write obligations can depend on rules and triggers. Unnamed-argument RPCs need different request bodies and are not named-object bindings. Polymorphic SQL types need explicit mappings. Bindings do not validate CHECK constraints or enforce RLS.

Bulk inserts can change missing-property behavior. Review [PostgREST's missing/default preference](https://docs.postgrest.org/en/stable/references/api/preferences.html#missing) before sending rows with different omitted keys. Omission in a Rust struct alone does not force defaults for every bulk request.

## Implementation verification

The implementation compiles and runs on Rust 1.85.1. The workspace test command passes 53 tests. Workspace Clippy passes with all features, all targets, and warnings denied.

The offline example uses a real `build.rs`, generated output, custom type imports, and a targeted `TypedBuilder` derive. A separate compiled consumer verifies primitive-name collisions, prelude alias collisions, composite and singleton OUT/INOUT result shapes, and percent-encoded relation and RPC names.

Live checks use PostgreSQL 17.11 and PostgREST 12.2.12. They exercise CRUD, views, scalar and composite RPCs, TABLE results, input defaults interspersed with OUT arguments, cross-schema enums, recursive domain defaults, generated columns, nullable multidimensional arrays, and exact numeric decoding. Snapshot export reloads as identical typed metadata.

Introspection also succeeds for a foreign table whose backing file does not exist, proving that the extraction does not need to read that table. A connection without explicit plaintext permission fails against the TLS-disabled local database. Failed authentication diagnostics omit the supplied password.

The workspace documentation build succeeds. It retains an existing unresolved `SupabaseAuth` link in the auth crate.
