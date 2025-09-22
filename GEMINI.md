# Geni - Database Migration Tool

## Project Overview

Geni is a standalone database migration tool written in Rust. It helps developers manage database schema changes across different database systems. Geni is inspired by `dbmate` but adds support for LibSQL.

**Key Technologies:**

*   **Language:** Rust
*   **Database Drivers:**
    *   `sqlx` (for Postgres, MariaDB, MySQL)
    *   `libsql-client-rs` (for SQLite and LibSQL)
*   **CLI Framework:** `clap`
*   **Async Runtime:** `tokio`

**Architecture:**

The project is structured as a Rust workspace with a library (`geni`) and a binary (`geni`). The library contains the core logic for database migrations, while the binary provides a command-line interface.

## Building and Running

**Build:**

```bash
cargo build
```

**Run:**

The `geni` binary is the main entry point for all commands.

```bash
# Show help
cargo run -- --help

# Create a new migration
cargo run -- new <migration_name>

# Apply pending migrations
cargo run -- up

# Rollback the last migration
cargo run -- down
```

**Testing:**

```bash
cargo test
```

## Development Conventions

*   **Migrations:** Migrations are written in SQL and placed in the `migrations` directory. Each migration has an `up` and a `down` file.
*   **Transactions:** Migrations are run in a transaction by default. This can be disabled on a per-migration basis.
*   **Configuration:** Database connection and other settings are configured via environment variables (e.g., `DATABASE_URL`, `DATABASE_TOKEN`).
*   **Dependencies:** Project dependencies are managed with `cargo`.
