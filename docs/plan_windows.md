# Windows Port Plan for Trident SSH Launcher

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
│   └── windows/        # Windows implementations
│       ├── mod.rs
│       ├── hotkey.rs   # Windows global hotkey (winapi/windows-rs)
│       ├── launcher.rs # Windows terminal launching
│       └── config.rs   # Windows terminal detection
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

### 3. Windows-Specific Implementation

#### Hotkey Management (src/platform/windows/hotkey.rs)
- Use `windows-rs` or `winapi` crate for `RegisterHotKey` API
- Handle `WM_HOTKEY` messages
- No accessibility permissions needed on Windows

#### Terminal Launching (src/platform/windows/launcher.rs)
- Detect Windows terminals: Windows Terminal, PowerShell, cmd, WSL
- Use `CreateProcess` or `std::process::Command` for launching
- Handle Windows-specific path formats and escaping
- Window activation via Windows API

#### Configuration (src/platform/windows/config.rs)
- Detect installed terminals from common paths:
  - Windows Terminal: `%LOCALAPPDATA%\Microsoft\WindowsApps\wt.exe`
  - PowerShell: `powershell.exe`, `pwsh.exe`
  - WSL: `wsl.exe`
- Windows SSH paths: `%USERPROFILE%\.ssh\`

### 4. Cargo.toml Changes
```toml
[target.'cfg(target_os = "macos")'.dependencies]
# Existing macOS deps...

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_System_Threading", 
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_WindowsAndMessaging"
]}
```

### 5. Build System Updates
- Update `build.rs` for Windows-specific linking if needed
- Create Windows packaging script (MSI/installer)
- Update `flake.nix` to support cross-compilation
- Add Windows CI/CD pipeline

### 6. Configuration Format Changes
Extend config.toml to support Windows terminals:
```toml
# Windows Terminal
program = "wt.exe"
args = ["--", "{ssh_command}"]

# PowerShell 
program = "powershell.exe"
args = ["-Command", "{ssh_command}"]

# WSL
program = "wsl.exe" 
args = ["-e", "bash", "-c", "{ssh_command}"]
```

## Implementation Phases

### Phase 1: Platform Abstraction (2-3 days)
1. Create platform trait definitions
2. Move macOS code into platform/macos/
3. Update imports and module structure
4. Ensure macOS functionality remains unchanged

### Phase 2: Windows Core Implementation (3-4 days)
1. Implement Windows hotkey manager
2. Implement Windows terminal launcher
3. Implement Windows config detection
4. Add Windows-specific dependencies

### Phase 3: Integration & Testing (2-3 days)
1. Update main.rs for platform selection
2. Cross-platform testing framework
3. Windows packaging and distribution
4. Documentation updates

### Phase 4: Polish & CI (1-2 days)
1. Windows installer creation
2. CI/CD for Windows builds
3. Performance optimization
4. User experience refinements

## Benefits of This Approach
- **Clean Separation**: Platform code isolated from business logic
- **Maintainable**: Easy to add future platforms (Linux)
- **Testable**: Platform traits can be mocked for testing
- **Consistent**: Same user experience across platforms
- **Extensible**: Easy to add platform-specific features

## Critical Windows-Specific Edge Cases

### Security & Distribution
- **Code Signing**: Windows requires code signing to avoid SmartScreen warnings and ensure user trust
- **UAC Elevation**: Global hotkey registration may require elevated privileges or specific manifest declarations
- **Windows Defender**: SSH key access may trigger antivirus scanning; consider exclusion guidance
- **Registry Integration**: Proper Windows app registration for system integration

### Extended Terminal Ecosystem
Beyond the basic terminals, Windows supports:
- **Modern Terminals**: Alacritty, kitty, ConEmu, Cmder, Hyper, Tabby
- **WSL Variants**: WSL1 vs WSL2 compatibility differences and detection
- **Portable Installations**: Non-registry terminal installations in custom paths
- **SSH Agent Diversity**: Pageant, 1Password SSH agent, OpenSSH for Windows

### Windows-Specific UX Challenges
- **DPI Scaling**: GPUI high-DPI display support validation required
- **Hotkey Conflicts**: Many Windows apps use Ctrl+Shift+S (consider alternative: Ctrl+Shift+T)
- **Notification System**: Windows notification integration for connection status
- **Window Management**: Different focus/activation behavior compared to macOS

## Updated Architecture Changes

### Enhanced Platform Traits
```rust
pub trait PlatformCapabilities {
    fn check_hotkey_permissions(&self) -> PermissionStatus;
    fn check_ssh_access_permissions(&self) -> PermissionStatus;
    fn requires_elevation(&self) -> bool;
    fn can_register_global_hotkey(&self) -> bool;
}

pub trait HotkeyManager {
    fn register_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()>;
    fn register_fallback_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()>;
    fn check_permissions(&self) -> bool;
    fn prompt_for_permissions(&self) -> bool;
}

pub trait TerminalLauncher {
    fn launch_command(&self, command: &str, config: &TerminalConfig) -> Result<()>;
    fn bring_to_front(&self, app_name: &str) -> Result<()>;
    fn recover_from_failure(&self, error: &LaunchError) -> Result<()>;
}
```

## Revised Implementation Phases

### Phase 0: Windows Feasibility Validation (1-2 days)
1. **GPUI Windows Compatibility**: Create minimal Windows GPUI app to verify rendering and input
2. **Hotkey Proof-of-Concept**: Basic global hotkey registration without elevation
3. **Terminal Launch Test**: Simple command execution with major Windows terminals
4. **Performance Baseline**: Establish Windows-specific benchmarking targets

### Phase 1: Platform Abstraction (2-3 days)
1. Create platform trait definitions with capabilities checking
2. Move macOS code into platform/macos/
3. Add error recovery patterns for platform-specific failures
4. Update imports and module structure
5. Ensure macOS functionality remains unchanged

### Phase 2a: Windows Core Hotkey (2-3 days)
1. Implement Windows hotkey manager with fallback mechanisms
2. Handle UAC and permission requirements
3. Add alternative hotkey options for conflict resolution
4. Cross-platform hotkey testing framework

### Phase 2b: Windows Terminal Detection (2-3 days)
1. Implement comprehensive terminal detection (registry + filesystem)
2. Support for portable and modern terminal installations
3. WSL1/WSL2 compatibility handling
4. SSH agent detection and integration

### Phase 3: Integration & Platform Testing (2-3 days)
1. Update main.rs for platform selection with capability checking
2. Windows-specific error handling and recovery
3. DPI scaling and display compatibility testing
4. Windows packaging and distribution setup

### Phase 4: Security & Distribution (2-3 days)
1. Code signing implementation and certificate management
2. Windows installer creation with proper registry integration
3. SmartScreen and Windows Defender compatibility
4. CI/CD for Windows builds with security validation

### Phase 5: Polish & Performance (1-2 days)
1. Windows notification system integration
2. Performance optimization for Windows-specific bottlenecks
3. User experience refinements and accessibility
4. Documentation updates and troubleshooting guides

## Enhanced Configuration Format
```toml
# Windows Terminal
program = "wt.exe"
args = ["--", "{ssh_command}"]
detection_paths = [
    "%LOCALAPPDATA%\\Microsoft\\WindowsApps\\wt.exe",
    "%PROGRAMFILES%\\WindowsApps\\Microsoft.WindowsTerminal_*\\wt.exe"
]

# Modern terminals
[terminals.alacritty]
program = "alacritty.exe"
args = ["-e", "{ssh_command}"]
detection_paths = [
    "%APPDATA%\\alacritty\\alacritty.exe",
    "%PROGRAMFILES%\\Alacritty\\alacritty.exe"
]

# WSL with version detection
[terminals.wsl]
program = "wsl.exe"
args = ["-e", "bash", "-c", "{ssh_command}"]
requires_wsl_check = true
```

## Risk Mitigation Strategies
- **GPUI Compatibility**: Validate in Phase 0 before architecture changes
- **Performance**: Windows-specific benchmarking with 50ms search target maintained
- **Permissions**: Design graceful degradation when elevated privileges unavailable
- **Terminal Diversity**: Extensible detection system with user override capabilities
- **Security**: Code signing pipeline and security best practices from project start

## Benefits of Enhanced Approach
- **Validated Feasibility**: Phase 0 prevents costly architecture mistakes
- **Security-First**: Built-in consideration for Windows security model
- **Comprehensive Terminal Support**: Covers modern Windows development ecosystem
- **Graceful Degradation**: Handles permission and capability limitations
- **Future-Proof**: Extensible for emerging Windows terminal technologies

This enhanced plan addresses Windows-specific security requirements, covers the diverse terminal ecosystem, and includes proper validation phases to ensure successful cross-platform deployment.