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
          cargo-audit
          cargo-deny
          cargo-outdated
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
            
            # Build the release bundle with cargo-bundle
            if ! cargo bundle --release; then
              echo "Failed to build app bundle"
              exit 1
            fi
            
            APP_PATH="target/release/bundle/osx/Trident.app"
            PLIST_PATH="$APP_PATH/Contents/Info.plist"
            
            echo "Adding LSUIElement to Info.plist..."
            
            # Add LSUIElement if not already present (fixes cargo-bundle v0.7.0 limitation)
            if ! grep -q "LSUIElement" "$PLIST_PATH"; then
              # Create temporary file with LSUIElement added
              awk '/<key>NSHighResolutionCapable<\/key>/ {print "  <key>LSUIElement</key>"; print "  <true/>"; print} !/<key>NSHighResolutionCapable<\/key>/ {print}' "$PLIST_PATH" > "$PLIST_PATH.tmp"
              mv "$PLIST_PATH.tmp" "$PLIST_PATH"
              echo "Added LSUIElement to Info.plist"
            else
              echo "LSUIElement already present in Info.plist"
            fi
            
            # Make the app executable if needed
            chmod +x "$APP_PATH/Contents/MacOS/trident"
            
            echo "✅ Build complete! App bundle created at: $APP_PATH"
            echo "App bundle size: $(du -h "$APP_PATH" | cut -f1)"
            echo ""
            echo "To install to Applications folder:"
            echo "  cp -r $APP_PATH /Applications/"
            echo "Ready to distribute!"
          '';
        };

        # Quality assurance checks
        trident-tests = pkgs.writeShellApplication {
          name = "trident-tests";
          runtimeInputs = buildInputs;
          text = ''
            set -e
            echo "🧪 Running Trident tests..."
            cargo test --all-features
            echo "✅ All tests passed!"
          '';
        };

        trident-clippy = pkgs.writeShellApplication {
          name = "trident-clippy";
          runtimeInputs = buildInputs;
          text = ''
            set -e
            echo "📎 Running Clippy lints..."
            cargo clippy --all-targets --all-features -- -D warnings
            echo "✅ Clippy checks passed!"
          '';
        };

        trident-fmt-check = pkgs.writeShellApplication {
          name = "trident-fmt-check";
          runtimeInputs = buildInputs;
          text = ''
            set -e
            echo "📝 Checking code formatting..."
            cargo fmt --all -- --check
            echo "✅ Code formatting is correct!"
          '';
        };

        trident-audit = pkgs.writeShellApplication {
          name = "trident-audit";
          runtimeInputs = buildInputs;
          text = ''
            set -e
            echo "🔒 Running security audit..."
            cargo audit
            echo "✅ Security audit passed!"
          '';
        };

        trident-deny = pkgs.writeShellApplication {
          name = "trident-deny";
          runtimeInputs = buildInputs;
          text = ''
            set -e
            echo "🚫 Checking licenses and dependencies..."
            if [ -f "deny.toml" ]; then
              cargo deny check
            else
              echo "⚠️  No deny.toml found, skipping cargo deny check"
              echo "   Consider adding deny.toml for dependency/license checking"
            fi
            echo "✅ Dependency checks completed!"
          '';
        };

        trident-build-check = pkgs.writeShellApplication {
          name = "trident-build-check";
          runtimeInputs = buildInputs;
          text = ''
            set -e
            echo "🔨 Checking that project builds..."
            cargo build --all-features
            echo "✅ Build check passed!"
          '';
        };

      in
      {
        packages = {
          default = trident-build;
          
          # Make individual checks available as packages too
          test = trident-tests;
          clippy = trident-clippy;
          fmt-check = trident-fmt-check;
          audit = trident-audit;
          deny = trident-deny;
          build-check = trident-build-check;
        };
        
        # Quality assurance checks for `nix flake check`
        checks = {
          # Run all tests
          tests = trident-tests;
          
          # Lint with clippy (treat warnings as errors)
          clippy = trident-clippy;
          
          # Check code formatting
          formatting = trident-fmt-check;
          
          # Security audit
          audit = trident-audit;
          
          # License and dependency checking
          deny = trident-deny;
          
          # Build verification
          build = trident-build-check;
        };
        
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
            echo "  cargo clippy         - Run linter"
            echo "  cargo fmt            - Format code"
            echo "  cargo audit          - Security audit"
            echo "  cargo outdated       - Check for outdated dependencies"
            echo "  ./build-app.sh       - Build macOS .app bundle"
            echo ""
            echo "Nix commands:"
            echo "  nix build            - Build .app bundle"
            echo "  nix flake check      - Run all QA checks"
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