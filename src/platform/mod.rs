// ABOUTME: Platform abstraction layer providing trait definitions for OS-specific functionality
// ABOUTME: Enables cross-platform implementation of hotkeys, terminal launching, and config detection

use crate::config::{DetectedTerminal, TerminalConfig};
use crate::ssh::HostEntry;
use anyhow::Result;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub mod unix;

/// Platform capabilities detection and feature support
#[allow(dead_code)]
pub trait PlatformCapabilities {
    /// Detect the current display server (X11, Wayland, etc.)
    fn detect_display_server(&self) -> DisplayServer;

    /// Check if global hotkeys are supported on this platform
    fn supports_global_hotkeys(&self) -> bool;

    /// Check if compositor integration is required for hotkeys
    fn requires_compositor_integration(&self) -> bool;

    /// Get the preferred hotkey implementation method
    fn get_preferred_hotkey_method(&self) -> HotkeyMethod;
}

/// Display server types for Unix systems
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum DisplayServer {
    X11,
    Wayland,
    Unknown,
}

/// Hotkey implementation methods
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum HotkeyMethod {
    Native,    // macOS NSEvent
    X11Global, // X11 XGrabKey
    WaylandDE, // Desktop environment integration
    Fallback,  // Manual setup required
}

/// Global hotkey management trait
#[allow(dead_code)]
pub trait HotkeyManager {
    /// Register a global hotkey with a callback
    fn register_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()>;

    /// Register a fallback hotkey mechanism
    fn register_fallback_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()>;

    /// Check if the display server supports hotkeys
    fn check_display_server_support(&self) -> bool;

    /// Prompt for any required permissions or setup
    fn check_permissions(&self) -> bool;

    /// Prompt for permissions if needed
    fn prompt_for_permissions(&self) -> bool;

    /// Unregister the hotkey
    fn unregister(&mut self) -> Result<()>;
}

/// Terminal launching trait
#[allow(dead_code)]
pub trait TerminalLauncher {
    /// Launch a command in the configured terminal
    fn launch_command(&self, command: &str, config: &TerminalConfig) -> Result<()>;

    /// Bring the terminal application to front
    fn bring_to_front(&self, app_name: &str) -> Result<()>;

    /// Launch an SSH connection to a specific host
    fn launch_host(&self, host: &HostEntry) -> Result<()>;
}

/// Terminal and configuration detection trait
#[allow(dead_code)]
pub trait ConfigDetector {
    /// Detect available terminals on the system
    fn detect_terminals(&self) -> Vec<DetectedTerminal>;

    /// Get default SSH file paths for this platform
    fn get_default_ssh_paths(&self) -> SshPaths;

    /// Detect via desktop files (Linux-specific)
    fn detect_via_desktop_files(&self) -> Vec<DetectedTerminal>;

    /// Handle desktop environment specific detection
    fn handle_desktop_environment(&self, de: &DesktopEnvironment) -> Result<()>;
}

/// SSH file paths for the platform
#[derive(Debug, Clone, PartialEq)]
pub struct SshPaths {
    pub known_hosts_path: String,
    pub config_path: String,
    pub ssh_binary: String,
}

/// Desktop environments for Unix systems
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum DesktopEnvironment {
    Gnome,
    Kde,
    Xfce,
    I3,
    Sway,
    Unknown,
}

/// Platform factory to get the appropriate implementations
pub struct Platform;

impl Platform {
    /// Get the platform-specific hotkey manager
    #[cfg(target_os = "macos")]
    pub fn hotkey_manager() -> Box<dyn HotkeyManager> {
        Box::new(macos::MacOSHotkeyManager::new())
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub fn hotkey_manager() -> Box<dyn HotkeyManager> {
        Box::new(unix::UnixHotkeyManager::new())
    }

    /// Get the platform-specific terminal launcher
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    pub fn terminal_launcher(config: TerminalConfig) -> Box<dyn TerminalLauncher> {
        Box::new(macos::MacOSTerminalLauncher::new(config))
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    #[allow(dead_code)]
    pub fn terminal_launcher(config: TerminalConfig) -> Box<dyn TerminalLauncher> {
        Box::new(unix::UnixTerminalLauncher::new(config))
    }

    /// Get the platform-specific config detector
    #[cfg(target_os = "macos")]
    pub fn config_detector() -> Box<dyn ConfigDetector> {
        Box::new(macos::MacOSConfigDetector::new())
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub fn config_detector() -> Box<dyn ConfigDetector> {
        Box::new(unix::UnixConfigDetector::new())
    }
}
