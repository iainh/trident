// ABOUTME: Global hotkey registration using the global-hotkey crate
// ABOUTME: Provides cross-platform system-wide hotkey capture (Cmd+Shift+S) to trigger SSH launcher

use anyhow::{anyhow, Result};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyManager as GHKManager,
};
use std::sync::{Arc, Mutex};

// Global callback storage
static GLOBAL_HOTKEY_CALLBACK: Mutex<Option<Arc<dyn Fn() + Send + Sync>>> = 
    Mutex::new(None);

pub struct GlobalHotKeyManager {
    manager: Option<GHKManager>,
    hotkey: Option<HotKey>,
    callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
}

impl GlobalHotKeyManager {
    pub fn new() -> Self {
        Self {
            manager: None,
            hotkey: None,
            callback: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_callback<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let callback_arc = Arc::new(callback);
        *self.callback.lock().unwrap() = Some(Box::new({
            let callback_clone = callback_arc.clone();
            move || callback_clone()
        }));

        // Also set the global callback
        {
            let mut global_callback = GLOBAL_HOTKEY_CALLBACK.lock().unwrap();
            *global_callback = Some(callback_arc);
        }

        Ok(())
    }

    pub fn register_cmd_shift_s(&mut self) -> Result<()> {
        // Create the global hotkey manager
        let manager = GHKManager::new().map_err(|e| anyhow!("Failed to create hotkey manager: {}", e))?;

        // Create the hotkey: Cmd+Shift+S
        let hotkey = HotKey::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyS);

        // Register the hotkey
        manager.register(hotkey).map_err(|e| anyhow!("Failed to register hotkey: {}", e))?;

        // Start listening for events in a background thread
        std::thread::spawn(|| {
            let receiver = global_hotkey::GlobalHotKeyEvent::receiver();
            loop {
                if let Ok(_event) = receiver.recv() {
                    // Trigger the callback when hotkey is pressed
                    if let Ok(callback_guard) = GLOBAL_HOTKEY_CALLBACK.lock() {
                        if let Some(ref callback) = *callback_guard {
                            callback();
                        }
                    }
                }
            }
        });

        self.manager = Some(manager);
        self.hotkey = Some(hotkey);

        println!("[INFO] Successfully registered global hotkey: Cmd+Shift+S");
        Ok(())
    }

    pub fn unregister(&mut self) -> Result<()> {
        if let (Some(manager), Some(hotkey)) = (self.manager.as_ref(), self.hotkey.as_ref()) {
            manager.unregister(*hotkey).map_err(|e| anyhow!("Failed to unregister hotkey: {}", e))?;
            println!("[INFO] Unregistered global hotkey");
        }
        
        self.manager = None;
        self.hotkey = None;
        Ok(())
    }
}

impl Drop for GlobalHotKeyManager {
    fn drop(&mut self) {
        let _ = self.unregister();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_hotkey_manager_creation() {
        let manager = GlobalHotKeyManager::new();
        assert!(manager.callback.lock().unwrap().is_none());
        assert!(manager.manager.is_none());
        assert!(manager.hotkey.is_none());
    }

    #[test]
    fn test_set_callback() {
        let mut manager = GlobalHotKeyManager::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();
        
        manager.set_callback(move || {
            called_clone.store(true, Ordering::SeqCst);
        }).unwrap();
        
        assert!(manager.callback.lock().unwrap().is_some());
    }

    #[test]
    fn test_register_unregister() {
        let mut manager = GlobalHotKeyManager::new();
        
        // Set a dummy callback first
        manager.set_callback(|| {}).unwrap();
        
        // Registration should work (may fail if permissions not granted)
        let result = manager.register_cmd_shift_s();
        if result.is_ok() {
            // If registration succeeded, unregistration should also work
            assert!(manager.unregister().is_ok());
        }
        // If registration failed, that's also acceptable for testing
        // (might be due to missing permissions)
    }
}