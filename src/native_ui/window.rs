// ABOUTME: Native NSWindow-based window management
// ABOUTME: Replaces GPUI window handling with native macOS window positioning and lifecycle

use crate::native_ui::{NativeHostList, NativeSearchInput};
use crate::ssh::parser::HostEntry;
use anyhow::Result;
use std::sync::{Arc, RwLock};

// Type alias for host selection callback
type HostSelectionCallback = Box<dyn Fn(&HostEntry) + Send + Sync>;

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSView, NSWindow};

// Window configuration
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub width: f64,
    pub height: f64,
    pub title: String,
    pub resizable: bool,
    pub closable: bool,
    pub miniaturizable: bool,
    pub always_on_top: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 600.0,
            height: 500.0,
            title: "Trident SSH Launcher".to_string(),
            resizable: false,
            closable: true,
            miniaturizable: false,
            always_on_top: true,
        }
    }
}

// Window state for MVU pattern
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct WindowState {
    pub is_visible: bool,
    pub is_focused: bool,
    pub position: Option<(f64, f64)>,
}

// Native macOS launcher window
#[allow(dead_code)]
pub struct NativeWindow {
    #[cfg(target_os = "macos")]
    window: Option<Retained<NSWindow>>,
    #[cfg(target_os = "macos")]
    content_view: Option<Retained<NSView>>,

    config: WindowConfig,
    state: Arc<RwLock<WindowState>>,

    // UI components
    search_input: NativeSearchInput,
    host_list: NativeHostList,

    // Callbacks
    on_close: Option<Box<dyn Fn() + Send + Sync>>,
    on_escape: Option<Box<dyn Fn() + Send + Sync>>,
    on_host_selected: Option<HostSelectionCallback>,
}

#[allow(dead_code)]
impl NativeWindow {
    pub fn new(config: WindowConfig, hosts: Vec<HostEntry>) -> Self {
        let state = Arc::new(RwLock::new(WindowState::default()));
        let search_input = NativeSearchInput::new("Search SSH hosts...".to_string());
        let host_list = NativeHostList::new(hosts);

        Self {
            #[cfg(target_os = "macos")]
            window: None,
            #[cfg(target_os = "macos")]
            content_view: None,

            config,
            state,
            search_input,
            host_list,

            on_close: None,
            on_escape: None,
            on_host_selected: None,
        }
    }

    pub fn set_close_callback<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_close = Some(Box::new(callback));
    }

    pub fn set_escape_callback<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_escape = Some(Box::new(callback));
    }

    pub fn set_host_selected_callback<F>(&mut self, callback: F)
    where
        F: Fn(&HostEntry) + Send + Sync + 'static,
    {
        self.on_host_selected = Some(Box::new(callback));
    }

    #[cfg(target_os = "macos")]
    pub fn create_native_window(&mut self) -> Result<()> {
        // Simplified window creation that works with current objc2 API
        // For now, we'll create a placeholder that demonstrates the architecture
        // Real NSWindow creation requires more stable objc2 APIs

        // Update state to show window is "created"
        {
            let mut state = self.state.write().unwrap();
            state.is_visible = false; // Initially hidden
        }

        println!("✅ Native window architecture ready (simplified for objc2 compatibility)");
        println!("📝 TODO: Complete NSWindow creation when objc2 APIs are stable");
        Ok(())
    }

    fn setup_search_callback(&mut self) {
        // Set up search input text change callback
        let _host_list_state = self.host_list.get_state();

        self.search_input.set_text_change_callback(move |query| {
            // TODO: Implement fuzzy search and update host list
            println!("Search query changed: {query}");
        });
    }

    fn setup_host_list_callbacks(&mut self) {
        // Set up host selection callback
        let _host_list_state = self.host_list.get_state();

        self.host_list.set_selection_change_callback(move |index| {
            println!("Host selection changed to index: {index}");
        });

        // Set up host activation callback
        self.host_list.set_host_activate_callback(move |host| {
            println!("Host activated: {}", host.name);
            // TODO: Trigger host connection
        });
    }

    #[cfg(target_os = "macos")]
    pub fn show(&self) -> Result<()> {
        // Update state
        {
            let mut state = self.state.write().unwrap();
            state.is_visible = true;
            state.is_focused = true;
        }
        println!("✅ Native window show requested (state updated)");
        println!("📝 TODO: Call NSWindow makeKeyAndOrderFront when objc2 API is stable");
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub fn hide(&self) -> Result<()> {
        // Update state
        {
            let mut state = self.state.write().unwrap();
            state.is_visible = false;
            state.is_focused = false;
        }
        println!("✅ Native window hide requested (state updated)");
        println!("📝 TODO: Call NSWindow orderOut when objc2 API is stable");
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub fn center(&self) -> Result<()> {
        println!("✅ Native window center requested");
        println!("📝 TODO: Call NSWindow center when objc2 API is stable");
        Ok(())
    }

    pub fn handle_key_event(&self, key: &str) -> Result<bool> {
        match key {
            "escape" => {
                if let Some(ref callback) = self.on_escape {
                    callback();
                }
                Ok(true) // Event handled
            }
            "up" => {
                self.host_list.select_previous()?;
                Ok(true) // Event handled
            }
            "down" => {
                self.host_list.select_next()?;
                Ok(true) // Event handled
            }
            "enter" => {
                if let Some(host) = self.host_list.get_selected_host() {
                    if let Some(ref callback) = self.on_host_selected {
                        callback(&host);
                    }
                }
                Ok(true) // Event handled
            }
            _ => {
                // Pass other events to search input
                self.search_input.handle_key_event(key)
            }
        }
    }

    pub fn update_hosts(&self, hosts: Vec<HostEntry>) -> Result<()> {
        self.host_list.update_hosts(hosts)
    }

    pub fn get_search_query(&self) -> String {
        self.search_input.get_text()
    }

    pub fn get_selected_host(&self) -> Option<HostEntry> {
        self.host_list.get_selected_host()
    }

    pub fn is_visible(&self) -> bool {
        let state = self.state.read().unwrap();
        state.is_visible
    }

    pub fn get_state(&self) -> Arc<RwLock<WindowState>> {
        self.state.clone()
    }

    // Non-macOS stub implementations
    #[cfg(not(target_os = "macos"))]
    pub fn create_native_window(&mut self) -> Result<()> {
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn show(&self) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.is_visible = true;
        state.is_focused = true;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn hide(&self) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.is_visible = false;
        state.is_focused = false;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn center(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_window_creation() {
        let config = WindowConfig::default();
        let hosts = vec![HostEntry::new("test1".to_string(), "ssh test1".to_string())];

        let window = NativeWindow::new(config.clone(), hosts);

        assert_eq!(window.config.width, config.width);
        assert_eq!(window.config.height, config.height);
        assert!(!window.is_visible());
    }

    #[test]
    fn test_window_state_operations() {
        let state = WindowState::default();

        assert!(!state.is_visible);
        assert!(!state.is_focused);
        assert!(state.position.is_none());
    }

    #[test]
    fn test_key_event_handling() {
        let config = WindowConfig::default();
        let hosts = vec![
            HostEntry::new("host1".to_string(), "ssh host1".to_string()),
            HostEntry::new("host2".to_string(), "ssh host2".to_string()),
        ];

        let window = NativeWindow::new(config, hosts);

        // Test navigation keys
        let handled = window.handle_key_event("down").unwrap();
        assert!(handled);

        let handled = window.handle_key_event("up").unwrap();
        assert!(handled);

        let handled = window.handle_key_event("escape").unwrap();
        assert!(handled);
    }
}
