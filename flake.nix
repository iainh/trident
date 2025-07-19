{
  description = "Development environment for Trident SSH Launcher - a macOS menubar application";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Use the latest stable Rust with required components
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };

        # macOS-specific dependencies - minimal approach during SDK transition
        darwinDeps = pkgs.lib.optionals pkgs.stdenv.isDarwin [
          # Let the stdenv provide the default SDK and frameworks
          # Individual frameworks will be auto-detected by the build system
        ];

        # Development dependencies
        buildInputs = with pkgs; [
          rustToolchain
          cargo-bundle
          pkg-config
          openssl
        ] ++ darwinDeps;

        # Build script that creates the .app bundle
        trident-build = pkgs.writeShellApplication {
          name = "trident-build";
          runtimeInputs = buildInputs;
          text = ''
            set -e
            echo "🔱 Building Trident.app bundle..."
            
            if [ ! -f "./build-app.sh" ]; then
              echo "Error: build-app.sh not found. Please run from the project root."
              exit 1
            fi
            
            # Build the .app bundle
            ./build-app.sh
            
            echo "✅ Build complete! App bundle created at: target/release/bundle/osx/Trident.app"
            echo ""
            echo "To install to Applications folder:"
            echo "  cp -r target/release/bundle/osx/Trident.app /Applications/"
          '';
        };

      in
      {
        packages.default = trident-build;
        
        devShells.default = pkgs.mkShell {
          inherit buildInputs;

          # Environment variables for development
          shellHook = ''
            echo "🔱 Trident SSH Launcher Development Environment"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
            echo "cargo-bundle: $(cargo bundle --version 2>/dev/null || echo 'Installing...')"
            
            # Ensure cargo-bundle is available
            if ! command -v cargo-bundle &> /dev/null; then
              echo "Installing cargo-bundle..."
              cargo install cargo-bundle
            fi
            
            echo ""
            echo "Available commands:"
            echo "  cargo build          - Build the project"
            echo "  cargo run            - Run the application"
            echo "  cargo test           - Run tests"
            echo "  ./build-app.sh       - Build macOS .app bundle"
            echo "  cargo bundle --release - Build app bundle directly"
            echo ""
            echo "To build the .app bundle: ./build-app.sh"
            echo "The bundle will be created at: target/release/bundle/osx/Trident.app"
          '';

          # Rust-specific environment variables
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          
          # macOS-specific environment setup
          MACOSX_DEPLOYMENT_TARGET = "12.0";
        };
      });
}