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

The client re-exports `postgrest::Postgrest`. Typed relation queries now retain generated column and projection types. They reuse the native builder and checked transport internally. `into_raw()` drops typed guarantees for expressions outside this interface. RPCs still return the native builder.

Follow [Cargo's build-script contract](https://doc.rust-lang.org/cargo/reference/build-scripts.html): write generated source into `OUT_DIR`, register input paths and environment variables, and never rewrite committed snapshots during ordinary builds.

## Deliberate limits

View bindings are read-only, including views PostgreSQL marks writable. Their write obligations can depend on rules and triggers. Unnamed-argument RPCs need different request bodies and are not named-object bindings. Polymorphic SQL types need explicit mappings. Bindings do not validate CHECK constraints or enforce RLS.

Bulk inserts can change missing-property behavior. Review [PostgREST's missing/default preference](https://docs.postgrest.org/en/stable/references/api/preferences.html#missing) before sending rows with different omitted keys. Omission in a Rust struct alone does not force defaults for every bulk request.

## Implementation verification

The implementation compiles and runs on Rust 1.85.1. The workspace test command passes 84 tests. Workspace Clippy passes with all features, all targets, and warnings denied. The workspace formatting check passes.

The offline example uses a real `build.rs`, generated output, custom type imports, and a targeted `TypedBuilder` derive. A separate compiled consumer verifies primitive-name collisions, prelude alias collisions, composite and singleton OUT/INOUT result shapes, and percent-encoded relation and RPC names.

Live checks use PostgreSQL 17.11 and PostgREST 12.2.12. They exercise CRUD, views, scalar and composite RPCs, TABLE results, input defaults interspersed with OUT arguments, cross-schema enums, recursive domain defaults, generated columns, nullable multidimensional arrays, and exact numeric decoding. Snapshot export reloads as identical typed metadata.

Introspection also succeeds for a foreign table whose backing file does not exist, proving that the extraction does not need to read that table. A connection without explicit plaintext permission fails against the TLS-disabled local database. Failed authentication diagnostics omit the supplied password.

The workspace documentation build succeeds. It retains an existing unresolved `SupabaseAuth` link in the auth crate.

## Typed relationship selections

Implemented in 0.7.0 on 2026-10-06. Direct FK selections extend the existing typed query runtime.

Extend the existing `projection!` macro rather than add a second fluent selection builder.
Named projections keep ordinary field access. A fluent builder would need tuples, typed lookup
records, or another macro to create equivalent named fields.

Implemented syntax:

```rust,ignore
projection! {
    struct AddressSummary for database::public::tables::addresses { id, label }
}
projection! {
    struct OrderSummary for database::public::tables::orders {
        id,
        billing_address: embed(
            database::public::tables::orders::relationships::orders_billing,
            AddressSummary,
        ),
        shipping_address: embed(
            database::public::tables::orders::relationships::orders_shipping,
            AddressSummary,
            inner,
        ),
    }
}
```

Each generated relationship marker records its source relation, target relation, direction,
cardinality, and exact PostgREST resource and constraint hint. The macro checks both relation
identities. Field names become response aliases. Distinct markers select billing and shipping
foreign keys without string hints at call sites.

The billing field renders `billing_address:addresses!orders_billing(id,label)`.
The shipping field renders `shipping_address:addresses!orders_shipping!inner(id,label)`.
The marker names and hints come from FK constraint metadata.
[PostgREST requires FK hints for ambiguous paths](https://docs.postgrest.org/en/stable/references/api/resource_embedding.html#multiple-many-to-one).

### Metadata acquisition

Snapshot version 2 records qualified relation references, ordered foreign-key column pairs,
constraint names, primary keys, unique constraints, and partition identity.
[PostgreSQL's constraint catalog](https://www.postgresql.org/docs/current/catalog-pg-constraint.html)
supplies these facts in the existing read-only repeatable-read transaction.
Generation derives inverse edges rather than store two copies of each relationship.

Regenerate version 1 snapshots. Required facts do not default to an empty relationship graph.
Qualified FK targets survive even when their schema is not selected.
Generation selection does not prove which schemas PostgREST exposes.
Referenced types can add dependency schemas, but cannot add unrelated table or RPC endpoints.
Direct markers cover both directions between nonpartition base tables in the same selected schema.
Reverse edges are to-one when their FK column set exactly matches a recorded primary key or unique constraint, as
[PostgREST documents](https://docs.postgrest.org/en/stable/references/api/resource_embedding.html#one-to-one-relationships).
Use nested junction selections before adding inferred many-to-many shortcuts.
Two foreign keys alone do not establish a PostgREST junction. Its keys also matter.

### Cardinality and filters

Decode to-one embeds as `Option<ChildProjection>` and to-many embeds as `Vec<ChildProjection>`.
Use these conservative types for inner selections too. A non-null foreign key does not prove
that a filtered or access-controlled child appears in the response. Every selected relationship
key must still exist. Distinguish a present null from a missing key.

[Embedded filters normally preserve parent rows](https://docs.postgrest.org/en/stable/references/api/resource_embedding.html#top-level-filtering).
`inner` explicitly changes parent selection. It does not make nullable child columns or unrelated
nested left embeds non-null.

Generate projection-owned embedding handles for scoped child filters.
For example, `OrderSummary::billing_address` identifies the selected alias and child projection.
Compose these handles for nested filter paths. Check child columns against the relationship target.
After adding an embedded filter, lock the selected projection. Replacing it could leave filters
that refer to an alias no longer selected.

Represent predicate-only empty embeds separately. They add selection and filter syntax but no
decoded response field. Relationship existence and anti-existence predicates are not scalar null tests.

### Unsupported relationships

Do not infer all view, recursive, or computed relationships from foreign keys.
[View inference depends on selected FK columns and view definitions](https://docs.postgrest.org/en/stable/references/api/resource_embedding.html#foreign-key-joins-on-views).
[Recursive direction disambiguation uses computed relationships](https://docs.postgrest.org/en/stable/references/api/resource_embedding.html#recursive-relationships).
[Computed relationships use relation argument/result types and ROWS estimates](https://docs.postgrest.org/en/stable/references/api/resource_embedding.html#computed-relationships).
The current named-argument RPC model does not capture that contract.

Keep network errors, authorization, schema-cache drift, and response-shape failures explicit.
Typed selections check captured relation identities and result shapes. They do not prove current
server availability or row visibility.

Live verification uses PostgreSQL 17.11 and PostgREST 16.2 with a non-superuser API role.
It covers ambiguous FK aliases, reverse uniqueness, ordered composite joins, nested child filters,
left versus inner behavior, empty embeds, existence predicates, and projected CRUD/view/RPC calls.
A separate RLS check hides a non-null FK target and decodes `None` without removing its parent.
Catalog checks preserve external target identities and type dependencies, and omit inherited partition FKs.
Grammar checks cover reserved resource, hint, alias, and column names with literal punctuation and Unicode values.
Projected insert, update, and delete also succeed after scoped child filters lock selection.
PostgREST 16.2 rejects embed-alias existence predicates as DELETE row conditions.
Use root column predicates for mutation conditions. The runtime preserves native execution errors rather than rewriting FK predicates.
