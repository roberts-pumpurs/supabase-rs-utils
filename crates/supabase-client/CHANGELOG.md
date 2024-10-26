# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/roberts-pumpurs/supabase-rs-utils/releases/tag/rp-supabase-client-v0.1.0) - 2024-10-26

### Added

- created an auth api module
- re-export postgrest error
- created simple supabase client wrapper
- supabase request builders
- cleanup the execute method
- smart postgrest error handling
- authenticated user client will expose user struct
- query builder keep hold of table name
- wip exapmle for crud ops
- execute queries
- base for supabase client
- initial client wrapper

### Fixed

- clone internal vec

### Other

- ai generate readmes
- internal crate renaming
- fmt
- fmt
- postgrest errors extracted as a crate, dropped supabase-client
- fmt
- instrument the code
- export the error
- remove unwraps in auth code
- linter errors
- replace serde_json with simd_json
