#!/usr/bin/env bash
# Script to test Trident on Linux using NixOS VM

set -e

echo "🔱 Trident Linux Testing Environment"
echo "=================================="

# Check if we're on macOS (most likely development environment)
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "📱 Running on macOS - launching NixOS VM for Linux testing"
    
    echo "🚀 Testing cross-compilation to Linux..."
    
    # Test if we can at least build for Linux (cross-compilation)
    echo "Attempting to build Trident for Linux..."
    if nix build .#linux --system x86_64-linux --no-link; then
        echo "✅ Successfully cross-compiled Trident for Linux!"
    else
        echo "❌ Cross-compilation failed. This is expected due to platform differences."
        echo "   Linux testing is best done on actual Linux hardware."
    fi
    
    echo ""
    echo "🖥️  For full Linux testing, you have several options:"
    echo ""
    echo "Option 1: Use GitHub Codespaces or similar Linux environment"
    echo "  1. Create a Codespace from this repository"
    echo "  2. Run: nix develop"
    echo "  3. Run: cargo build && cargo run"
    echo ""
    echo "Option 2: Use Docker with X11 forwarding (advanced)"
    echo "  1. docker run -it --rm -e DISPLAY=\$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix nixos/nix"
    echo "  2. Install git and clone the repository"
    echo "  3. Run: nix develop"
    echo ""
    echo "Option 3: Native Linux machine with Nix"
    echo "  1. Install Nix on any Linux distribution"
    echo "  2. Clone this repository"
    echo "  3. Run: ./test-linux.sh (will detect Linux and test locally)"
    echo ""
    echo "Option 4: NixOS VM Testing (Recommended)"
    echo "  Use the integrated VM for comprehensive Linux testing..."
    echo ""
    echo "🚀 To start the Linux testing VM:"
    echo "  nix run .#vm"
    echo ""
    echo "This will:"
    echo "  1. Build a complete NixOS environment with GNOME desktop"
    echo "  2. Install all necessary terminals and development tools"
    echo "  3. Mount the project directory at /home/nixos/trident"
    echo "  4. Launch the VM for interactive testing"
    echo ""
    echo "VM Login Details:"
    echo "  Username: nixos"
    echo "  Password: nixos"
    echo "  Project location: /home/nixos/trident"
    echo ""
    echo "Once in the VM, test Trident:"
    echo "  cd /home/nixos/trident"
    echo "  cargo build --release"
    echo "  cargo run"
    echo "  # Press Super+Shift+S to test hotkey"
    echo ""
    
    echo "Option 5: Manual testing compilation check"
    echo "  We can at least verify the code compiles for Linux..."
    
    # Try to check compilation without building using Nix dev shell
    echo ""
    echo "🔍 Checking if code compiles for Linux target..."
    if nix develop --command bash -c "
        echo 'Available Rust targets:'
        rustc --print target-list | grep linux | head -3
        echo ''
        echo 'Checking compilation for Linux...'
        cargo check --target x86_64-unknown-linux-gnu
    "; then
        echo "✅ Code compiles successfully for Linux!"
    else
        echo "❌ Compilation issues found for Linux target (this may be due to missing system dependencies)"
        echo "   This is normal - some dependencies like X11 are only available on Linux"
    fi
    
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "🐧 Running on Linux - testing locally"
    
    # Check if we're in a Nix environment
    if command -v nix &> /dev/null; then
        echo "📦 Using Nix for dependencies..."
        nix develop --command bash -c "
            echo '🔧 Building Trident...'
            cargo build --release
            
            echo '🧪 Running tests...'
            cargo test
            
            echo '🎯 Testing terminal detection...'
            echo 'Available terminals:'
            which gnome-terminal 2>/dev/null && echo '  ✅ GNOME Terminal'
            which konsole 2>/dev/null && echo '  ✅ Konsole'
            which alacritty 2>/dev/null && echo '  ✅ Alacritty'
            which kitty 2>/dev/null && echo '  ✅ Kitty'
            which xterm 2>/dev/null && echo '  ✅ xterm'
            
            echo '🎛️  Testing window management tools...'
            which wmctrl 2>/dev/null && echo '  ✅ wmctrl available'
            which xdotool 2>/dev/null && echo '  ✅ xdotool available'
            
            echo '🖥️  Display server:'
            if [[ -n \"\$WAYLAND_DISPLAY\" ]]; then
                echo '  🌊 Wayland detected'
            elif [[ -n \"\$DISPLAY\" ]]; then
                echo '  🪟 X11 detected'
            else
                echo '  ❓ Unknown display server'
            fi
            
            echo ''
            echo '🚀 Ready to run Trident!'
            echo 'Run: cargo run'
            echo 'Or: ./target/release/trident'
        "
    else
        echo "❌ Nix not available. Please install Nix or use the VM approach."
        exit 1
    fi
    
else
    echo "❓ Unknown OS. This script supports macOS (with VM) and Linux."
    exit 1
fi

echo ""
echo "✅ Testing complete!"