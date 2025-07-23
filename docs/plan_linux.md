# Unix Port Plan for Trident SSH Launcher (Linux/FreeBSD)

## Current macOS-Specific Code Analysis

### Platform-Specific Dependencies (Cargo.toml)
- **objc2** ecosystem (objc2, objc2-app-kit, objc2-foundation, objc2-core-foundation, block2)
- **ApplicationServices** framework linking (build.rs)
- **LSUIElement** plist entry for dock hiding

### Platform-Specific Code Files
1. **src/objc2_hotkey.rs** - Native global hotkey using NSEvent and AXIsProcessTrusted
2. **src/menubar.rs** - NSStatusBar/NSStatusItem implementation (currently unused)
3. **src/tray.rs** - Uses tray-icon crate (already cross-platform) with macOS template mode
4. **src/ssh/launcher.rs** - macOS `open` command and AppleScript activation
5. **src/config.rs** - macOS terminal detection and .app bundle paths

## Proposed Architecture Changes

### 1. Platform Abstraction Layer
Create a clean separation between platform-specific and shared code:

```
src/
├── platform/           # New platform abstraction
│   ├── mod.rs          # Platform trait definitions
│   ├── macos/          # macOS implementations
│   │   ├── mod.rs
│   │   ├── hotkey.rs   # Move objc2_hotkey logic here
│   │   ├── launcher.rs # macOS terminal launching
│   │   └── config.rs   # macOS-specific config detection
│   └── unix/           # Unix implementations (Linux/FreeBSD)
│       ├── mod.rs
│       ├── hotkey.rs   # X11/Wayland global hotkey
│       ├── launcher.rs # Unix terminal launching
│       └── config.rs   # Unix terminal detection
├── core/               # Rename existing modules
│   ├── app.rs         # Cross-platform app logic
│   ├── config.rs      # Shared config structures
│   ├── fuzzy.rs       # Search algorithm
│   └── ssh/           # SSH parsing (already cross-platform)
└── ui/                # UI remains cross-platform (GPUI)
```

### 2. Platform Traits
Define clean interfaces for platform-specific functionality:

```rust
pub trait HotkeyManager {
    fn register_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()>;
    fn check_permissions(&self) -> bool;
    fn prompt_for_permissions(&self) -> bool;
}

pub trait TerminalLauncher {
    fn launch_command(&self, command: &str, config: &TerminalConfig) -> Result<()>;
    fn bring_to_front(&self, app_name: &str) -> Result<()>;
}

pub trait ConfigDetector {
    fn detect_terminals() -> Vec<DetectedTerminal>;
    fn get_default_ssh_paths() -> SshPaths;
}
```

### 3. Unix-Specific Implementation (Linux/FreeBSD)

#### Hotkey Management (src/platform/unix/hotkey.rs)
- **X11**: Use `x11rb` or `xcb` for global hotkey registration via `XGrabKey`
- **Wayland**: Limited global hotkey support (Linux only - FreeBSD uses X11)
- **Fallback**: Desktop environment-specific solutions (GNOME Shell, KDE shortcuts)
- Handle display server detection and graceful degradation

#### Terminal Launching (src/platform/unix/launcher.rs)
- Detect Unix terminals: GNOME Terminal, Konsole, Alacritty, Kitty, xterm, urxvt
- Use `std::process::Command` for launching with proper environment
- Handle desktop environment integration (dbus, window activation)
- Support for terminal detection through .desktop files and package managers

#### Configuration (src/platform/unix/config.rs)
- Detect installed terminals from multiple sources:
  - Desktop files (Linux): `/usr/share/applications/`, `~/.local/share/applications/`
  - Package managers: apt, yum, pacman (Linux), pkg, ports (FreeBSD)
  - Standard paths: `/usr/bin/`, `/usr/local/bin/`
- Unix SSH paths: `~/.ssh/`, `/etc/ssh/`
- XDG Base Directory compliance for config files (Linux)
- FreeBSD-specific path preferences: `/usr/local/bin/` over `/usr/bin/`

### 4. Cargo.toml Changes
```toml
[target.'cfg(target_os = "macos")'.dependencies]
# Existing macOS deps...

[target.'cfg(any(target_os = "linux", target_os = "freebsd"))'.dependencies]
x11rb = { version = "0.13", features = ["extra"] }
# Alternative: xcb = "1.4"

[target.'cfg(target_os = "linux")'.dependencies]
libappindicator = "0.9"  # For system tray (Linux-specific)
freedesktop_desktop_entry = "0.5"  # Desktop file parsing (Linux-specific)
```

### 5. Build System Updates
- Update `build.rs` for X11 library linking
- Create Unix packaging scripts:
  - Linux: AppImage, deb, rpm, Flatpak
  - FreeBSD: port files for ports system
- Update `flake.nix` to support Unix cross-compilation
- Add Unix CI/CD pipelines

### 6. Configuration Format Changes
Extend config.toml to support Unix terminals:
```toml
# GNOME Terminal
program = "gnome-terminal"
args = ["--", "{ssh_command}"]

# Konsole
program = "konsole"
args = ["-e", "{ssh_command}"]

# Alacritty (cross-platform)
program = "alacritty"
args = ["-e", "{ssh_command}"]

# Kitty
program = "kitty"
args = ["{ssh_command}"]
```

## Implementation Phases

### Phase 1: Platform Abstraction (2-3 days)
1. Create platform trait definitions
2. Move macOS code into platform/macos/
3. Update imports and module structure
4. Ensure macOS functionality remains unchanged

### Phase 2: Unix Core Implementation (4-5 days)
1. Implement X11 hotkey manager with display server detection
2. Implement Wayland hotkey handling (Linux only, with fallback)
3. Implement Unix terminal launcher with desktop/package manager integration
4. Implement config detection supporting both Linux and FreeBSD patterns
5. Add Unix-specific dependencies with conditional compilation

### Phase 3: Integration & Testing (3-4 days)
1. Update main.rs for platform selection
2. Cross-platform testing framework
3. Unix packaging (AppImage, deb, rpm, FreeBSD ports)
4. Documentation updates

### Phase 4: Polish & Distribution (2-3 days)
1. Unix desktop integration (system tray, notifications)
2. Desktop environment support across both platforms
3. Performance optimization
4. User experience refinements

## Benefits of This Approach
- **Clean Separation**: Platform code isolated from business logic
- **Maintainable**: Easy to add future Unix-like platforms
- **Testable**: Platform traits can be mocked for testing
- **Consistent**: Same user experience across platforms
- **Extensible**: Easy to add platform-specific features

## Critical Linux/FreeBSD-Specific Edge Cases

### Display Server Diversity
- **X11 vs Wayland**: Different hotkey registration mechanisms
- **Multiple Display Servers**: Some distros support both simultaneously
- **Compositor Fragmentation**: Wayland compositors have different global hotkey support
- **Remote Displays**: SSH forwarding and DISPLAY variable handling

### Terminal Ecosystem Complexity
- **Package Manager Variance**: apt, yum, pacman, pkg, ports detection
- **Distribution Differences**: Terminal availability varies by distro
- **Custom Builds**: User-compiled terminals in non-standard locations
- **Desktop Environment Integration**: Different DE's have different terminal preferences

### Permission and Security Models
- **No Accessibility API**: Unlike macOS, no central permission system for global hotkeys
- **X11 Security**: X11 allows global key grabbing by default (security concern)
- **Wayland Security**: Wayland's security model limits global hotkey capabilities
- **Sandboxing**: Flatpak/Snap applications have different file access patterns

### System Integration Challenges
- **System Tray**: Multiple incompatible tray implementations (libappindicator, KDE, etc.)
- **Notification Systems**: D-Bus notifications vs distro-specific systems
- **Font Rendering**: Different font configuration systems affect GPUI
- **HiDPI Support**: Varying HiDPI implementations across desktop environments

## Updated Architecture Changes

### Enhanced Platform Traits
```rust
pub trait PlatformCapabilities {
    fn detect_display_server(&self) -> DisplayServer; // X11, Wayland, etc.
    fn supports_global_hotkeys(&self) -> bool;
    fn requires_compositor_integration(&self) -> bool;
    fn get_preferred_hotkey_method(&self) -> HotkeyMethod;
}

pub trait HotkeyManager {
    fn register_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()>;
    fn register_fallback_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()>;
    fn check_display_server_support(&self) -> bool;
    fn prompt_for_compositor_setup(&self) -> bool;
}

pub trait TerminalLauncher {
    fn launch_command(&self, command: &str, config: &TerminalConfig) -> Result<()>;
    fn bring_to_front(&self, app_name: &str) -> Result<()>;
    fn detect_via_desktop_files(&self) -> Vec<DetectedTerminal>;
    fn handle_desktop_environment(&self, de: &DesktopEnvironment) -> Result<()>;
}
```

## Revised Implementation Phases

### Phase 0: Linux/FreeBSD Feasibility Validation (2-3 days)
1. **GPUI Unix Compatibility**: Verify GPUI rendering on Linux/FreeBSD with various display servers
2. **Hotkey Proof-of-Concept**: Test X11 global hotkey registration across distros
3. **Wayland Limitations**: Document Wayland global hotkey limitations and compositor support
4. **Terminal Detection**: Test desktop file parsing and terminal launching
5. **Performance Baseline**: Establish Unix-specific benchmarking targets

### Phase 1: Platform Abstraction (2-3 days)
1. Create platform trait definitions with display server detection
2. Move macOS code into platform/macos/
3. Add error recovery patterns for Unix-specific failures
4. Update imports and module structure
5. Ensure macOS functionality remains unchanged

### Phase 2a: Linux Hotkey Implementation (3-4 days)
1. Implement X11 hotkey manager using x11rb or xcb
2. Add Wayland compositor detection and fallback mechanisms
3. Handle display server switching scenarios
4. Desktop environment integration for hotkey conflicts
5. Cross-platform hotkey testing framework

### Phase 2b: Linux Terminal Detection (2-3 days)
1. Implement desktop file parsing for terminal detection
2. Package manager integration for terminal discovery
3. Support for custom/compiled terminals
4. Desktop environment terminal preference detection
5. Terminal capability detection (tabbing, profiles, etc.)

### Phase 3: FreeBSD Implementation (2-3 days)
1. Adapt Linux X11 hotkey implementation for FreeBSD
2. FreeBSD ports system integration for terminal detection
3. FreeBSD-specific path and permission handling
4. Testing on FreeBSD desktop environments

### Phase 4: System Integration (3-4 days)
1. Linux system tray implementation with libappindicator
2. D-Bus notification integration
3. XDG desktop file creation for application registration
4. HiDPI and font rendering compatibility testing
5. Desktop environment compatibility matrix

### Phase 5: Packaging & Distribution (3-4 days)
1. AppImage creation for universal Linux distribution
2. Debian/Ubuntu packaging (deb)
3. Red Hat packaging (rpm)
4. Flatpak packaging for sandboxed distribution
5. FreeBSD ports system integration
6. CI/CD for multiple Linux distributions

### Phase 6: Polish & Performance (1-2 days)
1. Unix-specific performance optimization
2. Desktop environment theme integration
3. Accessibility support where available
4. Documentation and troubleshooting guides

## Enhanced Configuration Format
```toml
# GNOME Terminal
[terminals.gnome-terminal]
program = "gnome-terminal"
args = ["--", "{ssh_command}"]
desktop_file = "org.gnome.Terminal.desktop"
detection_paths = ["/usr/bin/gnome-terminal"]

# Konsole
[terminals.konsole]
program = "konsole"
args = ["-e", "{ssh_command}"]
desktop_file = "org.kde.konsole.desktop"
detection_paths = ["/usr/bin/konsole"]

# Alacritty (cross-platform)
[terminals.alacritty]
program = "alacritty"
args = ["-e", "{ssh_command}"]
desktop_file = "Alacritty.desktop"
detection_paths = [
    "/usr/bin/alacritty",
    "/usr/local/bin/alacritty",
    "~/.cargo/bin/alacritty"
]

# Distribution-specific settings
[platform.linux]
prefer_desktop_files = true
fallback_to_path_search = true
respect_desktop_environment_defaults = true

[platform.freebsd]
prefer_ports_detection = true
fallback_to_path_search = true
```

## Risk Mitigation Strategies
- **Display Server Compatibility**: Comprehensive testing matrix for X11/Wayland combinations
- **Terminal Diversity**: Extensible detection system with manual override capabilities
- **Hotkey Limitations**: Clear documentation of Wayland limitations and alternatives
- **Distribution Variance**: Package manager abstraction for consistent detection
- **Performance**: Unix-specific benchmarking with maintained 50ms search target

## Benefits of Enhanced Approach
- **Display Server Agnostic**: Handles both X11 and Wayland environments
- **Distribution Independent**: Works across major Linux distributions and FreeBSD
- **Desktop Environment Aware**: Integrates properly with GNOME, KDE, XFCE, etc.
- **Graceful Degradation**: Handles missing features appropriately
- **Future-Proof**: Extensible for emerging Unix display technologies

This comprehensive plan addresses Unix-specific challenges including display server diversity, terminal ecosystem complexity, and distribution variance while maintaining consistent user experience across platforms.