// ABOUTME: Unix hotkey manager using X11 XGrabKey and Wayland compositor integration
// ABOUTME: Provides global hotkey support for Linux and FreeBSD with display server detection

use crate::platform::{HotkeyManager, DisplayServer};
use crate::platform::unix::UnixPlatform;
use crate::platform::PlatformCapabilities;
use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex};

// Type alias for hotkey callback
type HotkeyCallback = Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>;

pub struct UnixHotkeyManager {
    platform: UnixPlatform,
    callback: HotkeyCallback,
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    x11_connection: Option<Arc<x11rb::connection::Connection>>,
    registered: bool,
}

impl UnixHotkeyManager {
    pub fn new() -> Self {
        Self {
            platform: UnixPlatform,
            callback: Arc::new(Mutex::new(None)),
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            x11_connection: None,
            registered: false,
        }
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn try_x11_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::*;

        // Connect to X11 display
        let (conn, screen_num) = x11rb::connect(None)
            .map_err(|e| anyhow!("Failed to connect to X11 display: {}", e))?;
        
        let conn = Arc::new(conn);
        let setup = conn.setup();
        let screen = &setup.roots[screen_num];
        let root = screen.root;

        // Define hotkey: Cmd+Shift+S (using Super+Shift+S on Unix)
        // KeySym for 's' is 0x073, Super is Mod4Mask, Shift is ShiftMask
        let modifiers = ModMask::SHIFT | ModMask::M4; // Shift + Super (Windows/Cmd key)
        let keycode = 39; // 's' key on most layouts - this may need runtime detection

        // Grab the key combination globally
        grab_key(
            &*conn,
            false, // owner_events
            root,
            modifiers,
            keycode,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )?;

        self.x11_connection = Some(conn.clone());
        
        // Store the callback
        {
            let mut cb = self.callback.lock().unwrap();
            *cb = Some(callback);
        }

        // Start event loop in background thread
        let callback_clone = self.callback.clone();
        let conn_clone = conn.clone();
        std::thread::spawn(move || {
            loop {
                match conn_clone.wait_for_event() {
                    Ok(event) => {
                        if let Event::KeyPress(key_event) = event {
                            // Check if this is our hotkey
                            if key_event.detail == keycode && 
                               key_event.state == (KeyButMask::SHIFT | KeyButMask::MOD4) {
                                if let Ok(callback_guard) = callback_clone.lock() {
                                    if let Some(ref cb) = *callback_guard {
                                        cb();
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => break, // Connection lost
                }
            }
        });

        self.registered = true;
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    fn try_x11_hotkey(&mut self, _callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        Err(anyhow!("X11 hotkeys not supported on this platform"))
    }

    fn try_wayland_de_integration(&mut self, _callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        // Wayland global hotkeys require compositor-specific integration
        // This would need implementation for each compositor (GNOME Shell, KDE, etc.)
        Err(anyhow!(
            "Wayland global hotkeys not yet implemented. \
             Please configure a compositor-specific hotkey that runs: 'trident --show-launcher'"
        ))
    }
}

impl HotkeyManager for UnixHotkeyManager {
    fn register_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        match self.platform.detect_display_server() {
            DisplayServer::X11 => {
                self.try_x11_hotkey(callback)
            }
            DisplayServer::Wayland => {
                self.try_wayland_de_integration(callback)
            }
            DisplayServer::Unknown => {
                Err(anyhow!(
                    "Unknown display server. Cannot register global hotkeys. \
                     Ensure DISPLAY or WAYLAND_DISPLAY environment variables are set."
                ))
            }
        }
    }

    fn register_fallback_hotkey(&mut self, _callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        // For Unix systems, fallback would be desktop environment specific shortcuts
        println!("To set up hotkeys manually:");
        println!("1. Open your desktop environment's keyboard shortcuts settings");
        println!("2. Add a custom shortcut for Cmd+Shift+S (or Super+Shift+S)");
        println!("3. Set the command to: trident --show-launcher");
        Ok(())
    }

    fn check_display_server_support(&self) -> bool {
        match self.platform.detect_display_server() {
            DisplayServer::X11 => true,
            DisplayServer::Wayland => false, // Limited
            DisplayServer::Unknown => false,
        }
    }

    fn check_permissions(&self) -> bool {
        // Unix doesn't have the same permission model as macOS
        // X11 allows global key grabbing by default
        match self.platform.detect_display_server() {
            DisplayServer::X11 => true,
            _ => false,
        }
    }

    fn prompt_for_permissions(&self) -> bool {
        self.check_permissions()
    }

    fn unregister(&mut self) -> Result<()> {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            if let Some(conn) = &self.x11_connection {
                // Ungrab the key - would need to store keycode and modifiers
                // For now, just clear the connection
                self.x11_connection = None;
            }
        }
        
        self.registered = false;
        Ok(())
    }
}