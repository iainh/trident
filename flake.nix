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
          targets = [ "x86_64-unknown-linux-gnu" "x86_64-apple-darwin" "aarch64-apple-darwin" ];
        };

        # macOS-specific dependencies - minimal approach during SDK transition
        darwinDeps = pkgs.lib.optionals pkgs.stdenv.isDarwin [
          # Let the stdenv provide the default SDK and frameworks
          # Individual frameworks will be auto-detected by the build system
        ];

        # Linux-specific dependencies for X11 and desktop integration
        linuxDeps = pkgs.lib.optionals pkgs.stdenv.isLinux [
          pkgs.xorg.libX11
          pkgs.xorg.libXrandr
          pkgs.xorg.libXcursor
          pkgs.xorg.libXi
          pkgs.xorg.libXext
          pkgs.xorg.libXfixes
          pkgs.libxkbcommon
          pkgs.wayland
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
          # QEMU for VM testing
          qemu
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

        # Linux build script
        trident-linux-build = pkgs.writeShellApplication {
          name = "trident-linux-build";
          runtimeInputs = buildInputs;
          text = ''
            set -e
            echo "🔱 Building Trident for Linux..."
            cargo build --release
            echo "✅ Linux build complete!"
            echo "Binary created at: target/release/trident"
          '';
        };

        # VM script for easy Linux testing
        trident-vm = pkgs.writeShellApplication {
          name = "trident-vm";
          runtimeInputs = [ pkgs.qemu ];
          text = ''
            set -e
            echo "🔱 Starting Trident Linux Testing VM..."
            echo ""
            
            # Check if we're on macOS
            if [[ "$(uname)" == "Darwin" ]]; then
              echo "🍎 Running on macOS - Cross-system VM building has limitations"
              echo ""
              echo "❌ Issue: macOS cannot build Linux VMs natively due to Nix security restrictions"
              echo ""
              echo "🔧 Solutions:"
              echo ""
              echo "1. Use UTM (recommended for macOS):"
              echo "   • Download UTM from Mac App Store"
              echo "   • Download NixOS ISO from https://nixos.org/download"
              echo "   • Create VM in UTM with NixOS ISO"
              echo "   • Install Nix in the VM: sh <(curl -L https://nixos.org/nix/install)"
              echo ""
              echo "2. Use GitHub Codespaces:"
              echo "   • Create a Codespace from this repository"
              echo "   • Run: nix develop && cargo build && cargo run"
              echo ""
              echo "3. Use Docker with X11 forwarding:"
              echo "   • Install XQuartz: brew install --cask xquartz"
              echo "   • Run: xhost +local:docker"
              echo "   • docker run -it --rm -e DISPLAY=host.docker.internal:0 -v $(pwd):/workspace nixos/nix"
              echo ""
              echo "4. Enable Nix trusted users (requires admin):"
              echo "   • Add to /etc/nix/nix.conf: trusted-users = root $(whoami)"
              echo "   • Restart Nix daemon: sudo launchctl unload /Library/LaunchDaemons/org.nixos.nix-daemon.plist"
              echo "   • sudo launchctl load /Library/LaunchDaemons/org.nixos.nix-daemon.plist"
              echo ""
              exit 1
            fi
            
            echo "🐧 Running on Linux - Building NixOS VM..."
            
            # Build the VM if it doesn't exist
            if ! nix build ".#nixosConfigurations.test-vm.config.system.build.vm" --out-link ./vm-result 2>/dev/null; then
              echo "❌ Failed to build VM. This may require:"
              echo "  1. Sufficient disk space (~2GB)"
              echo "  2. Virtualization support"
              echo ""
              exit 1
            fi
            
            echo "✅ VM built successfully!"
            echo ""
            echo "🚀 Launching VM..."
            echo "Login: username=nixos, password=nixos"
            echo "Project location: /home/nixos/trident"
            echo ""
            echo "To stop the VM: Press Ctrl+C"
            echo ""
            
            # Run the VM
            ./vm-result/bin/run-nixos-vm
          '';
        };

      in
      {
        packages = {
          default = if pkgs.stdenv.isDarwin then trident-build else trident-linux-build;
          
          # Platform-specific builds
          macos = trident-build;
          linux = trident-linux-build;
          
          # VM for Linux testing
          vm = trident-vm;
          
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
            echo "GitHub commands:"
            echo "  gh pr create         - Create a pull request"
            echo "  gh pr status         - Show pull request status"
            echo "  gh issue create      - Create an issue"
            echo "  gh repo view         - View repository details"
            echo ""
            echo "Nix commands:"
            echo "  nix build            - Build .app bundle"
            echo "  nix run .#vm         - Start Linux testing VM"
            echo "  nix flake check      - Run all QA checks"
            echo ""
            echo "To build the .app bundle: ./build-app.sh"
            echo "The bundle will be created at: target/release/bundle/osx/Trident.app"
            echo ""
            echo "To test Linux implementation: nix run .#vm"
          '';

          # Rust-specific environment variables
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          
          # macOS-specific environment setup
          MACOSX_DEPLOYMENT_TARGET = "12.0";
        };
      }) // {
        # NixOS VM for Linux testing
        nixosConfigurations.test-vm = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            # Import the VM module first
            "${nixpkgs}/nixos/modules/virtualisation/qemu-vm.nix"
            ./nixos-test.nix
            {
              # Make the rust toolchain available in the VM
              environment.systemPackages = [
                (import nixpkgs {
                  system = "x86_64-linux";
                  overlays = [ (import rust-overlay) ];
                }).rust-bin.stable.latest.default
              ];
            }
          ];
        };
      };
}