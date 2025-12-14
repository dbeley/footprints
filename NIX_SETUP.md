# Nix Setup Guide for Footprints

This guide explains how to use the Nix flake for development and deployment of Footprints.

## Prerequisites

- [Nix](https://nixos.org/download.html) with flakes enabled
- (Optional) [direnv](https://direnv.net/) for automatic environment loading

### Enable Nix Flakes

Add to `~/.config/nix/nix.conf` or `/etc/nix/nix.conf`:

```
experimental-features = nix-command flakes
```

## Development Environment

### Quick Start

```bash
# Enter development shell
nix develop

# Or with direnv (automatic)
echo "use flake" > .envrc
direnv allow
```

### What's Included

The development environment provides:

- **Rust toolchain**: Latest stable Rust with rust-analyzer and rust-src
- **Build dependencies**: SQLite, OpenSSL, pkg-config
- **Pre-commit hooks**: Automatically installed and configured
- **Environment variables**: DATABASE_PATH, RUST_LOG, RUST_SRC_PATH

### Pre-commit Hooks

Pre-commit hooks are automatically installed when entering the dev shell. They include:

- **rustfmt**: Ensures code is formatted correctly
- **clippy**: Rust linter to catch common mistakes
- **cargo-check**: Verifies the code compiles
- **trailing-whitespace**: Removes trailing whitespace
- **end-of-file-fixer**: Ensures files end with newline
- **check-yaml**: Validates YAML files
- **check-added-large-files**: Prevents large files from being committed

To manually run all checks:

```bash
nix flake check
```

## Building

### Build the Application

```bash
# Build with Nix
nix build

# The binary will be in ./result/bin/footprints
./result/bin/footprints
```

### Run Directly

```bash
# Run without building
nix run

# Run from GitHub (no need to clone)
nix run github:dbeley/footprints
```

## NixOS Deployment

### Flake-based Configuration

Add Footprints to your NixOS flake:

```nix
{
  description = "My NixOS configuration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    footprints.url = "github:dbeley/footprints";
  };

  outputs = { self, nixpkgs, footprints }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        footprints.nixosModules.default
        ./configuration.nix
      ];
    };
  };
}
```

### Configuration Options

In your `configuration.nix`:

```nix
{ config, pkgs, ... }:

{
  services.footprints = {
    enable = true;
    
    # Port configuration
    port = 3000;
    
    # Open firewall
    openFirewall = true;
    
    # Data directory (default: /var/lib/footprints)
    dataDir = "/var/lib/footprints";
    
    # User and group (default: footprints)
    user = "footprints";
    group = "footprints";
    
    # Optional: Environment file for secrets (API keys, etc.)
    # Create this file with:
    # LASTFM_API_KEY=your_key_here
    environmentFile = /run/secrets/footprints-env;
  };
}
```

### With nginx Reverse Proxy

```nix
{ config, pkgs, ... }:

{
  services.footprints = {
    enable = true;
    port = 3000;
    openFirewall = false;  # nginx will handle this
  };

  services.nginx = {
    enable = true;
    recommendedProxySettings = true;
    
    virtualHosts."footprints.example.com" = {
      enableACME = true;
      forceSSL = true;
      
      locations."/" = {
        proxyPass = "http://127.0.0.1:3000";
        proxyWebsockets = true;
      };
    };
  };
  
  networking.firewall.allowedTCPPorts = [ 80 443 ];
}
```

### Security Hardening

The NixOS module includes comprehensive systemd security hardening:

- **Filesystem protection**: Read-only system, protected home directory
- **Process isolation**: Private /tmp, restricted address families
- **System call filtering**: Limited to safe system calls
- **Capability restrictions**: No new privileges, no SUID/SGID
- **Kernel protection**: Protected kernel tunables and modules

### Secrets Management

For API keys and other secrets, use NixOS secrets management:

#### With agenix

```nix
{
  age.secrets.footprints-env = {
    file = ./secrets/footprints-env.age;
    owner = "footprints";
    group = "footprints";
  };

  services.footprints = {
    enable = true;
    environmentFile = config.age.secrets.footprints-env.path;
  };
}
```

#### With sops-nix

```nix
{
  sops.secrets.footprints-env = {
    owner = "footprints";
    group = "footprints";
  };

  services.footprints = {
    enable = true;
    environmentFile = config.sops.secrets.footprints-env.path;
  };
}
```

## Testing

### Run Tests

```bash
# In development shell
cargo test

# Or directly with Nix
nix develop -c cargo test
```

### Run Pre-commit Checks

```bash
# Run all checks
nix flake check

# This runs:
# - rustfmt (format checking)
# - clippy (linting)
# - cargo check (compilation)
# - file quality checks
```

## Updating Dependencies

### Update Flake Inputs

```bash
# Update all inputs
nix flake update

# Update specific input
nix flake lock --update-input nixpkgs

# Update Rust toolchain
nix flake lock --update-input rust-overlay
```

### Update Cargo Dependencies

```bash
# In development shell
cargo update

# Update Cargo.lock in flake
nix flake check
```

## Troubleshooting

### Pre-commit Hooks Not Working

```bash
# Reinstall hooks
nix develop
# Hooks are automatically installed

# Or manually
pre-commit install
```

### Build Failures

```bash
# Clean build
cargo clean
nix build --rebuild

# Check for dependency issues
nix flake check
```

### Permission Issues on NixOS

```bash
# Check service status
systemctl status footprints

# Check logs
journalctl -u footprints -f

# Verify data directory permissions
ls -la /var/lib/footprints
```

## Continuous Integration

### GitHub Actions with Nix

```yaml
name: CI
on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: cachix/install-nix-action@v22
        with:
          extra_nix_config: |
            experimental-features = nix-command flakes
      
      - name: Check flake
        run: nix flake check
      
      - name: Build
        run: nix build
      
      - name: Run tests
        run: nix develop -c cargo test
```

## Additional Resources

- [Nix Manual](https://nixos.org/manual/nix/stable/)
- [NixOS Manual](https://nixos.org/manual/nixos/stable/)
- [Nix Flakes Book](https://nixos-and-flakes.thiscute.world/)
- [Zero to Nix](https://zero-to-nix.com/)
