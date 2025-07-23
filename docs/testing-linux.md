# Testing Trident on Linux

This document describes how to test Trident's Linux implementation using Nix's virtualization support.

## Overview

We've implemented a comprehensive testing setup that allows you to test Trident's Linux functionality from any platform:

- **NixOS VM**: Complete Linux environment with X11, GNOME, and terminal emulators
- **Cross-platform builds**: Linux and macOS builds available through Nix
- **Automated testing**: Scripts to validate functionality

## Quick Start

### Option 1: NixOS Virtual Machine (Recommended)

Run the automated testing script:

```bash
./test-linux.sh
```

This will:
1. Build a NixOS VM with GNOME desktop
2. Install all necessary dependencies and terminal emulators
3. Mount the project directory in the VM
4. Launch the VM for interactive testing

### Option 2: Direct Linux Testing

If you're already on Linux with Nix:

```bash
# Enter development environment
nix develop

# Build for Linux
nix build .#linux

# Run tests
cargo test

# Run Trident
cargo run
```

## VM Testing Environment

The NixOS VM includes:

### Desktop Environment
- **GNOME 45+** with X11 support
- **GDM** display manager  
- **Full desktop experience** for realistic testing

### Terminal Emulators
- GNOME Terminal
- Alacritty 
- Kitty
- xterm

### X11 Tools
- `wmctrl` - Window management for bringing terminals to front
- `xdotool` - Alternative window activation
- `xev` - X11 event viewer for debugging hotkeys

### Development Tools
- Rust toolchain (stable)
- cargo with required tools
- OpenSSH server for testing connections
- Debugging tools (strace, gdb)

## Testing Checklist

### ✅ Hotkey Functionality
1. **X11 Global Hotkey**:
   - Press `Super+Shift+S` 
   - Should open Trident launcher window
   - Verify keycode detection works

2. **Permission Handling**:
   - Test on fresh system (no permissions)
   - Verify helpful error messages
   - Test fallback instructions

### ✅ Terminal Detection
1. **Automatic Detection**:
   - Run `cargo run` and check detected terminals
   - Verify multiple terminals are found
   - Test desktop file parsing

2. **Launch Testing**:
   - Select different terminals from config
   - Verify SSH commands launch correctly
   - Test terminal activation (bring to front)

### ✅ Desktop Integration
1. **Window Management**:
   - Test wmctrl/xdotool integration
   - Verify terminal windows activate properly
   - Test on different window managers

2. **System Tray**:
   - Verify tray icon appears in GNOME
   - Test tray menu functionality
   - Check icon rendering

### ✅ Configuration
1. **Auto-generation**:
   - Delete config file and restart
   - Verify auto-detection generates correct config
   - Test with different terminal preferences

2. **Manual Setup**:
   - Test custom terminal configurations
   - Verify SSH path detection
   - Test config validation

## Debugging Tips

### X11 Hotkey Issues
```bash
# Check if hotkey is registered
xev  # Press keys to see keycodes

# Test X11 connection
echo $DISPLAY

# Check for conflicts
# (Other apps using the same hotkey)
```

### Terminal Detection Issues
```bash
# Check desktop files
find /usr/share/applications -name "*terminal*"
find ~/.local/share/applications -name "*terminal*"

# Test terminal executables
which gnome-terminal alacritty kitty xterm

# Check PATH
echo $PATH
```

### Build Issues
```bash
# Check dependencies
pkg-config --libs x11
pkg-config --libs xrandr

# Verify Rust target
rustup target list --installed
```

## VM Usage

### Starting the VM
```bash
./test-linux.sh
```

### VM Login
- **Username**: `nixos`
- **Password**: `nixos`

### Project Location
The project is mounted at `/home/nixos/trident` in the VM.

### Building in VM
```bash
cd /home/nixos/trident
cargo build --release
cargo run
```

### Shutting Down
Press `Ctrl+C` in the terminal that launched the VM.

## Performance Notes

- VM requires ~4GB RAM and 2 CPU cores
- Graphics acceleration enabled for smooth UI
- File sharing allows real-time code editing on host

## Troubleshooting

### VM Won't Start
- Ensure virtualization is enabled in BIOS
- Check available disk space (VM needs ~2GB)
- Try reducing VM memory in `nixos-test.nix`

### Hotkey Not Working
- Verify X11 is running (not Wayland-only session)
- Check for permission prompts in console
- Test with `xev` to verify key events

### Terminal Detection Fails
- Check if terminals are actually installed
- Verify desktop files are present
- Test manual terminal path configuration

## Contributing Test Cases

When adding new terminal support:

1. Add terminal to `nixos-test.nix` packages
2. Test auto-detection works
3. Verify launch arguments are correct
4. Update this documentation

## Next Steps

After successful VM testing:

1. **Real Hardware Testing**: Test on actual Linux distributions
2. **Different Desktop Environments**: Test KDE, XFCE, etc.
3. **Wayland Testing**: Test compositor-specific integrations
4. **Packaging**: Create distribution packages (deb, rpm, AppImage)