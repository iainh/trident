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
            .map_err(|e| anyhow!("Failed to connect to X11 display: {}. Ensure DISPLAY is set and X11 is running.", e))?;
        
        let conn = Arc::new(conn);
        let setup = conn.setup();
        let screen = &setup.roots[screen_num];
        let root = screen.root;

        // Get the keycode for 's' dynamically
        let keycode = self.get_keycode_for_keysym(&conn, 0x073)?; // 's' keysym
        
        // Define hotkey: Super+Shift+S on Unix (equivalent to Cmd+Shift+S on macOS)
        let modifiers = ModMask::SHIFT | ModMask::M4; // Shift + Super (Windows/Cmd key)

        println!("[DEBUG] Attempting to grab X11 hotkey: Super+Shift+S (keycode: {})", keycode);

        // Grab the key combination globally
        let grab_result = grab_key(
            &*conn,
            false, // owner_events
            root,
            modifiers,
            keycode,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        );
        
        // Flush to ensure the grab request is sent
        conn.flush()?;
        
        // Check if grab was successful
        if let Err(e) = grab_result {
            return Err(anyhow!("Failed to grab hotkey. Another application may be using Super+Shift+S: {:?}", e));
        }

        self.x11_connection = Some(conn.clone());
        
        // Store the callback
        {
            let mut cb = self.callback.lock().unwrap();
            *cb = Some(callback);
        }

        // Start event loop in background thread
        let callback_clone = self.callback.clone();
        let conn_clone = conn.clone();
        let expected_keycode = keycode;
        std::thread::spawn(move || {
            println!("[DEBUG] X11 hotkey event loop started");
            loop {
                match conn_clone.wait_for_event() {
                    Ok(event) => {
                        if let Event::KeyPress(key_event) = event {
                            // Check if this is our hotkey
                            let has_shift = key_event.state & u16::from(ModMask::SHIFT.bits()) != 0;
                            let has_super = key_event.state & u16::from(ModMask::M4.bits()) != 0;
                            
                            if key_event.detail == expected_keycode && has_shift && has_super {
                                println!("[DEBUG] X11 hotkey triggered: Super+Shift+S");
                                if let Ok(callback_guard) = callback_clone.lock() {
                                    if let Some(ref cb) = *callback_guard {
                                        cb();
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("[DEBUG] X11 event loop error: {:?}", e);
                        break; // Connection lost
                    }
                }
            }
            println!("[DEBUG] X11 hotkey event loop ended");
        });

        self.registered = true;
        println!("[INFO] Successfully registered X11 global hotkey: Super+Shift+S");
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn get_keycode_for_keysym(&self, conn: &impl x11rb::connection::Connection, keysym: u32) -> Result<u8> {
        use x11rb::protocol::xproto::*;
        
        // Get the keyboard mapping
        let min_keycode = conn.setup().min_keycode;
        let max_keycode = conn.setup().max_keycode;
        
        let keyboard_mapping = get_keyboard_mapping(
            conn,
            min_keycode,
            max_keycode - min_keycode + 1,
        )?.reply()?;
        
        // Search for the keysym in the mapping
        let keysyms_per_keycode = keyboard_mapping.keysyms_per_keycode;
        
        for (i, chunk) in keyboard_mapping.keysyms.chunks(keysyms_per_keycode as usize).enumerate() {
            if chunk.iter().any(|&sym| sym == keysym) {
                return Ok(min_keycode + i as u8);
            }
        }
        
        Err(anyhow!("Could not find keycode for keysym 0x{:x}", keysym))
    }

    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    fn try_x11_hotkey(&mut self, _callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        Err(anyhow!("X11 hotkeys not supported on this platform"))
    }

    fn try_wayland_de_integration(&mut self, _callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        // Wayland global hotkeys require compositor-specific integration
        // Detect desktop environment and provide specific instructions
        let de = self.detect_desktop_environment();
        
        let instructions = match de {
            "gnome" => {
                "GNOME Shell detected. To set up the hotkey:\n\
                 1. Open Settings > Keyboard > Keyboard Shortcuts\n\
                 2. Add a custom shortcut with command: trident --show-launcher\n\
                 3. Set the key combination to Super+Shift+S"
            }
            "kde" => {
                "KDE Plasma detected. To set up the hotkey:\n\
                 1. Open System Settings > Shortcuts > Custom Shortcuts\n\
                 2. Add a new shortcut with command: trident --show-launcher\n\
                 3. Set the key combination to Meta+Shift+S"
            }
            "sway" => {
                "Sway compositor detected. Add to your ~/.config/sway/config:\n\
                 bindsym $mod+Shift+s exec trident --show-launcher"
            }
            _ => {
                "Wayland compositor detected. Global hotkeys require manual setup:\n\
                 Configure your desktop environment to run 'trident --show-launcher' on Super+Shift+S"
            }
        };

        Err(anyhow!(
            "Wayland global hotkeys require compositor integration.\n\n{}\n\
             For now, use the tray icon to access the launcher.", 
            instructions
        ))
    }

    fn detect_desktop_environment(&self) -> &'static str {
        if let Ok(de) = std::env::var("XDG_CURRENT_DESKTOP") {
            match de.to_lowercase().as_str() {
                "gnome" | "ubuntu:gnome-shell" => "gnome",
                "kde" | "plasma" => "kde",
                "sway" => "sway",
                "xfce" => "xfce",
                _ => "unknown",
            }
        } else if let Ok(_) = std::env::var("SWAYSOCK") {
            "sway"
        } else {
            "unknown"
        }
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
        let de = self.detect_desktop_environment();
        
        println!("Manual hotkey setup required for your desktop environment:");
        match de {
            "gnome" => {
                println!("GNOME Shell:");
                println!("1. Open Settings > Keyboard > Keyboard Shortcuts > Custom Shortcuts");
                println!("2. Click '+' to add a new shortcut");
                println!("3. Name: 'Trident SSH Launcher'");
                println!("4. Command: trident --show-launcher");  
                println!("5. Click 'Set Shortcut' and press Super+Shift+S");
            }
            "kde" => {
                println!("KDE Plasma:");
                println!("1. Open System Settings > Shortcuts");
                println!("2. Click 'Custom Shortcuts' > 'Edit' > 'New' > 'Global Shortcut' > 'Command/URL'");
                println!("3. Name: 'Trident SSH Launcher'"); 
                println!("4. Command: trident --show-launcher");
                println!("5. Set trigger to Meta+Shift+S");
            }
            "xfce" => {
                println!("XFCE:");
                println!("1. Open Settings > Keyboard > Application Shortcuts");
                println!("2. Click 'Add' button");
                println!("3. Command: trident --show-launcher");
                println!("4. Press Super+Shift+S when prompted");
            }
            "sway" => {
                println!("Sway (add to ~/.config/sway/config):");
                println!("bindsym $mod+Shift+s exec trident --show-launcher");
                println!("Then reload config with: swaymsg reload");
            }
            _ => {
                println!("Generic instructions:");
                println!("1. Open your desktop environment's keyboard shortcuts settings");
                println!("2. Add a custom shortcut for Super+Shift+S");
                println!("3. Set the command to: trident --show-launcher");
            }
        }
        println!("\nAfter setting up the hotkey, restart Trident to use it alongside the tray icon.");
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
            if let Some(conn) = self.x11_connection.take() {
                // Best effort to ungrab the key
                // In practice, the connection closing will release the grab
                if let Ok(setup) = std::panic::catch_unwind(|| conn.setup()) {
                    if let Some(screen) = setup.roots.first() {
                        // Try to ungrab - this may fail if the connection is already closed
                        let _ = x11rb::protocol::xproto::ungrab_key(
                            &*conn,
                            x11rb::protocol::xproto::GRAB_ANY,
                            screen.root,
                            x11rb::protocol::xproto::ModMask::ANY,
                        );
                        let _ = conn.flush();
                    }
                }
                println!("[DEBUG] X11 hotkey connection closed");
            }
        }
        
        // Clear the callback
        {
            let mut cb = self.callback.lock().unwrap();
            *cb = None;
        }
        
        self.registered = false;
        println!("[INFO] Unix hotkey manager unregistered");
        Ok(())
    }
}