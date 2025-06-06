# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/roberts-pumpurs/supabase-rs-utils/compare/rp-supabase-client-v0.2.3...rp-supabase-client-v0.3.0) - 2025-05-18

### Added

- add presence ([#19](https://github.com/roberts-pumpurs/supabase-rs-utils/pull/19))

## [0.2.3](https://github.com/roberts-pumpurs/supabase-rs-utils/compare/rp-supabase-client-v0.2.2...rp-supabase-client-v0.2.3) - 2025-03-23

### Other

- update Cargo.toml dependencies

## [0.1.2](https://github.com/roberts-pumpurs/supabase-rs-utils/compare/rp-supabase-client-v0.1.1...rp-supabase-client-v0.1.2) - 2024-10-26

### Other

- release v0.1.1 ([#12](https://github.com/roberts-pumpurs/supabase-rs-utils/pull/12))

## [0.1.1](https://github.com/roberts-pumpurs/supabase-rs-utils/releases/tag/rp-supabase-client-v0.1.1) - 2024-10-26

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

- release
- init changelog
- add changelog files
- get rid of a git dep
- readme update
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

## [0.1.1](https://github.com/roberts-pumpurs/supabase-auth-rs/releases/tag/rp-supabase-client-v0.1.1) - 2024-10-26

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

- init changelog
- add changelog files
- get rid of a git dep
- readme update
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

## [0.1.1](https://github.com/roberts-pumpurs/supabase-auth-rs/compare/rp-supabase-client-v0.1.0...rp-supabase-client-v0.1.1) - 2024-10-26

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

- add changelog files
- get rid of a git dep
- readme update
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
