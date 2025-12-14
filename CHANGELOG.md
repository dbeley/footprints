# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-12-14

### Added
- Initial release of Footprints music history manager
- SQLite database with scrobbles table and indices
- Last.fm data import via API
- ListenBrainz data import via API
- Automatic deduplication of scrobbles
- REST API endpoints:
  - `GET /` - Main web interface
  - `GET /api/scrobbles` - Get scrobbles with pagination
  - `GET /api/stats` - Get overall statistics
  - `GET /api/timeline` - Get timeline data
  - `GET /api/reports/:type` - Get reports
  - `POST /api/import` - Import from Last.fm or ListenBrainz
- Minimalist web frontend with tabs:
  - Overview: Top artists and tracks
  - Timeline: Chronological listening history
  - Reports: Yearly, monthly, and all-time reports
  - Import: Data import interface
- Statistics features:
  - Top artists, tracks, and albums
  - Play counts and rankings
  - Time-based filtering
- Report generation:
  - All-time reports
  - Yearly reports (e.g., 2024, 2023)
  - Monthly reports
  - Last month report
- Docker support with multi-stage build
- docker-compose configuration
- Comprehensive documentation:
  - README with quick start guide
  - ARCHITECTURE document
  - CONTRIBUTING guidelines
  - MIT License
- Test suite covering database operations
- Makefile for common tasks
- Development tooling configuration

### Technical Details
- Built with Rust 1.75+ and Axum web framework
- SQLite database with r2d2 connection pooling
- Async/await with tokio runtime
- No frontend dependencies (vanilla JavaScript)
- Environment variable configuration
- Logging with tracing and tracing-subscriber

[0.1.0]: https://github.com/dbeley/footprints/releases/tag/v0.1.0
