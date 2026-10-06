# Schema bindings example

`build.rs` generates bindings from `schema.json` by default. It adds a prelude type alias, a text mapping, `PartialEq`, and a targeted `TypedBuilder` derive. `src/main.rs` includes the output with `include_schema!`.

Run without a database:

```sh
cargo run -p rp-supabase-codegen-example --offline
```

Cargo's offline flag also requires dependencies to be cached. The executable checks omitted identity/default fields, explicit null, enum labels, nested arrays, and exact numeric decoding.

## Live scenario

Apply `smoke.sql` to a disposable PostgreSQL database. It creates its own `messages` table and grants access for verification. Do not apply it to the repository's existing Supabase database or a production project.

Expose `public` through PostgREST. Set these environment variables:

- `SUPABASE_CODEGEN_DATABASE_URL`, a database connection string used only by `build.rs`. Use verified TLS remotely. Add `sslmode=disable` for a trusted local database.
- `SUPABASE_CODEGEN_API_URL`, the PostgREST base URL. Use `https://PROJECT.supabase.co/rest/v1` for Supabase.
- `SUPABASE_CODEGEN_API_KEY`, optional Supabase API key.
- `SUPABASE_CODEGEN_ACCESS_TOKEN`, optional bearer access token.

Then run the same Cargo command. Live introspection never falls back to the snapshot after an error. The executable inserts a row, reads it, updates its nullable field, reads the view, calls the RPC, and deletes the row. It verifies numeric precision through the client response decoder.

The scenario uses a 30-second timeout. A panic or timeout can leave its inserted row. Use a disposable database.

Remote DDL does not trigger Cargo change detection. Change the tracked database environment variable, or register your migrations in `build.rs`, to request another introspection. Remove the database variable to return to the committed snapshot.
