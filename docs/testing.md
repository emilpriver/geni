# Testing Guide

This document explains how to run tests for the Geni project.

## Test Types

Geni has two types of tests:
- **Unit tests**: Fast tests that don't require external dependencies
- **Integration tests**: Tests that require real database connections via Docker

## Prerequisites

### For Unit Tests Only
- Rust toolchain (`cargo`)

### For Integration Tests
- Rust toolchain (`cargo`)
- Docker and Docker Compose
- All test databases running

## Running Tests

### Quick Unit Tests (No Docker Required)

To run only unit tests without starting Docker:

```bash
# This will skip integration tests if Docker databases aren't available
cargo test --lib -- --skip integration_test
```

### Full Test Suite (Requires Docker)

**Option 1: Automated (Recommended)**
```bash
make test
```
This command:
1. Starts Docker databases
2. Runs all tests
3. Cleans up databases automatically

**Option 2: Manual Control**
```bash
# 1. Build the project first
cargo build

# 2. Start test databases
docker compose up -d

# 3. Run all tests (unit + integration)
cargo test

# 4. Stop and clean up databases when done
docker compose down -v
```

### Running Specific Tests

```bash
# Run only integration tests
cargo test integration_test

# Run specific database integration test
cargo test test_migrate_postgres

# Run only unit tests
cargo test utils::tests
cargo test generate::tests
```

## Test Database Configuration

The integration tests connect to these databases running in Docker:

| Database | URL | Port |
|----------|-----|------|
| SQLite | `sqlite://temp.sqlite` | N/A |
| PostgreSQL | `psql://postgres:mysecretpassword@localhost:6437/app` | 6437 |
| MySQL | `mysql://root:password@localhost:3306/app` | 3306 |
| MariaDB | `mariadb://root:password@localhost:3307/app` | 3307 |
| LibSQL | `http://localhost:6000` | 6000 |

## Troubleshooting

### "Connection refused" errors
- Ensure Docker is running: `docker --version`
- Check if databases are up: `docker compose ps`
- Restart databases: `docker compose down -v && docker compose up -d`

### Tests hang or timeout
- The databases might be starting up
- Wait 30 seconds after `docker compose up -d` before running tests
- Check database logs: `docker compose logs`

### Port conflicts
- Check if ports 3306, 3307, 6000, 6437 are available
- Stop other services using these ports
- Modify `docker-compose.yml` if needed (update test URLs accordingly)

### Permission errors
- Ensure Docker daemon is running
- Check Docker permissions: `docker ps`

## Test Development

### Adding Unit Tests
Unit tests go in the same file as the code being tested:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        // Test implementation
    }
}
```

### Adding Integration Tests
Integration tests should be added to `src/lib/integration_test.rs` and use the `#[serial]` attribute to prevent database conflicts:

```rust
#[test]
#[serial]
async fn test_my_feature() -> Result<()> {
    // Test implementation with real database
}
```

## CI/CD Considerations

For continuous integration:
1. Use `make test` which handles Docker lifecycle
2. Ensure Docker is available in the CI environment
3. Consider using database services instead of Docker in CI for better performance

## Performance Notes

- Unit tests: ~1 second
- Integration tests: ~15-30 seconds (including database setup)
- Full test suite: ~45 seconds with Docker startup time

For faster development iteration, use unit tests when possible and run integration tests before committing changes.