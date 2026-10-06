# Schema bindings example

`build.rs` generates bindings from `schema.json` by default. It adds a prelude type alias, a text mapping, `PartialEq`, and a targeted `TypedBuilder` derive. `src/main.rs` includes the output with `include_schema!`.

Run without a database:

```sh
cargo run -p rp-supabase-codegen-example --offline
```

Cargo's offline flag requires cached dependencies. The executable checks omitted identity/default
fields, explicit null, named projection decoding, exact numeric values, and nested relationship shapes.

The example uses `projection!` to declare `Message` without repeating schema field types.
Typed column markers check filters. Generated payloads check insert and update fields.
`fetch()` infers the projected response type and checks HTTP errors before decoding JSON.

`src/relationship_projections.rs` contains mixed scalar, nested, inner, and predicate-only projections.
Relationship handles check child columns and selected projection ownership.
Compile-fail consumers cover wrong edges, wrong child types, duplicate aliases, and selection changes after embedded predicates.


Run consumer behavior and compiler rejection checks:

```sh
cargo test -p rp-supabase-codegen-example --offline
```

## Live scenario

Apply `smoke.sql` to a disposable PostgreSQL database. It creates message and relationship fixtures and grants access for verification. Do not apply it to the repository's existing Supabase database or a production project.

Expose `public` through PostgREST. Set these environment variables:

- `SUPABASE_CODEGEN_DATABASE_URL`, a database connection string used only by `build.rs`. Use verified TLS remotely. Add `sslmode=disable` for a trusted local database.
- `SUPABASE_CODEGEN_API_URL`, the PostgREST base URL. Use `https://PROJECT.supabase.co/rest/v1` for Supabase.
- `SUPABASE_CODEGEN_API_KEY`, optional Supabase API key.
- `SUPABASE_CODEGEN_ACCESS_TOKEN`, optional bearer access token.

Then run the same Cargo command. Live introspection never falls back to the snapshot after an error.
The executable first checks direct and reverse FK relationships, billing/shipping ambiguity,
PK/UNIQUE reverse to-one shapes, ordered composite FKs, nested filters, and empty-embed existence predicates.
It compares left and inner behavior. Run-specific cleanup removes relationship fixtures after recoverable errors.
Projected insert, update, and delete also run after scoped child filters lock the selection.

It then inserts a projected message, reads it with a typed filter, updates its nullable field,
reads the view, calls the RPC, and deletes the row. It checks numeric precision through typed fetches.

The scenario uses a 30-second timeout. A panic or timeout can leave inserted rows. Use a disposable database.

Remote DDL does not trigger Cargo change detection. Change the tracked database environment variable, or register your migrations in `build.rs`, to request another introspection. Remove the database variable to return to the committed snapshot.
