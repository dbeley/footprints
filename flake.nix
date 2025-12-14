{
  description = "Footprints - Self-hosted music history manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, pre-commit-hooks }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

        buildInputs = with pkgs; [
          sqlite
          openssl
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
        ];

      in
      {
        checks = {
          pre-commit-check = pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              # Rust hooks
              rustfmt = {
                enable = true;
                entry = "${rustToolchain}/bin/cargo fmt --check";
              };
              clippy = {
                enable = true;
                entry = "${rustToolchain}/bin/cargo clippy -- -D warnings";
              };
              cargo-check = {
                enable = true;
                entry = "${rustToolchain}/bin/cargo check";
              };
              
              # General hooks
              trailing-whitespace = {
                enable = true;
                entry = "${pkgs.python3Packages.pre-commit-hooks}/bin/trailing-whitespace-fixer";
              };
              end-of-file-fixer = {
                enable = true;
                entry = "${pkgs.python3Packages.pre-commit-hooks}/bin/end-of-file-fixer";
              };
              check-yaml = {
                enable = true;
                entry = "${pkgs.python3Packages.pre-commit-hooks}/bin/check-yaml";
              };
              check-added-large-files = {
                enable = true;
                entry = "${pkgs.python3Packages.pre-commit-hooks}/bin/check-added-large-files";
              };
            };
          };
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          
          shellHook = ''
            ${self.checks.${system}.pre-commit-check.shellHook}
            echo "🎵 Footprints development environment"
            echo "Rust version: $(rustc --version)"
            echo ""
            echo "Available commands:"
            echo "  cargo build          - Build the project"
            echo "  cargo test           - Run tests"
            echo "  cargo run            - Run the application"
            echo "  cargo clippy         - Run linter"
            echo "  cargo fmt            - Format code"
            echo ""
          '';

          # Environment variables
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          DATABASE_PATH = "footprints.db";
          RUST_LOG = "footprints=info";
        };

        packages = {
          default = self.packages.${system}.footprints;
          
          footprints = pkgs.rustPlatform.buildRustPackage {
            pname = "footprints";
            version = "0.1.0";
            
            src = ./.;
            
            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs;

            meta = with pkgs.lib; {
              description = "Self-hosted music history manager with stats, reports and charts";
              homepage = "https://github.com/dbeley/footprints";
              license = licenses.mit;
              maintainers = [ ];
            };
          };
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.footprints}/bin/footprints";
        };
      }
    ) // {
      # NixOS module for system-wide deployment
      nixosModules.default = { config, lib, pkgs, ... }:
        with lib;
        let
          cfg = config.services.footprints;
        in
        {
          options.services.footprints = {
            enable = mkEnableOption "Footprints music history manager";

            package = mkOption {
              type = types.package;
              default = self.packages.${pkgs.system}.footprints;
              description = "The footprints package to use";
            };

            user = mkOption {
              type = types.str;
              default = "footprints";
              description = "User account under which footprints runs";
            };

            group = mkOption {
              type = types.str;
              default = "footprints";
              description = "Group under which footprints runs";
            };

            dataDir = mkOption {
              type = types.path;
              default = "/var/lib/footprints";
              description = "Directory where footprints stores its database";
            };

            port = mkOption {
              type = types.port;
              default = 3000;
              description = "Port on which footprints listens";
            };

            openFirewall = mkOption {
              type = types.bool;
              default = false;
              description = "Whether to open the firewall for the footprints port";
            };

            environmentFile = mkOption {
              type = types.nullOr types.path;
              default = null;
              description = "Environment file containing secrets (API keys, etc.)";
            };
          };

          config = mkIf cfg.enable {
            users.users.${cfg.user} = {
              isSystemUser = true;
              group = cfg.group;
              home = cfg.dataDir;
              createHome = true;
            };

            users.groups.${cfg.group} = {};

            systemd.services.footprints = {
              description = "Footprints music history manager";
              wantedBy = [ "multi-user.target" ];
              after = [ "network.target" ];

              serviceConfig = {
                Type = "simple";
                User = cfg.user;
                Group = cfg.group;
                WorkingDirectory = cfg.dataDir;
                ExecStart = "${cfg.package}/bin/footprints";
                Restart = "on-failure";
                RestartSec = "5s";

                # Security hardening
                NoNewPrivileges = true;
                PrivateTmp = true;
                ProtectSystem = "strict";
                ProtectHome = true;
                ReadWritePaths = cfg.dataDir;
                ProtectKernelTunables = true;
                ProtectKernelModules = true;
                ProtectControlGroups = true;
                RestrictAddressFamilies = [ "AF_INET" "AF_INET6" "AF_UNIX" ];
                RestrictNamespaces = true;
                LockPersonality = true;
                RestrictRealtime = true;
                RestrictSUIDSGID = true;
                PrivateDevices = true;
                ProtectHostname = true;
                ProtectClock = true;
                ProtectKernelLogs = true;
                ProtectProc = "invisible";
                ProcSubset = "pid";
                SystemCallArchitectures = "native";
                SystemCallFilter = [ "@system-service" "~@privileged" ];
              };

              environment = {
                DATABASE_PATH = "${cfg.dataDir}/footprints.db";
                PORT = toString cfg.port;
                RUST_LOG = "footprints=info,tower_http=info";
              };

              serviceConfig.EnvironmentFile = mkIf (cfg.environmentFile != null) cfg.environmentFile;
            };

            networking.firewall = mkIf cfg.openFirewall {
              allowedTCPPorts = [ cfg.port ];
            };
          };
        };
    };
}
