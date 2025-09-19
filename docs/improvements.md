# Test Coverage and Quality Improvements

This document tracks potential improvements for the Geni project.

## Missing Test Coverage

### 1. Database Configuration Module (`src/lib/config.rs`)
- [ ] Unit tests for `Database::new()` with valid database URL parsing
- [ ] Unit tests for invalid database URL handling
- [ ] Unit tests for `Database::as_str()` enum conversion methods
- [ ] Edge cases for unknown database types

### 2. Status Command Module (`src/lib/status.rs`)
- [ ] Unit tests for pending migration identification logic
- [ ] Tests for verbose vs non-verbose output formatting
- [ ] Error handling tests for database connection issues
- [ ] Tests for migration comparison logic

### 3. Dump Functionality (`src/lib/dump.rs`)
- [ ] Unit tests for schema dumping logic
- [ ] Error handling tests for dump failures
- [ ] Tests for different database schema formats

### 4. Migration Utils (`src/lib/utils.rs`)
- [ ] Tests for `get_local_migrations()` with various naming patterns
- [ ] Tests for migration file discovery edge cases
- [ ] Tests for file sorting by timestamp
- [ ] Error handling tests for missing/invalid directories
- [ ] Tests for `read_file_content()` edge cases (empty files, permissions, etc.)

### 5. Core Migration Logic (`src/lib/migrate.rs`)
- [ ] Unit tests for migration execution logic (currently only integration tests)
- [ ] Tests for transaction handling
- [ ] Tests for rollback scenarios
- [ ] Error handling for malformed migrations

### 6. Individual Database Drivers
- [ ] Mock-based tests for each driver's specific SQL generation
- [ ] Connection string parsing tests for each database type
- [ ] Error handling tests for database-specific failures
- [ ] Tests for driver-specific migration table creation

## Code Quality Improvements

### Error Handling
- [ ] Add property-based testing with `proptest` for edge cases
- [ ] Replace generic `anyhow::Error` with more specific error types
- [ ] Consider using `thiserror` for better structured error handling
- [ ] Add error context and better error messages

### Performance & Reliability
- [ ] Benchmark tests for large migration sets
- [ ] Memory usage tests for bulk operations
- [ ] Connection pooling and timeout handling tests
- [ ] Concurrent migration execution safety tests

### Documentation
- [ ] Add `cargo doc` examples to all public API functions
- [ ] Document integration test setup and requirements
- [ ] Add examples for programmatic library usage
- [ ] Document testing strategy and Docker requirements

### CI/CD & Tooling
- [ ] Add test coverage reporting (e.g., `tarpaulin`)
- [ ] Set up mutation testing with `cargo-mutants`
- [ ] Create database compatibility matrix testing
- [ ] Add automated dependency updates
- [ ] Set up performance regression testing

### Code Standards
- [ ] Add `clippy::pedantic` lints and address warnings
- [ ] Add more comprehensive logging/tracing throughout
- [ ] Standardize error propagation patterns
- [ ] Add integration with `tracing` for better observability

## Testing Infrastructure Improvements

### Test Organization
- [ ] Separate unit tests from integration tests more clearly
- [ ] Add test utilities/helpers to reduce code duplication
- [ ] Create test fixtures for common migration scenarios
- [ ] Add parameterized tests for multi-database scenarios

### Test Environment
- [ ] Document Docker setup requirements clearly
- [ ] Add test environment validation scripts
- [ ] Create lightweight test database setup for faster iteration
- [ ] Add support for testing against different database versions

## Future Considerations

### New Features to Test
- [ ] Migration rollback safety and validation
- [ ] Concurrent migration handling
- [ ] Migration dependency management
- [ ] Schema validation and drift detection

### Long-term Quality Goals
- [ ] Achieve >90% test coverage
- [ ] Zero-panic guarantee in library code
- [ ] Comprehensive fuzzing of migration parsing
- [ ] Performance benchmarks for large-scale deployments