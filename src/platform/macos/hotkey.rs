// ABOUTME: macOS hotkey manager implementation using objc2 and NSEvent
// ABOUTME: Wraps the existing NativeHotKeyManager in the platform abstraction trait

use crate::objc2_hotkey::NativeHotKeyManager;
use crate::platform::HotkeyManager;
use anyhow::Result;

pub struct MacOSHotkeyManager {
    native_manager: NativeHotKeyManager,
}

impl MacOSHotkeyManager {
    pub fn new() -> Self {
        Self {
            native_manager: NativeHotKeyManager::new(),
        }
    }
}

impl HotkeyManager for MacOSHotkeyManager {
    fn register_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        // Convert boxed callback to the format expected by NativeHotKeyManager
        self.native_manager.set_callback(callback)?;
        self.native_manager.register_cmd_shift_s()
    }

    fn register_fallback_hotkey(&mut self, _callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        // macOS doesn't need fallback - native hotkeys work well
        Ok(())
    }

    fn check_display_server_support(&self) -> bool {
        true // macOS always supports hotkeys
    }

    fn check_permissions(&self) -> bool {
        // Use the existing accessibility check
        self.native_manager.prompt_for_accessibility_if_needed()
    }

    fn prompt_for_permissions(&self) -> bool {
        self.native_manager.prompt_for_accessibility_if_needed()
    }

    fn unregister(&mut self) -> Result<()> {
        self.native_manager.unregister()
    }
}
