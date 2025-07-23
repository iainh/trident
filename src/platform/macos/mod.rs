// ABOUTME: macOS platform implementations for hotkey management, terminal launching, and config detection
// ABOUTME: Wraps existing objc2-based functionality in the platform abstraction traits

pub mod hotkey;
pub mod launcher;
pub mod config;

pub use hotkey::MacOSHotkeyManager;
pub use launcher::MacOSTerminalLauncher;
pub use config::MacOSConfigDetector;

use super::{PlatformCapabilities, DisplayServer, HotkeyMethod};

pub struct MacOSPlatform;

impl PlatformCapabilities for MacOSPlatform {
    fn detect_display_server(&self) -> DisplayServer {
        // macOS always uses Quartz/Cocoa
        DisplayServer::Unknown // We could add a Quartz variant
    }
    
    fn supports_global_hotkeys(&self) -> bool {
        true // macOS has excellent global hotkey support
    }
    
    fn requires_compositor_integration(&self) -> bool {
        false // macOS handles this at the system level
    }
    
    fn get_preferred_hotkey_method(&self) -> HotkeyMethod {
        HotkeyMethod::Native // Use NSEvent
    }
}