# Changelog

## [3.0.0] - 2026-10-06

### Added

- Workspace-owned concrete PostgREST implementation with the `rp_postgrest` library name and Rust 1.85 minimum.
- Fallible default/configured client constructors. Supplied Reqwest transports retain connection pools, headers, timeout, proxy, and TLS policy.
- Checked execution that rejects all non-2xx statuses, including HTTP 300, plus public `fetch<T>` with centralized successful JSON decoding.
- One flat canonical error with structured-body getters and observed status, headers, and effective URL. Malformed error envelopes retain exact bytes. Successful body-read and decoding errors retain response metadata.
- JSON serialization helpers with deferred failures; safe `in_values` for literal list elements alongside raw `in_` grammar.
- Exact/planned/estimated count modes, data-plus-total fetch, HEAD read counts, minimal-return mutations, and affected-row counts. Missing or invalid totals report typed errors.
- HTTP 204 unit decoding for void responses. Empty HTTP 200 bodies remain JSON errors.

### Changed

- Breaking: replace the old `postgrest` library name with `rp_postgrest`; remove obsolete execution/query error aliases and public mutable request state.
- Breaking: client construction, client auth/header changes, and request build return errors instead of panicking or assuming valid configuration. `Builder` is not Clone.
- Breaking: resource names are literal SQL names encoded exactly once. Dot-only resources fail explicitly rather than addressing another endpoint.
- Breaking: count preferences no longer force Range 0-0. Caller pagination remains intact; use `execute_count` for body-free totals.
- Compose Prefer directives by key with last-value wins and preserve unrelated directives. Repeated order calls compose one effective order value while repeated filters retain their order.
- Enable rustls by default; no-default builds enable no TLS backend through this crate. HTTPS requires a TLS backend in the dependency graph.

The implementation adapts MIT/Apache-2.0 code from rp-postgrest 2.1.0. See the README for migration and raw grammar contracts.
