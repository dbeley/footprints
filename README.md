# 🎵 Footprints

Self-hosted music history manager with stats, reports and charts, inspired by [maloja](https://github.com/krateng/maloja).

## Features

- 📊 **Statistics & Analytics**: Track your listening habits with detailed stats
- 📈 **Charts & Reports**: View yearly, monthly, and all-time reports
- 📅 **Timeline**: Browse your complete listening history
- 🔄 **Multi-source Import**: Import from Last.fm and ListenBrainz
- 🚫 **Deduplication**: Automatic prevention of duplicate scrobbles
- 🐳 **Docker Support**: Easy deployment with Docker and docker-compose
- ⚡ **Lightweight**: Minimal dependencies, fast and efficient
- 🗄️ **SQLite Database**: Simple, portable database storage

## Tech Stack

- **Backend**: Rust with Axum web framework
- **Database**: SQLite with rusqlite
- **Frontend**: Vanilla HTML/CSS/JavaScript (no frameworks)
- **Deployment**: Docker & docker-compose

## Quick Start

### Using Nix Flakes (Recommended for NixOS)

```bash
# Development environment
nix develop

# Run directly
nix run

# Build the package
nix build
```

**NixOS Configuration:**

Add to your `configuration.nix`:

```nix
{
  inputs.footprints.url = "github:dbeley/footprints";
  
  # In your configuration
  services.footprints = {
    enable = true;
    port = 3000;
    openFirewall = true;
    # Optional: environment file for API keys
    environmentFile = /run/secrets/footprints-env;
  };
}
```

### Using Docker Compose

```bash
docker-compose up -d
```

The application will be available at `http://localhost:3000`

### Manual Build

```bash
# Build the application
cargo build --release

# Run the application
cargo run --release
```

## Configuration

Create a `.env` file in the project root (optional):

```env
DATABASE_PATH=footprints.db
PORT=3000
RUST_LOG=footprints=info
```

## Usage

1. **Access the Web Interface**: Open `http://localhost:3000` in your browser

2. **Import Data**:
   - Go to the "Import" tab
   - For Last.fm: Enter your username and API key (get one at https://www.last.fm/api/account/create)
   - For ListenBrainz: Enter your username (token is optional)
   - Click import and wait for the process to complete

3. **View Statistics**:
   - **Overview**: See your top artists and tracks
   - **Timeline**: Browse your listening history chronologically
   - **Reports**: Generate yearly, monthly, or all-time reports

## API Endpoints

- `GET /api/scrobbles?limit=100&offset=0` - Get scrobbles with pagination
- `GET /api/stats` - Get overall statistics
- `GET /api/timeline?limit=50&offset=0` - Get timeline data
- `GET /api/reports/{type}` - Get reports (types: `alltime`, `lastmonth`, `2024`, etc.)
- `POST /api/import` - Import data from Last.fm or ListenBrainz

### Import API Example

```bash
# Last.fm import
curl -X POST http://localhost:3000/api/import \
  -H "Content-Type: application/json" \
  -d '{
    "source": "lastfm",
    "username": "your_username",
    "api_key": "your_api_key"
  }'

# ListenBrainz import
curl -X POST http://localhost:3000/api/import \
  -H "Content-Type: application/json" \
  -d '{
    "source": "listenbrainz",
    "username": "your_username",
    "token": "optional_token"
  }'
```

## Development

### Using Nix (Recommended)

```bash
# Enter development shell with all dependencies
nix develop

# Pre-commit hooks are automatically installed
# They will run on git commit

# Manually run pre-commit on all files
nix flake check
```

### Traditional Development

```bash
# Run in development mode with auto-reload
cargo watch -x run

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

### Pre-commit Hooks

This project uses pre-commit hooks to ensure code quality. With Nix:

```bash
# Hooks are automatically installed in `nix develop`
# They run automatically on `git commit`

# Manually run all hooks
nix flake check
```

Without Nix, install pre-commit manually:

```bash
# Install pre-commit (requires Python)
pip install pre-commit

# Install the hooks
pre-commit install

# Run manually
pre-commit run --all-files
```

## Project Structure

```
footprints/
├── src/
│   ├── api/          # API endpoints and handlers
│   ├── db/           # Database operations
│   ├── importers/    # Last.fm and ListenBrainz importers
│   ├── models/       # Data models
│   ├── reports/      # Report generation
│   └── main.rs       # Application entry point
├── templates/        # HTML templates
├── static/           # Static assets (if needed)
├── Dockerfile        # Docker configuration
├── docker-compose.yml
└── Cargo.toml        # Rust dependencies
```

## License

MIT

## Acknowledgments

Inspired by [maloja](https://github.com/krateng/maloja) - a self-hosted music scrobble database
