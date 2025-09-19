# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Geni is a standalone database migration tool written in Rust that supports multiple database systems including PostgreSQL, MySQL, MariaDB, SQLite, and LibSQL. It's designed to work alongside existing ORMs and provides collaborative migration management for teams.

## Development Commands

### Building and Testing
- `cargo build` - Build the project
- `cargo run -- <command>` - Run geni locally with commands (e.g., `cargo run -- status`)
- `make test` - Run integration tests (starts Docker databases, runs tests, cleans up)
- **Manual testing**: Build first, then start Docker, then test:
  1. `cargo build`
  2. `docker compose up -d` - Start test databases
  3. `cargo test` - Run all tests (unit + integration)
  4. `docker compose down -v` - Stop and clean test databases
- `cargo test` (without Docker) - Run unit tests only

### Database URLs for Testing
- SQLite: `sqlite://temp.sqlite`
- Postgres: `postgres://postgres:mysecretpassword@localhost:6437/development?sslmode=disable`
- MySQL: `mysql://root:password@localhost:3306/app`
- MariaDB: `mariadb://root:password@localhost:3307/app`
- LibSQL: `http://localhost:6000`

### Running Geni Commands
All geni commands require a `DATABASE_URL` environment variable:
```bash
DATABASE_URL="sqlite://temp.sqlite" cargo run -- new create_users
DATABASE_URL="postgres://..." cargo run -- up
DATABASE_URL="postgres://..." cargo run -- down -a 2
DATABASE_URL="postgres://..." cargo run -- status
```

## Architecture

### Project Structure
- `src/bin/geni/` - CLI binary entry point and argument parsing
- `src/lib/` - Core library functionality exposed for programmatic use
- `src/lib/database_drivers/` - Database-specific implementations (postgres.rs, mysql.rs, maria.rs, sqlite.rs, libsql.rs)
- `src/lib/migrate.rs` - Core migration logic (up/down operations)
- `src/lib/generate.rs` - Migration file generation
- `src/lib/status.rs` - Migration status checking
- `src/lib/management.rs` - Database creation/dropping
- `src/lib/dump.rs` - Schema dumping functionality

### Key Components
- **CLI Interface**: Uses `clap` for command parsing with subcommands (new, up, down, create, drop, status, dump)
- **Database Drivers**: Modular driver system supporting multiple databases via SQLX (for SQL databases) and libsql-client-rs (for LibSQL)
- **Migration Management**: Timestamp-based migrations with transaction support
- **Library Interface**: Public API exposed in `lib.rs` for programmatic usage

### Configuration
Environment variables used:
- `DATABASE_URL` - Database connection string (required)
- `DATABASE_TOKEN` - Authentication token for LibSQL/Turso
- `DATABASE_MIGRATIONS_FOLDER` - Migration folder path (default: `./migrations`)
- `DATABASE_MIGRATIONS_TABLE` - Migration tracking table name
- `DATABASE_SCHEMA_FILE` - Schema dump file name
- `DATABASE_WAIT_TIMEOUT` - Connection timeout (default: 30s)
- `DATABASE_NO_DUMP_SCHEMA` - Disable schema dumping

### Testing Strategy
- Integration tests in `src/lib/integration_test.rs` test against real databases
- Docker Compose provides test database instances
- Use `make test` to run full integration test suite
- Individual database drivers can be tested by setting appropriate `DATABASE_URL`

## Development Notes

### Adding New Database Support
1. Create new driver in `src/lib/database_drivers/`
2. Implement the required traits/interfaces
3. Add driver to `mod.rs` in database_drivers
4. Update connection logic to handle new URL schemes
5. Add integration tests

### Migration File Format
- Migrations are timestamp-based: `YYYYMMDDHHMMSS_name.up.sql` and `YYYYMMDDHHMMSS_name.down.sql`
- Support transaction control with `-- transaction:no` header
- Schema dumping creates `schema.sql` after successful migrations