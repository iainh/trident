// ABOUTME: Native macOS global hotkey registration using objc2 and NSEvent
// ABOUTME: Provides single-process system-wide hotkey capture with main thread callbacks

use anyhow::{anyhow, Result};
use std::sync::{Arc, Mutex};

#[cfg(target_os = "macos")]
use objc2_app_kit::{NSEvent, NSEventType, NSEventModifierFlags, NSEventMask};
#[cfg(target_os = "macos")]
use objc2_foundation::MainThreadMarker;
#[cfg(target_os = "macos")]
use objc2::{runtime::AnyObject};
#[cfg(target_os = "macos")]
use block2::RcBlock;
use std::ptr::NonNull;

// Link to ApplicationServices framework for accessibility permissions
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
}

// Global callback storage for the NSEvent monitor
static GLOBAL_HOTKEY_CALLBACK: Mutex<Option<Arc<dyn Fn() + Send + Sync>>> = 
    Mutex::new(None);

pub struct NativeHotKeyManager {
    #[cfg(target_os = "macos")]
    event_monitor: Option<objc2::rc::Retained<AnyObject>>, // NSEvent monitor reference
    callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
}

impl NativeHotKeyManager {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "macos")]
            event_monitor: None,
            callback: Arc::new(Mutex::new(None)),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn prompt_for_accessibility_if_needed(&self) -> bool {
        self.check_and_prompt_for_accessibility_permissions()
    }

    #[cfg(not(target_os = "macos"))]
    pub fn prompt_for_accessibility_if_needed(&self) -> bool {
        false // No accessibility permissions needed on non-macOS
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

        // Set the global callback for the NSEvent monitor
        {
            let mut global_callback = GLOBAL_HOTKEY_CALLBACK.lock().unwrap();
            *global_callback = Some(callback_arc);
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub fn register_cmd_shift_s(&mut self) -> Result<()> {
        // Check if accessibility is enabled and prompt if needed
        if !self.check_and_prompt_for_accessibility_permissions() {
            return Err(anyhow!(
                "Accessibility permissions required for global hotkeys. \
                 Please enable accessibility access for Trident in System Settings > \
                 Privacy & Security > Accessibility and restart Trident"
            ));
        }

        unsafe {
            let _mtm = MainThreadMarker::new_unchecked();
            
            // Create a block that will handle NSEvent callbacks
            let handler = RcBlock::new(|event: NonNull<NSEvent>| {
                let event = event.as_ref();
                
                // Check if this is a key down event
                if event.r#type() == NSEventType::KeyDown {
                    // Get the key code and modifiers
                    let key_code = event.keyCode();
                    let modifier_flags = event.modifierFlags();
                    
                    // Check for Cmd+Shift+S (keyCode 1 = S)
                    let cmd_flag = NSEventModifierFlags::Command;
                    let shift_flag = NSEventModifierFlags::Shift;
                    let expected_modifiers = cmd_flag | shift_flag;
                    
                    if key_code == 1 && modifier_flags.contains(expected_modifiers) {
                        println!("[DEBUG] objc2_hotkey: Cmd+Shift+S detected via NSEvent monitor");
                        
                        // Trigger the callback on main thread
                        if let Ok(callback_guard) = GLOBAL_HOTKEY_CALLBACK.lock() {
                            if let Some(ref callback) = *callback_guard {
                                println!("[DEBUG] objc2_hotkey: Executing callback");
                                callback();
                            }
                        }
                        
                        // Note: Global monitors cannot consume events - that's why we get double triggering
                        // We need to use local monitor for event consumption
                    }
                }
            });

            // Register the global event monitor for key down events
            let mask = NSEventMask::KeyDown;
            
            let monitor = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &handler);
            
            match monitor {
                Some(monitor_obj) => {
                    self.event_monitor = Some(monitor_obj);
                    println!("[INFO] Registered native global hotkey monitor for Cmd+Shift+S");
                    Ok(())
                }
                None => {
                    Err(anyhow!(
                        "Failed to register global event monitor. \
                         Please ensure accessibility permissions are granted in System Settings > \
                         Privacy & Security > Accessibility"
                    ))
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn check_accessibility_permissions(&self) -> bool {
        // Check if we have accessibility permissions using AXIsProcessTrusted
        unsafe {
            AXIsProcessTrusted()
        }
    }

    #[cfg(target_os = "macos")]
    pub fn check_and_prompt_for_accessibility_permissions(&self) -> bool {
        // First check if we already have permissions
        if self.check_accessibility_permissions() {
            return true;
        }
        
        println!("[INFO] Accessibility permissions not granted.");
        println!("[INFO] ⚠️  To enable global hotkey (Cmd+Shift+S):");
        println!("[INFO]    1. Open System Settings > Privacy & Security > Accessibility");
        println!("[INFO]    2. Click the lock icon to make changes (enter your password)");
        println!("[INFO]    3. Click the + button and add Trident to the list");
        println!("[INFO]    4. Enable the checkbox next to Trident");
        println!("[INFO]    5. Restart Trident");
        println!("[INFO] 🖱️  For now, use the tray icon (ψ) to access the launcher");
        
        // Return false since we don't have permissions yet
        false
    }

    #[cfg(not(target_os = "macos"))]
    pub fn register_cmd_shift_s(&mut self) -> Result<()> {
        println!("[INFO] Native global hotkeys only supported on macOS");
        println!("[INFO] Falling back to process spawning approach");
        Err(anyhow!("Native hotkeys not supported on this platform"))
    }

    #[cfg(target_os = "macos")]
    pub fn unregister(&mut self) -> Result<()> {
        if let Some(monitor) = self.event_monitor.take() {
            unsafe {
                NSEvent::removeMonitor(&monitor);
            }
            println!("[INFO] Unregistered native global hotkey monitor");
        }
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn unregister(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Drop for NativeHotKeyManager {
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
    fn test_native_hotkey_manager_creation() {
        let manager = NativeHotKeyManager::new();
        assert!(manager.callback.lock().unwrap().is_none());
    }

    #[test]
    fn test_set_callback() {
        let mut manager = NativeHotKeyManager::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();
        
        manager.set_callback(move || {
            called_clone.store(true, Ordering::SeqCst);
        }).unwrap();
        
        assert!(manager.callback.lock().unwrap().is_some());
    }

    #[test]
    fn test_register_unregister() {
        let mut manager = NativeHotKeyManager::new();
        
        // Set a dummy callback first
        manager.set_callback(|| {}).unwrap();
        
        // Registration may fail if not on macOS or permissions not granted
        let result = manager.register_cmd_shift_s();
        
        // If registration succeeded, unregistration should also work
        if result.is_ok() {
            assert!(manager.unregister().is_ok());
        }
        // If registration failed, that's also acceptable (permissions, platform, etc.)
    }
}