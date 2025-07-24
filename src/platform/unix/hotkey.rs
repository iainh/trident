// ABOUTME: Unix hotkey manager using X11 XGrabKey and Wayland compositor integration
// ABOUTME: Provides global hotkey support for Linux and FreeBSD with display server detection

use crate::platform::{HotkeyManager, DisplayServer};
use crate::platform::unix::UnixPlatform;
use crate::platform::PlatformCapabilities;
use crate::config::HotkeyConfig;
use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex};

// Type alias for hotkey callback
type HotkeyCallback = Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>;

pub struct UnixHotkeyManager {
    platform: UnixPlatform,
    callback: HotkeyCallback,
    config: HotkeyConfig,
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    x11_connection: Option<Arc<x11rb::connection::Connection>>,
    registered: bool,
}

impl UnixHotkeyManager {
    pub fn new(config: HotkeyConfig) -> Self {
        Self {
            platform: UnixPlatform,
            callback: Arc::new(Mutex::new(None)),
            config,
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            x11_connection: None,
            registered: false,
        }
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn try_x11_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::*;
        use x11rb::rust_connection::ReplyError;

        let (conn, screen_num) = x11rb::connect(None).map_err(|e| 
            anyhow!("Failed to connect to X11 display: {}. Ensure DISPLAY is set.", e)
        )?;
        
        let conn = Arc::new(conn);
        let setup = conn.setup();
        let screen = &setup.roots[screen_num];
        let root = screen.root;

        let (modifiers, key_name) = self.parse_hotkey_combination(&self.config.combination)?;
        let keycode = self.get_keycode_for_key_name(&conn, key_name)?;
        
        tracing::debug!("Attempting to grab X11 hotkey: {} (keycode: {})", self.config.combination, keycode);

        let grab_result = grab_key(
            &*conn,
            false, // owner_events
            root,
            modifiers,
            keycode,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        ).get_reply();

        if let Err(ReplyError::X11Error(ref error)) = grab_result {
            if error.error_code == x11rb::protocol::xproto::BAD_ACCESS {
                return Err(anyhow!("Failed to grab hotkey '{}'. It is likely already in use by another application.", self.config.combination));
            }
        }
        grab_result?;

        self.x11_connection = Some(conn.clone());
        
        {
            let mut cb = self.callback.lock().unwrap();
            *cb = Some(callback);
        }

        let callback_clone = self.callback.clone();
        let conn_clone = conn.clone();
        std::thread::spawn(move || {
            tracing::debug!("X11 hotkey event loop started");
            loop {
                match conn_clone.wait_for_event() {
                    Ok(Event::KeyPress(key_event)) => {
                        if key_event.detail == keycode && key_event.state == modifiers.into() {
                            log::debug!("X11 hotkey triggered: {}", self.config.combination);
                            if let Ok(callback_guard) = callback_clone.lock() {
                                if let Some(ref cb) = *callback_guard {
                                    cb();
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("X11 event loop error: {:?}", e);
                        break;
                    }
                    _ => {}
                }
            }
            tracing::debug!("X11 hotkey event loop ended");
        });

        self.registered = true;
        tracing::info!("Successfully registered X11 global hotkey: {}", self.config.combination);
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn parse_hotkey_combination(&self, combination: &str) -> Result<(ModMask, &str)> {
        use x11rb::protocol::xproto::ModMask;
        let mut modifiers = ModMask::empty();
        let mut key_part = "";

        for part in combination.split('+') {
            match part.to_lowercase().as_str() {
                "shift" => modifiers |= ModMask::SHIFT,
                "control" | "ctrl" => modifiers |= ModMask::CONTROL,
                "alt" => modifiers |= ModMask::M1,
                "super" | "win" | "cmd" => modifiers |= ModMask::M4,
                _ => key_part = part,
            }
        }

        if key_part.is_empty() {
            return Err(anyhow!("Invalid hotkey combination: no key specified"));
        }

        Ok((modifiers, key_part))
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn get_keycode_for_key_name(&self, conn: &impl x11rb::connection::Connection, key_name: &str) -> Result<u8> {
        // This is a simplified mapping. For a full implementation, a library like `xkbcommon` would be better.
        let keysym = match key_name.to_lowercase().as_str() {
            "s" => 0x0073,
            "t" => 0x0074,
            // ... add other keys as needed
            _ => return Err(anyhow!("Unsupported key name: {}", key_name)),
        };

        use x11rb::protocol::xproto::*;
        let min_keycode = conn.setup().min_keycode;
        let max_keycode = conn.setup().max_keycode;
        let keyboard_mapping = get_keyboard_mapping(conn, min_keycode, max_keycode - min_keycode + 1)?.reply()?;
        let keysyms_per_keycode = keyboard_mapping.keysyms_per_keycode as usize;

        for (i, chunk) in keyboard_mapping.keysyms.chunks(keysyms_per_keycode).enumerate() {
            if chunk.contains(&keysym) {
                return Ok(min_keycode + i as u8);
            }
        }

        Err(anyhow!("Could not find keycode for key: {}", key_name))
    }

    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    fn try_x11_hotkey(&mut self, _callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        Err(anyhow!("X11 hotkeys not supported on this platform"))
    }

    fn try_wayland_de_integration(&mut self, _callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        let de = self.detect_desktop_environment();
        let instructions = match de {
            "gnome" => "...", // Instructions for GNOME
            "kde" => "...",   // Instructions for KDE
            _ => "...",       // Generic instructions
        };
        Err(anyhow!(
            "Wayland global hotkeys require manual setup.\n\n{}",
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
        } else if std::env::var("SWAYSOCK").is_ok() {
            "sway"
        } else {
            "unknown"
        }
    }
}

impl HotkeyManager for UnixHotkeyManager {
    fn register_hotkey(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        match self.platform.detect_display_server() {
            DisplayServer::X11 => self.try_x11_hotkey(callback),
            DisplayServer::Wayland => self.try_wayland_de_integration(callback),
            DisplayServer::Unknown => Err(anyhow!("Unknown display server.")),
        }
    }

    fn unregister(&mut self) -> Result<()> {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            if let Some(conn) = self.x11_connection.take() {
                if let Ok(setup) = std::panic::catch_unwind(|| conn.setup()) {
                    if let Some(screen) = setup.roots.first() {
                        let (modifiers, key_name) = self.parse_hotkey_combination(&self.config.combination)?;
                        if let Ok(keycode) = self.get_keycode_for_key_name(&*conn, key_name) {
                            let _ = x11rb::protocol::xproto::ungrab_key(&*conn, keycode, screen.root, modifiers);
                            let _ = conn.flush();
                        }
                    }
                }
                tracing::debug!("X11 hotkey connection closed");
            }
        }
        
        {
            let mut cb = self.callback.lock().unwrap();
            *cb = None;
        }
        
        self.registered = false;
        tracing::info!("Unix hotkey manager unregistered");
        Ok(())
    }
}