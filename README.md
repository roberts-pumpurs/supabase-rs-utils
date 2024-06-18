# Supabase Rust utilities

Simple utilities for interacting with Supabase & their services from Rust.
I'm adding methods on use-case basis of what I need, rather than with the goal of creating full featured-client for every API.

## Supabase Auth

Supported methods:

- [POST /token](https://github.com/supabase/auth/blob/master/README.md#post-token)

The client will also automatically fetch a new auth token using the refresh token.
