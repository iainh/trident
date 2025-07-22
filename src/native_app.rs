// ABOUTME: Native single-process application using objc2 and native UI components
// ABOUTME: Eliminates process spawning issues by using window show/hide within same process

use crate::app::AppState;
use crate::config::Config;
use crate::native_ui::{NativeWindow, WindowConfig};
use crate::objc2_hotkey::NativeHotKeyManager;
// use crate::menubar::TridentMenuBar; // Replaced with cross-platform tray-icon
use crate::ssh::parser::HostEntry;
use anyhow::Result;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, RwLock, mpsc};

#[cfg(target_os = "macos")]
use objc2_app_kit::NSApplication;
#[cfg(target_os = "macos")]
use objc2_foundation::MainThreadMarker;

// Simple logging utility
pub struct Logger;

impl Logger {
    pub fn info(msg: &str) {
        println!("[INFO] {msg}");
    }

    pub fn warn(msg: &str) {
        eprintln!("[WARN] {msg}");
    }

    pub fn error(msg: &str) {
        eprintln!("[ERROR] {msg}");
    }

    pub fn debug(msg: &str) {
        if std::env::var("TRIDENT_DEBUG").is_ok() {
            eprintln!("[DEBUG] {msg}");
        }
    }
}

// Commands that can be sent to the main app thread
#[derive(Debug, Clone)]
pub enum AppCommand {
    ToggleWindow,
    #[allow(dead_code)]
    ShowWindow,
    #[allow(dead_code)]
    HideWindow,
    #[allow(dead_code)]
    Quit,
}

// Native application state that manages the window lifecycle
pub struct NativeApp {
    // Core application logic (unchanged)
    app_state: Arc<RwLock<AppState>>,

    // Native UI components
    launcher_window: Option<NativeWindow>,

    // System integration
    hotkey_manager: Option<NativeHotKeyManager>,
    // menubar: Option<TridentMenuBar>, // Replaced with cross-platform tray-icon

    // Configuration
    #[allow(dead_code)]
    config: Config,

    // Command channel for thread-safe communication
    command_sender: Sender<AppCommand>,
    command_receiver: Receiver<AppCommand>,
}

impl NativeApp {
    pub fn new() -> Result<Self> {
        // Create command channel for thread-safe communication
        let (command_sender, command_receiver) = mpsc::channel();

        // Load configuration
        let mut config = Self::load_config().unwrap_or_else(|e| {
            eprintln!("Failed to load config: {e}. Using defaults.");
            Config::default()
        });

        // Expand tilde paths
        if let Err(e) = config.expand_path() {
            eprintln!("Failed to expand config paths: {e}. Using defaults.");
            config = Config::default();
        }

        // Validate configuration
        if let Err(e) = config.validate() {
            eprintln!("Invalid configuration: {e}. Using defaults.");
            config = Config::default();
        }

        // Create core application state
        let mut app_state = AppState::new();
        app_state.config = config.clone();

        // Load SSH hosts
        let hosts = Self::load_ssh_hosts(&config);
        app_state.hosts = hosts.clone();
        app_state.filtered_hosts = hosts.clone();

        Ok(Self {
            app_state: Arc::new(RwLock::new(app_state)),
            launcher_window: None,
            hotkey_manager: None,
            // menubar: None, // Replaced with cross-platform tray-icon
            config,
            command_sender,
            command_receiver,
        })
    }

    fn load_config() -> Result<Config> {
        let config_path = Config::default_config_path()?;

        if !config_path.exists() {
            Config::save_generated_config(&config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create configuration file: {}", e))?;
            Logger::info(&format!(
                "Created configuration with auto-detected terminal at: {}",
                config_path.display()
            ));
        }

        Config::load_from_file(&config_path)
    }

    fn load_ssh_hosts(config: &Config) -> Vec<HostEntry> {
        let mut all_hosts = Vec::new();

        // Parse known_hosts if enabled
        if config.parsing.parse_known_hosts {
            let known_hosts_path = std::path::Path::new(&config.ssh.known_hosts_path);
            if known_hosts_path.exists() {
                match crate::ssh::parser::parse_known_hosts(
                    known_hosts_path,
                    config.parsing.skip_hashed_hosts,
                ) {
                    Ok(hosts) => {
                        Logger::info(&format!("Loaded {} hosts from known_hosts", hosts.len()));
                        all_hosts.extend(hosts);
                    }
                    Err(e) => {
                        Logger::error(&format!("Failed to parse known_hosts: {e}"));
                    }
                }
            }
        }

        // Parse SSH config if enabled
        if config.parsing.parse_ssh_config {
            let ssh_config_path = std::path::Path::new(&config.ssh.config_path);
            if ssh_config_path.exists() {
                match crate::ssh::parser::parse_ssh_config(
                    ssh_config_path,
                    config.parsing.simple_config_parsing,
                ) {
                    Ok(hosts) => {
                        Logger::info(&format!("Loaded {} hosts from SSH config", hosts.len()));
                        all_hosts.extend(hosts);
                    }
                    Err(e) => {
                        Logger::error(&format!("Failed to parse SSH config: {e}"));
                    }
                }
            }
        }

        // Remove duplicates and sort
        all_hosts.sort_by(|a, b| a.name.cmp(&b.name));
        all_hosts.dedup_by(|a, b| a.name == b.name);

        if all_hosts.is_empty() {
            Logger::warn("No SSH hosts found, using examples");
            vec![
                HostEntry::new(
                    "example1.com".to_string(),
                    "ssh user@example1.com".to_string(),
                ),
                HostEntry::new(
                    "example2.com".to_string(),
                    "ssh user@example2.com".to_string(),
                ),
            ]
        } else {
            all_hosts
        }
    }

    pub fn initialize_ui(&mut self) -> Result<()> {
        // Create launcher window with native components
        let window_config = WindowConfig::default();
        let hosts = {
            let state = self.app_state.read().unwrap();
            state.hosts.clone()
        };

        let mut window = NativeWindow::new(window_config, hosts);

        // Set up window callbacks
        let _app_state_clone = self.app_state.clone();
        window.set_host_selected_callback(move |host| {
            Logger::info(&format!("Selected host: {}", host.name));
            // TODO: Launch SSH connection
        });

        let _app_state_clone = self.app_state.clone();
        window.set_escape_callback(move || {
            Logger::info("Escape pressed - hiding window");
            // TODO: Hide window
        });

        // Create the native window (simplified for now)
        window.create_native_window()?;

        self.launcher_window = Some(window);
        Logger::info("Native launcher window initialized");

        Ok(())
    }

    pub fn setup_global_hotkey(&mut self) -> Result<()> {
        let mut hotkey_manager = NativeHotKeyManager::new();

        // Clone the command sender for the hotkey callback
        let command_sender = self.command_sender.clone();

        // Create callback that sends toggle command to main thread
        let window_show_callback = move || {
            Logger::info("Global hotkey triggered - sending toggle window command");
            if let Err(e) = command_sender.send(AppCommand::ToggleWindow) {
                Logger::error(&format!("Failed to send toggle window command: {e}"));
            } else {
                Logger::info("🎯 Hotkey integration working - toggle command sent");
            }
        };

        hotkey_manager.set_callback(window_show_callback)?;

        match hotkey_manager.register_cmd_shift_s() {
            Ok(()) => {
                Logger::info("✅ Native global hotkey registered: Cmd+Shift+S (single-process)");
                Logger::info("🔗 Hotkey successfully integrated with native window management");
                self.hotkey_manager = Some(hotkey_manager);
                Ok(())
            }
            Err(e) => {
                Logger::error(&format!("❌ Failed to register global hotkey: {e}"));
                Err(e)
            }
        }
    }

    // pub fn setup_menubar(&mut self) -> Result<()> {
    //     // Replaced with cross-platform tray-icon implementation in main.rs
    //     Ok(())
    // }

    #[cfg(target_os = "macos")]
    pub fn configure_app_as_background(&self) -> Result<()> {
        // For now, skip the activation policy to avoid objc2 compatibility issues
        // The app will still work, just with a dock icon visible
        Logger::info("Skipping activation policy (app will show in dock)");
        Logger::info("TODO: Configure as menubar-only app when objc2 API is stable");
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn configure_app_as_background(&self) -> Result<()> {
        Logger::info("Background app configuration not needed on this platform");
        Ok(())
    }

    pub fn toggle_launcher(&mut self) -> Result<()> {
        if let Some(window) = &self.launcher_window {
            if window.is_visible() {
                window.hide()?;
                Logger::info("Launcher window hidden");
            } else {
                window.show()?;
                Logger::info("Launcher window shown");
            }
        } else {
            Logger::warn("Launcher window not initialized");
        }
        Ok(())
    }

    pub fn show_launcher(&mut self) -> Result<()> {
        if let Some(window) = &self.launcher_window {
            window.show()?;
            Logger::info("Launcher window shown");
        } else {
            Logger::warn("Launcher window not initialized");
        }
        Ok(())
    }

    pub fn hide_launcher(&mut self) -> Result<()> {
        if let Some(window) = &self.launcher_window {
            window.hide()?;
            Logger::info("Launcher window hidden");
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn handle_key_event(&self, key: &str) -> Result<bool> {
        if let Some(window) = &self.launcher_window {
            window.handle_key_event(key)
        } else {
            Ok(false)
        }
    }

    #[allow(dead_code)]
    pub fn update_hosts(&self, hosts: Vec<HostEntry>) -> Result<()> {
        // Update app state
        {
            let mut state = self.app_state.write().unwrap();
            state.hosts = hosts.clone();
            state.filtered_hosts = hosts.clone();
        }

        // Update UI
        if let Some(window) = &self.launcher_window {
            window.update_hosts(hosts)?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_launcher_visible(&self) -> bool {
        if let Some(window) = &self.launcher_window {
            window.is_visible()
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn get_app_state(&self) -> Arc<RwLock<AppState>> {
        self.app_state.clone()
    }

    pub fn run_command_loop(&mut self) -> Result<()> {
        Logger::info("Starting command processing loop");

        loop {
            // Process commands with a timeout to prevent blocking indefinitely
            match self
                .command_receiver
                .recv_timeout(std::time::Duration::from_millis(100))
            {
                Ok(command) => {
                    Logger::debug(&format!("Processing command: {command:?}"));

                    match command {
                        AppCommand::ToggleWindow => {
                            if let Err(e) = self.toggle_launcher() {
                                Logger::error(&format!("Failed to toggle window: {e}"));
                            } else {
                                Logger::info("✅ Window toggled successfully");
                            }
                        }
                        AppCommand::ShowWindow => {
                            if let Err(e) = self.show_launcher() {
                                Logger::error(&format!("Failed to show window: {e}"));
                            } else {
                                Logger::info("✅ Window shown successfully");
                            }
                        }
                        AppCommand::HideWindow => {
                            if let Err(e) = self.hide_launcher() {
                                Logger::error(&format!("Failed to hide window: {e}"));
                            } else {
                                Logger::info("✅ Window hidden successfully");
                            }
                        }
                        AppCommand::Quit => {
                            Logger::info("Quit command received - exiting application");
                            break;
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Normal timeout - continue loop
                    continue;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    Logger::info("Command channel disconnected - exiting");
                    break;
                }
            }
        }

        Logger::info("Command loop finished");
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_command_sender(&self) -> Sender<AppCommand> {
        self.command_sender.clone()
    }
}

// Helper function to run the native app without GPUI
pub fn run_native_app() -> Result<()> {
    Logger::info("Starting Trident SSH Launcher (Native Mode)...");

    // First, initialize NSApplication to set up Core Graphics properly
    #[cfg(target_os = "macos")]
    {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let _app = NSApplication::sharedApplication(mtm);
            // This initializes the Core Graphics connection properly
        }
    }

    let mut app = NativeApp::new()?;

    // Configure as background app (after NSApplication is initialized)
    app.configure_app_as_background()?;

    // Set up system integration (before UI to avoid graphics calls)
    if let Err(e) = app.setup_global_hotkey() {
        Logger::warn(&format!("Failed to set up global hotkey: {e}"));
        Logger::warn("Continuing with menubar-only operation");
    }

    // if let Err(e) = app.setup_menubar() {
    //     Logger::warn(&format!("Failed to set up menubar: {e}"));
    //     Logger::warn("Continuing without menubar integration");
    // }
    Logger::info("Using cross-platform tray-icon instead of native menubar");

    // Initialize UI components (after Core Graphics is ready)
    app.initialize_ui()?;

    Logger::info("🚀 Trident is running in native mode!");
    Logger::info("• Press Cmd+Shift+S to open SSH launcher");
    Logger::info("• Click the ψ (trident) icon in your menubar");
    Logger::info("• No process spawning - single process architecture");

    // Use a simple event loop without UI creation for now
    // This proves the architecture works without graphics complications
    Logger::info("📍 Native app is running - press Ctrl+C to exit");

    // Main event loop to process commands
    app.run_command_loop()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_app_creation() {
        let app = NativeApp::new();
        assert!(app.is_ok());

        let app = app.unwrap();
        assert!(!app.is_launcher_visible());
    }

    #[test]
    fn test_load_ssh_hosts() {
        let config = Config::default();
        let hosts = NativeApp::load_ssh_hosts(&config);

        // Should return example hosts when no SSH files exist
        assert!(!hosts.is_empty());
        assert!(hosts.iter().any(|h| h.name.contains("example")));
    }
}
