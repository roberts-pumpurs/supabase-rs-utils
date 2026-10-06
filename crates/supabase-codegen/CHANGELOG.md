# Changelog

## [Unreleased]

- Respect non-table snapshot nullability, infer safe single-table view projections with a SQL parser, and support checked `not_null` builder and SQL comment contracts. Snapshot format is now version 3.

## [0.9.3](https://github.com/roberts-pumpurs/supabase-rs-utils/compare/rp-supabase-codegen-v0.9.2...rp-supabase-codegen-v0.9.3) - 2026-10-06

### Other

- *(schema)* share column contracts and projection compilation

### Changed

- Resolve relation column nullability and write obligations once for row fields, column markers,
  insert/update payloads, and automatic `Default` eligibility. Generated contracts and snapshot
  version 2 remain unchanged.

## [0.9.2](https://github.com/roberts-pumpurs/supabase-rs-utils/compare/rp-supabase-codegen-v0.9.1...rp-supabase-codegen-v0.9.2) - 2026-10-06

### Changed

- Align the version with client 0.9.2. Generated bindings are unchanged.

## [0.9.1](https://github.com/roberts-pumpurs/supabase-rs-utils/compare/rp-supabase-codegen-v0.9.0...rp-supabase-codegen-v0.9.1) - 2026-10-06

### Fixed

- Require PostgreSQL client 0.19.14 with patched protocol decoding and bounded SCRAM authentication.
- Link the package API documentation.
- Publish committed source through release-plz. Version 0.9.0 archives retain their original dirty-tree provenance.

## [0.9.0] - 2026-10-06

### Added

- Direct finite column and FK lookup implementations for query-first selections on exact schema-qualified rows.
- Lossless character-type names from the actual emitted Rust identifiers, preserving original SQL keys and FK hints.

### Changed

- Regenerate Rust output for client 0.9. Snapshot version 2 and existing relationship capability limits remain unchanged.

## [0.8.0] - 2026-10-06

### Changed

- **Breaking:** generated rows implement relation-parameterized `Projection<Row>`. Regenerate Rust bindings with codegen 0.8 for runtime 0.8; snapshots remain version 2.
- Emit `JsonColumn` markers for JSON and JSONB columns, including domains over those types.
- Update usage for shared projection DTOs, pure typed query pairs, typed pagination and counts, minimal writes, and raw checked DTO decoding.
- Document inferred typed RPC returns, HTTP 204 unit responses, canonical REST errors, and exact numeric decoding.

## [0.7.0] - 2026-10-06

### Added

- Generated column markers, typed relation query entry points, row projections, nullable
  predicates, and base-table write contracts.
- Generated enum display implementations that preserve database labels for typed filters.
- Forward and reverse FK markers with exact constraint hints and PK/UNIQUE-derived reverse cardinality.
- Ordered FK, key, qualified target, and partition metadata acquired from PostgreSQL catalogs.

### Changed

- **Breaking:** snapshots use version 2. Regenerate version 1 snapshots from PostgreSQL.
- Keep dependency-schema types separate from explicitly selected table and RPC endpoints.

## [0.6.0] - 2026-10-06

### Changed

- Document schema-only runtime use through `rp-supabase-client` with default features disabled.

## [0.5.0] - 2026-10-06

### Added

- Build-script-first Rust schema generation from versioned snapshots or direct PostgreSQL catalog introspection.
- Row, insert, update, enum, composite, domain, array, and named-argument RPC contracts.
- Prelude imports, custom derives, global and per-type macro attributes, SQL type overrides, and runtime path configuration.
- Optional verified-TLS database acquisition, Cargo input tracking, snapshot export, and deterministic `OUT_DIR` output.
- An executable offline example and a finite live PostgREST verification scenario.
