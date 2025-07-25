{
  description = "Development environment for Trident SSH Launcher - a cross-platform SSH connection launcher";

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

        # Use the latest stable Rust with required components and cross-compilation targets
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
          targets = [ "x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu" "x86_64-apple-darwin" "aarch64-apple-darwin" ];
        };

        # macOS-specific dependencies - minimal approach during SDK transition
        darwinDeps = pkgs.lib.optionals pkgs.stdenv.isDarwin [
          # Let the stdenv provide the default SDK and frameworks
          # Individual frameworks will be auto-detected by the build system
        ];

        # Linux-specific dependencies for X11 and desktop integration
        linuxDeps = pkgs.lib.optionals pkgs.stdenv.isLinux [
          # X11 libraries
          pkgs.xorg.libX11
          pkgs.xorg.libXrandr
          pkgs.xorg.libXcursor
          pkgs.xorg.libXi
          pkgs.xorg.libXext
          pkgs.xorg.libXfixes
          pkgs.libxkbcommon
          pkgs.wayland
          # GLib/GTK dependencies for GPUI
          pkgs.glib
          pkgs.gobject-introspection
          pkgs.gtk3
          pkgs.gtk4
          pkgs.cairo
          pkgs.pango
          pkgs.gdk-pixbuf
          pkgs.atk
          # Additional system libraries
          pkgs.fontconfig
          pkgs.freetype
          pkgs.libGL
          pkgs.libdrm
          pkgs.mesa
          # Tools for testing
          pkgs.wmctrl
          pkgs.xdotool
          # Terminal emulators for testing
          pkgs.gnome-terminal
          pkgs.alacritty
          pkgs.kitty
          pkgs.xterm
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
          # GitHub CLI for repository management
          gh
          # Lima for lightweight Linux VMs on macOS
          lima
        ] ++ darwinDeps ++ linuxDeps;

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
            
            # Set PKG_CONFIG_PATH for Linux builds
            ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
              export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPath "lib/pkgconfig" linuxDeps}:$PKG_CONFIG_PATH"
              echo "PKG_CONFIG_PATH set for Linux: $PKG_CONFIG_PATH"
            ''}
            
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
            
            # Set PKG_CONFIG_PATH for Linux builds
            ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
              export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPath "lib/pkgconfig" linuxDeps}:$PKG_CONFIG_PATH"
              echo "PKG_CONFIG_PATH set for Linux: $PKG_CONFIG_PATH"
            ''}
            
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
            
            # Set PKG_CONFIG_PATH for Linux builds
            ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
              export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPath "lib/pkgconfig" linuxDeps}:$PKG_CONFIG_PATH"
              echo "PKG_CONFIG_PATH set for Linux: $PKG_CONFIG_PATH"
            ''}
            
            cargo build --all-features
            echo "✅ Build check passed!"
          '';
        };

        # Linux build script
        trident-linux-build = pkgs.writeShellApplication {
          name = "trident-linux-build";
          runtimeInputs = buildInputs;
          text = ''
            set -e
            echo "🔱 Building Trident for Linux..."
            
            # Set PKG_CONFIG_PATH for Linux builds
            ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
              export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPath "lib/pkgconfig" linuxDeps}:$PKG_CONFIG_PATH"
              echo "PKG_CONFIG_PATH set for Linux: $PKG_CONFIG_PATH"
            ''}
            
            cargo build --release
            echo "✅ Linux build complete!"
            echo "Binary created at: target/release/trident"
          '';
        };


        # Lima VM management scripts
        trident-lima-start = pkgs.writeShellApplication {
          name = "trident-lima-start";
          runtimeInputs = [ pkgs.lima ];
          text = ''
            set -e
            echo "🔱 Starting Trident Lima Development VM..."
            
            # Start Lima VM with our configuration
            if ! limactl list | grep -q "nix-dev.*Running"; then
              echo "Starting Lima VM..."
              limactl start lima-nix-dev.yaml --name=nix-dev
              echo "✅ Lima VM started successfully!"
            else
              echo "ℹ️  Lima VM already running"
            fi
            
            echo ""
            echo "🚀 Lima VM is ready for development!"
            echo ""
            echo "Available commands:"
            echo "  limactl shell nix-dev                    # Enter VM shell"
            echo "  limactl shell nix-dev nix develop        # Enter Nix development shell"
            echo "  limactl shell nix-dev -- cargo build     # Build in VM"
            echo "  limactl shell nix-dev -- cargo test      # Test in VM"
            echo ""
            echo "Quick development workflow:"
            echo "  nix run .#lima-build                  # Build for Linux via Lima"
            echo "  nix run .#lima-test                   # Test via Lima"
            echo "  nix run .#lima-shell                  # Enter Lima development shell"
          '';
        };

        trident-lima-stop = pkgs.writeShellApplication {
          name = "trident-lima-stop";
          runtimeInputs = [ pkgs.lima ];
          text = ''
            set -e
            echo "🔱 Stopping Trident Lima Development VM..."
            
            if limactl list | grep -q "nix-dev.*Running"; then
              limactl stop nix-dev
              echo "✅ Lima VM stopped successfully!"
            else
              echo "ℹ️  Lima VM not running"
            fi
          '';
        };

        trident-lima-build = pkgs.writeShellApplication {
          name = "trident-lima-build";
          runtimeInputs = [ pkgs.lima ];
          text = ''
            set -e
            echo "🔱 Building Trident for Linux via Lima..."
            
            # Check if Lima VM is running
            if ! limactl list 2>/dev/null | grep -q "nix-dev.*Running"; then
              echo "⚠️  Lima VM not running. Starting it now..."
              limactl start lima-nix-dev.yaml --name=nix-dev
            fi
            
            # Build the project inside Lima VM
            echo "Building inside Lima VM..."
            limactl shell nix-dev -- bash -c "
              cd '$PWD' || { echo 'Project directory not found'; exit 1; }
              nix develop --impure --command bash -c 'cargo build --release --target aarch64-unknown-linux-gnu'
            "
            
            echo "✅ Linux build completed via Lima!"
            echo "Binary available at: target/aarch64-unknown-linux-gnu/release/trident"
          '';
        };

        trident-lima-test = pkgs.writeShellApplication {
          name = "trident-lima-test";
          runtimeInputs = [ pkgs.lima ];
          text = ''
            set -e
            echo "🔱 Running Trident tests via Lima..."
            
            # Check if Lima VM is running
            if ! limactl list 2>/dev/null | grep -q "nix-dev.*Running"; then
              echo "⚠️  Lima VM not running. Starting it now..."
              limactl start lima-nix-dev.yaml --name=nix-dev
            fi
            
            # Run tests inside Lima VM
            echo "Running tests inside Lima VM..."
            limactl shell nix-dev -- bash -c "
              cd '$PWD' || { echo 'Project directory not found'; exit 1; }
              nix develop --impure --command bash -c 'cargo test --target aarch64-unknown-linux-gnu'
            "
            
            echo "✅ Tests completed via Lima!"
          '';
        };

        trident-lima-shell = pkgs.writeShellApplication {
          name = "trident-lima-shell";
          runtimeInputs = [ pkgs.lima ];
          text = ''
            set -e
            echo "🔱 Entering Trident Lima Development Shell..."
            
            # Check if Lima VM is running
            if ! limactl list 2>/dev/null | grep -q "nix-dev.*Running"; then
              echo "⚠️  Lima VM not running. Starting it now..."
              limactl start lima-nix-dev.yaml --name=nix-dev
            fi
            
            # Enter development shell inside Lima VM
            echo "Entering development shell inside Lima VM..."
            limactl shell nix-dev -- bash -c "
              cd '$PWD' || { echo 'Project directory not found'; exit 1; }
              nix develop --impure
            "
          '';
        };

      in
      {
        packages = {
          default = if pkgs.stdenv.isDarwin then trident-build else trident-linux-build;
          
          # Platform-specific builds
          macos = trident-build;
          linux = trident-linux-build;
          
          # Lima development commands
          lima-start = trident-lima-start;
          lima-stop = trident-lima-stop;
          lima-build = trident-lima-build;
          lima-test = trident-lima-test;
          lima-shell = trident-lima-shell;
          
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
            
            # Set PKG_CONFIG_PATH for Linux builds
            ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
              export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPath "lib/pkgconfig" linuxDeps}:$PKG_CONFIG_PATH"
              echo "PKG_CONFIG_PATH set for Linux: $PKG_CONFIG_PATH"
            ''}
            
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
            echo "GitHub commands:"
            echo "  gh pr create         - Create a pull request"
            echo "  gh pr status         - Show pull request status"
            echo "  gh issue create      - Create an issue"
            echo "  gh repo view         - View repository details"
            echo ""
            echo "Nix commands:"
            echo "  nix build            - Build .app bundle"
            echo "  nix flake check      - Run all QA checks"
            echo ""
            echo "Lima Linux development:"
            echo "  nix run .#lima-start - Start Lima VM"
            echo "  nix run .#lima-build - Build for Linux via Lima"
            echo "  nix run .#lima-test  - Run tests via Lima"
            echo "  nix run .#lima-shell - Enter Lima development shell"
            echo "  nix run .#lima-stop  - Stop Lima VM"
            echo ""
            echo "To build the .app bundle: ./build-app.sh"
            echo "The bundle will be created at: target/release/bundle/osx/Trident.app"
            echo ""
            echo "Cross-platform development:"
            echo "  macOS: Use cargo commands and nix build"
            echo "  Linux: Use Lima commands for lightweight VM development"
          '';

          # Rust-specific environment variables
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          
          # macOS-specific environment setup
          MACOSX_DEPLOYMENT_TARGET = "12.0";
        };
      });
}