# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/roberts-pumpurs/supabase-rs-utils/compare/rp-supabase-realtime-v0.2.3...rp-supabase-realtime-v0.3.0) - 2025-05-18

### Added

- add presence ([#19](https://github.com/roberts-pumpurs/supabase-rs-utils/pull/19))

## [0.2.3](https://github.com/roberts-pumpurs/supabase-rs-utils/compare/rp-supabase-realtime-v0.2.2...rp-supabase-realtime-v0.2.3) - 2025-03-23

### Fixed

- realtime will handle fatal errors

## [0.2.1](https://github.com/roberts-pumpurs/supabase-rs-utils/compare/rp-supabase-realtime-v0.2.0...rp-supabase-realtime-v0.2.1) - 2025-01-04

### Added

- add extra null case handlings for realtime messages

## [0.2.0](https://github.com/roberts-pumpurs/supabase-rs-utils/compare/rp-supabase-realtime-v0.1.2...rp-supabase-realtime-v0.2.0) - 2024-11-24

### Added

- presence typings
- add broadcast

## [0.1.2](https://github.com/roberts-pumpurs/supabase-rs-utils/compare/rp-supabase-realtime-v0.1.1...rp-supabase-realtime-v0.1.2) - 2024-10-26

### Other

- release v0.1.1 ([#12](https://github.com/roberts-pumpurs/supabase-rs-utils/pull/12))

## [0.1.1](https://github.com/roberts-pumpurs/supabase-rs-utils/releases/tag/rp-supabase-realtime-v0.1.1) - 2024-10-26

### Added

- created an auth api module
- created simple supabase client wrapper
- can deserialize postgres changes packets
- realtime can receive proper updates
- add phx_close event
- supabase realtime will decode the incoming json
- add hb capabilities
- update deps
- query builder keep hold of table name
- base for supabase client
- steriliaztion tests ([#3](https://github.com/roberts-pumpurs/supabase-rs-utils/pull/3))
- configurable params for stream
- added error phx_reply
- updated the realtime connectoin example
- async ws handler will call the waker of LiveRealtimeConnection
- fir realtime connection implementaiton
- improved url resolvement in supabase-realtime
- example `fastwebsockets` implementation

### Fixed

- broken example

### Other

- release
- init changelog
- update cargo.toml
- add changelog files
- readme update
- ai generate readmes
- internal crate renaming
- fmt
- remove unused deps
- examples split into own crate
- created client / server architecture for realtime data
- simplify jwt refresh
- replace serde_json with simd_json
- docs & renaming
- send phx join message
- fix broken tests
- split ws connection handling into modules

## [0.1.1](https://github.com/roberts-pumpurs/supabase-auth-rs/releases/tag/rp-supabase-realtime-v0.1.1) - 2024-10-26

### Added

- created an auth api module
- created simple supabase client wrapper
- can deserialize postgres changes packets
- realtime can receive proper updates
- add phx_close event
- supabase realtime will decode the incoming json
- add hb capabilities
- update deps
- query builder keep hold of table name
- base for supabase client
- steriliaztion tests ([#3](https://github.com/roberts-pumpurs/supabase-auth-rs/pull/3))
- configurable params for stream
- added error phx_reply
- updated the realtime connectoin example
- async ws handler will call the waker of LiveRealtimeConnection
- fir realtime connection implementaiton
- improved url resolvement in supabase-realtime
- example `fastwebsockets` implementation

### Fixed

- broken example

### Other

- init changelog
- update cargo.toml
- add changelog files
- readme update
- ai generate readmes
- internal crate renaming
- fmt
- remove unused deps
- examples split into own crate
- created client / server architecture for realtime data
- simplify jwt refresh
- replace serde_json with simd_json
- docs & renaming
- send phx join message
- fix broken tests
- split ws connection handling into modules

## [0.1.1](https://github.com/roberts-pumpurs/supabase-auth-rs/compare/rp-supabase-realtime-v0.1.0...rp-supabase-realtime-v0.1.1) - 2024-10-26

### Added

- created an auth api module
- created simple supabase client wrapper
- can deserialize postgres changes packets
- realtime can receive proper updates
- add phx_close event
- supabase realtime will decode the incoming json
- add hb capabilities
- update deps
- query builder keep hold of table name
- base for supabase client
- steriliaztion tests ([#3](https://github.com/roberts-pumpurs/supabase-auth-rs/pull/3))
- configurable params for stream
- added error phx_reply
- updated the realtime connectoin example
- async ws handler will call the waker of LiveRealtimeConnection
- fir realtime connection implementaiton
- improved url resolvement in supabase-realtime
- example `fastwebsockets` implementation

### Fixed

- broken example

### Other

- update cargo.toml
- add changelog files
- readme update
- ai generate readmes
- internal crate renaming
- fmt
- remove unused deps
- examples split into own crate
- created client / server architecture for realtime data
- simplify jwt refresh
- replace serde_json with simd_json
- docs & renaming
- send phx join message
- fix broken tests
- split ws connection handling into modules
