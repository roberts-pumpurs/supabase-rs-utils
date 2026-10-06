# Changelog

## [Unreleased]

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
