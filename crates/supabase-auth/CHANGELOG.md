# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/roberts-pumpurs/supabase-rs-utils/releases/tag/rp-supabase-auth-v0.1.0) - 2024-10-26

### Added

- created an auth api module
- can deserialize postgres changes packets
- realtime can receive proper updates
- authenticated user client will expose user struct
- initial client wrapper
- configurable params for stream
- added error phx_reply
- fir realtime connection implementaiton
- add api key to the login headers
- fix parsing of jwt expiratoin
- supabase mock utils
- simple jwt flow
- wip jwt expiry tracker
- wip token refreshing template

### Fixed

- invalid refresh token path
- make AuthResponse struct fields public
- make `TokenBody` params public
- expose the `redact` crate
- re-export `url` lib
- refresh token is not a jwt

### Other

- readme update
- ai generate readmes
- internal crate renaming
- fmt
- remove unused deps
- examples split into own crate
- created client / server architecture for realtime data
- cargo xtask fmt
- fix broken tests
- simplify jwt refresh
- postgrest errors extracted as a crate, dropped supabase-client
- use simd_json
- replace serde_json with simd_json
- fix broken tests
- can build reqwest Client form the JWT response
- add extra derives to supabase login payload
- use `std::task::ready!()`
- add tests for jwt token refresh
- formatting
- batman
