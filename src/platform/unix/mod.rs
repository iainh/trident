// ABOUTME: Unix platform implementations for Linux and FreeBSD support
// ABOUTME: Provides X11/Wayland hotkey management, Unix terminal detection, and desktop integration

pub mod config;
pub mod hotkey;
pub mod launcher;

pub use config::UnixConfigDetector;
pub use hotkey::UnixHotkeyManager;
pub use launcher::UnixTerminalLauncher;

use super::{DisplayServer, HotkeyMethod, PlatformCapabilities};

pub struct UnixPlatform;

impl PlatformCapabilities for UnixPlatform {
    fn detect_display_server(&self) -> DisplayServer {
        // Check environment variables to detect display server
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            DisplayServer::Wayland
        } else if std::env::var("DISPLAY").is_ok() {
            DisplayServer::X11
        } else {
            DisplayServer::Unknown
        }
    }

    fn supports_global_hotkeys(&self) -> bool {
        match self.detect_display_server() {
            DisplayServer::X11 => true,
            DisplayServer::Wayland => false, // Limited support in Wayland
            DisplayServer::Unknown => false,
        }
    }

    fn requires_compositor_integration(&self) -> bool {
        match self.detect_display_server() {
            DisplayServer::Wayland => true,
            _ => false,
        }
    }

    fn get_preferred_hotkey_method(&self) -> HotkeyMethod {
        match self.detect_display_server() {
            DisplayServer::X11 => HotkeyMethod::X11Global,
            DisplayServer::Wayland => HotkeyMethod::WaylandDE,
            DisplayServer::Unknown => HotkeyMethod::Fallback,
        }
    }
}
