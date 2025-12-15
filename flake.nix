{
  description = "Footprints - Self-hosted music history manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
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
        ];

        devInputs = with pkgs; [
          prek
          rustfmt
          clippy
          mold
          clang
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = nativeBuildInputs ++ buildInputs ++ devInputs;

          shellHook = ''
            echo "Footprints development environment"
            echo "Available commands:"
            echo "  cargo build          - Build the project"
            echo "  cargo test           - Run tests"
            echo "  cargo run            - Run the application"
            echo "  cargo clippy         - Run linter"
            echo "  cargo fmt            - Format code"
            echo "  prek run             - Run pre-commit hooks"
            echo ""
          '';

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
      }
    );
}
