# Contributing to Footprints

Thank you for your interest in contributing to Footprints! This document provides guidelines and instructions for contributing.

## Development Setup

1. **Prerequisites**:
   - Rust 1.75 or later
   - SQLite3 development libraries
   - Docker (optional, for testing containerized deployment)

2. **Clone and Build**:
   ```bash
   git clone https://github.com/dbeley/footprints.git
   cd footprints
   cargo build
   ```

3. **Run Tests**:
   ```bash
   cargo test
   ```

4. **Run in Development**:
   ```bash
   cargo run
   ```

## Code Style

- Follow Rust standard formatting with `cargo fmt`
- Run `cargo clippy` and address all warnings
- Write tests for new functionality
- Keep dependencies minimal

## Project Structure

```
src/
├── api/          # Web API endpoints
├── db/           # Database operations
├── importers/    # Data import from Last.fm and ListenBrainz
├── models/       # Data models
├── reports/      # Report generation
└── main.rs       # Application entry point
```

## Adding New Features

1. Create a feature branch from `main`
2. Implement your feature with tests
3. Ensure all tests pass
4. Update documentation if needed
5. Submit a pull request

## Reporting Issues

When reporting issues, please include:
- Steps to reproduce
- Expected behavior
- Actual behavior
- Environment details (OS, Rust version, etc.)

## Pull Request Process

1. Update README.md with details of changes if needed
2. Ensure tests pass and code is formatted
3. Update CHANGELOG if applicable
4. Pull request will be reviewed by maintainers

## Questions?

Feel free to open an issue for questions or clarifications.
