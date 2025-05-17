# Supabase Utils Examples

This directory contains example applications demonstrating various features of the Supabase Utils Rust library. Each example focuses on a specific functionality and shows how to use it in practice.

## Prerequisites

- Rust toolchain installed
- Supabase CLI installed
- A running Supabase instance (the examples will try to connect to a local instance at `./supabase`)

```bash
cd supabase
supabase start
```

## Running the Examples

All examples can be run using Cargo. For example:

```bash
cargo run --bin auth-example
```

## Available Examples

| Example | File | Description | Use Cases |
|---------|------|-------------|-----------|
| Auth Example | [`cargo run --bin auth-example`](./src/auth_example.rs) | Demonstrates Supabase authentication and authenticated API requests | - User authentication<br>- Making authenticated API calls<br>- Getting user information |
| Broadcast Example | [`cargo run --bin broadcast-example`](./src/broadcast_example.rs) | Shows how to use Supabase's broadcast feature for real-time messaging | - Real-time notifications<br>- Chat features<br>- Broadcasting messages to multiple clients |
| Presence Example | [`cargo run --bin presence-example`](./src/presence_example.rs) | Demonstrates presence tracking in channels | - Online status indicators<br>- Typing indicators<br>- User presence tracking |
| JWT Stream Example | [`cargo run --bin jwt-stream-example`](./src/jwt_stream_example.rs) | Shows JWT token management and refresh handling | - Session management<br>- Token refresh automation<br>- Maintaining authenticated sessions |
| Database Updates Example | [`cargo run --bin db-updates-example`](./src/db_updates_example.rs) | Demonstrates real-time database change listening | - Real-time data synchronization<br>- Live updates<br>- Database change notifications |
| Client Example | [`cargo run --bin client-example`](./src/client_example.rs) | Shows basic database operations with Supabase client | - Database queries<br>- CRUD operations<br>- Authenticated database requests |

## Common Features

All examples share some common features:
- Logging setup with `tracing`
- Configuration management
- Authentication handling
