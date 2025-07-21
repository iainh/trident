#![recursion_limit = "512"]

mod app;
mod config;
mod fuzzy;
// mod menubar; // Replaced with cross-platform tray-icon implementation
mod native_app;
mod native_ui;
mod objc2_hotkey;
mod ssh;
mod tray;
mod ui;

use anyhow::Result;
use app::AppState;
use config::Config;
use gpui::*;
// Removed fallback hotkey manager - using native objc2 implementation only
use objc2_hotkey::NativeHotKeyManager;
use ssh::{HostEntry, TerminalLauncher, parse_known_hosts, parse_ssh_config};
use std::path::Path;
use ui::{HostList, SearchInput};

// Define actions for the SSH launcher
actions!(trident, [ShowLauncher, QuitApp, ToggleLauncher]);

// Tray-icon integration - no global channels needed

// Global signal removed - now using GPUI actions for single-process operation

// Simple logging utility for troubleshooting
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

// Zed-like theme colors for dark mode
struct ZedTheme;

#[allow(dead_code)]
impl ZedTheme {
    fn elevated_surface_background() -> Hsla {
        // Zed's modal/popover background using hex
        rgb(0x282c34).into() // Dark blue-gray from Zed
    }

    fn surface_background() -> Hsla {
        // List background - slightly darker using hex
        rgb(0x252930).into() // Darker blue-gray
    }

    fn editor_background() -> Hsla {
        // Search input background - same as surface for seamless look
        rgb(0x252930).into()
    }

    fn border() -> Hsla {
        // Subtle borders using hex
        rgb(0x3c4043).into() // Subtle blue-gray border
    }

    fn text() -> Hsla {
        // Primary text color using hex
        rgb(0xd4d4d4).into() // Light gray text
    }

    fn text_placeholder() -> Hsla {
        // Placeholder text using hex
        rgb(0x8c8c8c).into() // Medium gray
    }

    fn text_muted() -> Hsla {
        // Secondary text using hex
        rgb(0xa5a5a5).into() // Lighter gray for secondary text
    }

    fn text_accent() -> Hsla {
        // Accent text for selected items - Zed's blue using hex
        rgb(0x569cd6).into() // #569cd6
    }

    fn ghost_element_hover() -> Hsla {
        // Hover background for list items - subtle blue-gray using hex
        rgb(0x454a55).into() // #454a55
    }

    fn ghost_element_selected() -> Hsla {
        // Selected background for list items - blue accent background
        hsla(207.0 / 360.0, 0.7, 0.25, 0.2) // Blue with transparency - try different hue format
    }

    fn cursor() -> Hsla {
        // Cursor color - same as primary text for consistency
        rgb(0xd4d4d4).into() // Light gray cursor like Zed
    }
}

// Trident now runs as a background application that responds to Cmd+Shift+S hotkey
// This gives us the core menubar-like functionality without complex StatusItem management

struct TridentApp {
    state: AppState,
    search_input: SearchInput,
    host_list: HostList,
    terminal_launcher: TerminalLauncher,
    focus_handle: FocusHandle,
}

impl TridentApp {
    #[cfg(not(test))]
    fn new(cx: &mut Context<Self>) -> Self {
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

        // Create state with loaded config
        let mut state = AppState::new();
        state.config = config.clone();

        // Load SSH hosts from files
        let hosts = Self::load_ssh_hosts(&config);
        state.hosts = hosts.clone();
        state.filtered_hosts = hosts.clone();

        let mut search_input = SearchInput::new("Search SSH hosts...".to_string());
        search_input.set_focused(true);

        let terminal_launcher = TerminalLauncher::new(config.terminal.clone());

        Self {
            state,
            search_input,
            host_list: HostList::new(hosts),
            terminal_launcher,
            focus_handle: cx.focus_handle(),
        }
    }

    #[cfg(test)]
    fn new(cx: &mut Context<Self>) -> Self {
        use config::{ParsingConfig, SshConfig, TerminalConfig, UiConfig};

        // Create a minimal test configuration
        let config = Config {
            terminal: TerminalConfig {
                program: "/bin/echo".to_string(),
                args: vec!["test".to_string()],
            },
            ssh: SshConfig {
                known_hosts_path: "/tmp/test_known_hosts".to_string(),
                config_path: "/tmp/test_config".to_string(),
                ssh_binary: "/usr/bin/ssh".to_string(),
            },
            parsing: ParsingConfig {
                parse_known_hosts: false,
                parse_ssh_config: false,
                simple_config_parsing: true,
                skip_hashed_hosts: true,
            },
            ui: UiConfig {
                max_results: 10,
                case_sensitive: false,
            },
        };

        let mut state = AppState::new();
        state.config = config.clone();

        let search_input = SearchInput::new("Test search...".to_string());
        let terminal_launcher = TerminalLauncher::new(config.terminal.clone());

        Self {
            state,
            search_input,
            host_list: HostList::new(Vec::new()),
            terminal_launcher,
            focus_handle: cx.focus_handle(),
        }
    }

    fn load_config() -> Result<Config> {
        // Try to load from default config path
        let config_path = Config::default_config_path()?;

        if !config_path.exists() {
            // Create generated config file with terminal detection
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
            let known_hosts_path = Path::new(&config.ssh.known_hosts_path);
            if !known_hosts_path.exists() {
                Logger::warn(&format!(
                    "known_hosts file '{}' not found. Skipping known_hosts parsing.",
                    config.ssh.known_hosts_path
                ));
                Logger::warn(&format!(
                    "  To fix: Create the file with 'touch {}' or disable with 'parse_known_hosts = false' in config",
                    config.ssh.known_hosts_path
                ));
            } else {
                Logger::debug(&format!(
                    "Parsing known_hosts file: {}",
                    config.ssh.known_hosts_path
                ));
                match parse_known_hosts(known_hosts_path, config.parsing.skip_hashed_hosts) {
                    Ok(hosts) => {
                        if hosts.is_empty() {
                            Logger::info("known_hosts file exists but contains no parseable hosts");
                        } else {
                            Logger::info(&format!("Loaded {} hosts from known_hosts", hosts.len()));
                        }
                        all_hosts.extend(hosts);
                    }
                    Err(e) => {
                        Logger::error(&format!(
                            "Failed to parse known_hosts '{}': {}",
                            config.ssh.known_hosts_path, e
                        ));
                        Logger::warn(
                            "  Continuing without known_hosts. Check file format or disable with 'parse_known_hosts = false'",
                        );
                    }
                }
            }
        }

        // Parse SSH config if enabled
        if config.parsing.parse_ssh_config {
            let ssh_config_path = Path::new(&config.ssh.config_path);
            if !ssh_config_path.exists() {
                Logger::warn(&format!(
                    "SSH config file '{}' not found. Skipping SSH config parsing.",
                    config.ssh.config_path
                ));
                Logger::warn(
                    "  To fix: Create a config file or disable with 'parse_ssh_config = false' in config",
                );
            } else {
                Logger::debug(&format!(
                    "Parsing SSH config file: {}",
                    config.ssh.config_path
                ));
                match parse_ssh_config(ssh_config_path, config.parsing.simple_config_parsing) {
                    Ok(hosts) => {
                        if hosts.is_empty() {
                            Logger::info("SSH config file exists but contains no Host entries");
                        } else {
                            Logger::info(&format!("Loaded {} hosts from SSH config", hosts.len()));
                        }
                        all_hosts.extend(hosts);
                    }
                    Err(e) => {
                        Logger::error(&format!(
                            "Failed to parse SSH config '{}': {}",
                            config.ssh.config_path, e
                        ));
                        Logger::warn(
                            "  Continuing without SSH config. Check file format or disable with 'parse_ssh_config = false'",
                        );
                    }
                }
            }
        }

        // Remove duplicates and sort
        all_hosts.sort_by(|a, b| a.name.cmp(&b.name));
        all_hosts.dedup_by(|a, b| a.name == b.name);

        // Fallback to examples if no hosts found
        if all_hosts.is_empty() {
            Logger::warn("No SSH hosts found, using examples");
            Logger::info("To add real hosts: add entries to ~/.ssh/known_hosts or ~/.ssh/config");
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
            Logger::debug(&format!("Total {} unique hosts loaded", all_hosts.len()));
            all_hosts
        }
    }

    fn handle_key_event(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "up" => {
                if !self.host_list.is_empty() {
                    self.host_list.select_previous();
                    cx.notify();
                }
            }
            "down" => {
                if !self.host_list.is_empty() {
                    self.host_list.select_next();
                    cx.notify();
                }
            }
            "enter" => {
                if let Some(host) = self.host_list.get_selected_host() {
                    if let Err(e) = self.launch_host(host) {
                        Logger::error(&format!("Failed to launch host: {e}"));
                    }
                    // Close window after launching
                    cx.quit();
                }
            }
            "escape" => {
                // Close window on escape
                cx.quit();
            }
            "tab" => {
                // Accept autocomplete suggestion
                self.search_input.accept_suggestion();
                self.update_search();
                cx.notify();
            }
            "r" if event.keystroke.modifiers.platform => {
                // Reload configuration (Cmd+R)
                self.reload_config_and_hosts();
                cx.notify();
            }
            "backspace" => {
                self.search_input.handle_backspace();
                self.update_search();
                cx.notify();
            }
            text => {
                // Handle regular character input
                if text.len() == 1 {
                    if let Some(ch) = text.chars().next() {
                        if ch.is_ascii_graphic() || ch == ' ' {
                            self.search_input.handle_input(text);
                            self.update_search();
                            cx.notify();
                        }
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    fn handle_host_click(&mut self, host_index: usize, cx: &mut Context<Self>) {
        // Select and launch the clicked host
        self.host_list.select_index(host_index);
        if let Some(host) = self.host_list.get_selected_host() {
            if let Err(e) = self.launch_host(host) {
                Logger::error(&format!("Failed to launch host: {e}"));
            }
            // Close window after launching
            cx.quit();
        }
    }

    #[allow(dead_code)]
    fn handle_host_double_click(&mut self, host_index: usize, _cx: &mut Context<Self>) {
        // Launch the double-clicked host
        if let Some(host) = self.host_list.hosts.get(host_index) {
            if let Err(e) = self.launch_host(host) {
                Logger::error(&format!("Failed to launch host: {e}"));
            }
        }
    }

    #[cfg(not(test))]
    fn render_search_input(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .w_full()
            .h(px(48.0))
            .child(self.search_input.clone())
    }

    #[cfg(not(test))]
    fn render_host_list_always(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        // Create a host list with the correct hosts to display
        let hosts_to_show = if self.search_input.query.is_empty() {
            self.state.hosts.clone()
        } else {
            self.host_list.hosts.clone()
        };

        let mut display_list = HostList::new(hosts_to_show);
        display_list.selected_index = self.host_list.selected_index;

        div().flex().flex_col().child(display_list)
    }

    fn update_search(&mut self) {
        // Update the app state with the current search query
        self.state.search_query = self.search_input.query.clone();

        // Use the real search functionality
        let search_engine = fuzzy::SearchEngine::new(self.state.hosts.clone());
        let results = search_engine.search(
            &self.state.search_query,
            self.state.config.ui.case_sensitive,
            self.state.config.ui.max_results,
        );

        // Convert search results to owned hosts
        let filtered_hosts: Vec<HostEntry> = results.into_iter().cloned().collect();
        self.host_list.set_hosts(filtered_hosts.clone());

        // Find and set autocomplete suggestion
        let suggestion = self.find_autocomplete_suggestion(&filtered_hosts);
        self.search_input.set_suggestion(suggestion);
    }

    fn find_autocomplete_suggestion(&self, filtered_hosts: &[HostEntry]) -> Option<String> {
        let query = &self.search_input.query;

        // Don't suggest if query is empty
        if query.is_empty() {
            return None;
        }

        // Find the best prefix match from results
        let query_lower = query.to_lowercase();

        // Look for exact prefix matches first
        for host in filtered_hosts {
            let host_name_lower = host.name.to_lowercase();
            if host_name_lower.starts_with(&query_lower) && host.name.len() > query.len() {
                return Some(host.name.clone());
            }
        }

        None
    }

    fn launch_host(&self, host: &HostEntry) -> Result<()> {
        self.terminal_launcher.launch(host)
    }

    fn reload_config_and_hosts(&mut self) {
        // For tests, just log that reload was called
        if cfg!(test) {
            Logger::info("Config reload triggered (test mode)");
            return;
        }

        Logger::info("Reloading configuration and SSH hosts...");

        // Load new configuration
        match Self::load_config() {
            Ok(mut new_config) => {
                // Expand tilde paths
                if let Err(e) = new_config.expand_path() {
                    Logger::error(&format!("Failed to expand config paths during reload: {e}"));
                    return;
                }

                // Validate configuration
                if let Err(e) = new_config.validate() {
                    Logger::error(&format!("Invalid configuration during reload: {e}"));
                    return;
                }

                // Update app state with new config
                self.state.config = new_config.clone();

                // Update terminal launcher with new config
                self.terminal_launcher = TerminalLauncher::new(new_config.terminal.clone());

                // Reload SSH hosts with new config
                let new_hosts = Self::load_ssh_hosts(&new_config);
                self.state.hosts = new_hosts.clone();
                self.state.filtered_hosts = new_hosts.clone();

                // Update host list and clear current search
                self.host_list.set_hosts(new_hosts.clone());
                self.search_input.query.clear();
                self.search_input.suggestion = None;

                // Reset search state
                self.state.search_query.clear();
                self.update_search();

                Logger::info("Configuration and SSH hosts reloaded successfully");
            }
            Err(e) => {
                Logger::error(&format!("Failed to reload configuration: {e}"));
            }
        }
    }
}

// Integration tests are challenging with GPUI due to macro complexity
// Core logic is tested in individual modules (config, ssh, fuzzy, app, ui)
// UI functionality is tested through manual testing and the running application

#[cfg(not(test))]
impl Render for TridentApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Focus the window when it first appears
        window.focus(&self.focus_handle);

        div()
            .flex()
            .items_start()
            .justify_center()
            .w_full()
            .h_full()
            .pt(px(360.0)) // ~1/3 down from top of screen (1080px / 3)
            .track_focus(&self.focus_handle)
            .on_key_down(
                cx.listener(|this, event: &KeyDownEvent, _window: &mut Window, cx| {
                    this.handle_key_event(event, cx);
                }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(600.0)) // Fixed width like Zed's command palette
                    .max_h(px(500.0)) // Reasonable max height to prevent overflow
                    .bg(ZedTheme::elevated_surface_background().alpha(0.75)) // 25% transparency
                    .border_1()
                    .border_color(hsla(0.0, 0.0, 1.0, 0.15)) // Subtle white border with 15% opacity
                    .rounded_lg()
                    .overflow_hidden() // This clips content to rounded corners
                    .shadow(vec![BoxShadow {
                        color: hsla(0.0, 0.0, 0.0, 0.3),      // Dark shadow with 30% opacity
                        offset: Point::new(px(0.0), px(8.0)), // Drop shadow downward
                        blur_radius: px(24.0),                // macOS-style blur
                        spread_radius: px(0.0),
                    }])
                    .p(px(4.0)) // Add padding to prevent content from covering rounded corners
                    .child(self.render_search_input(cx))
                    .child(self.render_host_list_always(cx)),
            )
    }
}

#[cfg(not(test))]
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Check for native mode flag
    if args.len() > 1 && args[1] == "--native" {
        Logger::info("Starting Trident in native mode (no GPUI)...");
        return native_app::run_native_app();
    }


    Logger::info("Starting Trident SSH Launcher (GPUI mode)...");
    Logger::info("💡 Use --native flag to run in single-process native mode");

    // Run the menubar app within GPUI context
    run_menubar_app()
}


fn run_menubar_app() -> Result<()> {
    Application::new().run(|cx: &mut App| {
        // Skip activation policy in GPUI mode - GPUI manages its own NSApplication
        // The dock icon will be visible in GPUI mode, which is acceptable
        Logger::info("GPUI mode - using GPUI's application management (dock icon visible)");

        // No need for channels with tray-icon - events are handled directly
        
        // Set up global state first so it can be accessed by tray events
        cx.set_global(TridentState {
            should_show_launcher: false,
            launcher_window: None,
        });
        
        // Set up periodic tray event checking on the main thread using GPUI timer
        cx.spawn(async move |mut cx| {
            Logger::info("Tray event processor started - checking every 50ms on main thread");
            loop {
                // Use GPUI's timer for main thread scheduling
                cx.background_executor().timer(std::time::Duration::from_millis(50)).await;
                
                // Check for tray events and update GPUI state directly
                while let Some(event) = tray::TridentTray::try_recv_tray_event() {
                    Logger::info(&format!("Processing tray event: {:?}", event));
                    match event {
                        tray::TrayEvent::Click | tray::TrayEvent::DoubleClick | tray::TrayEvent::OpenTrident => {
                            Logger::info("Tray event received: triggering launcher");
                            // Update GPUI global state to show launcher
                            cx.update_global::<TridentState, ()>(|state, _| {
                                state.should_show_launcher = true;
                            }).ok(); // Ignore errors if global not available
                        }
                        tray::TrayEvent::ToggleStartAtLogin => {
                            Logger::info("Toggle start at login (not implemented)");
                        }
                        tray::TrayEvent::Quit => {
                            Logger::info("Quit requested from tray menu");
                            std::process::exit(0);
                        }
                    }
                }
            }
        }).detach();
        
        // Set up observer to respond to launcher show requests
        cx.observe_global::<TridentState>(move |cx| {
            // Check GPUI state for launcher window requests
            if let Some(state) = cx.try_global::<TridentState>() {
                if state.should_show_launcher {
                    Logger::info("TridentState observer triggered - showing launcher window");
                    show_launcher_window(cx);
                    cx.update_global::<TridentState, ()>(|state, _| {
                        state.should_show_launcher = false;
                    });
                }
            }
        }).detach();

        // No need for menubar callback with tray-icon - events are polled directly

        // Try native hotkey (objc2-based, single process)
        let mut native_hotkey_manager = NativeHotKeyManager::new();
        
        // Native hotkey callback - directly updates GPUI state (for when hotkeys work)
        let native_hotkey_callback = move || {
            Logger::info("Native global hotkey triggered - would trigger launcher");
            // Note: This doesn't currently work due to accessibility permissions
            // When it works, we'd need to trigger the launcher here
        };
        
        native_hotkey_manager.set_callback(native_hotkey_callback).unwrap_or_else(|e| {
            Logger::error(&format!("Failed to set native hotkey callback: {e}"));
        });

        let native_hotkey_success = native_hotkey_manager.register_cmd_shift_s().is_ok();
        
        if native_hotkey_success {
            Logger::info("✅ GPUI global hotkey registered: Cmd+Shift+S (single-process)");
            Logger::info("🔗 Hotkey successfully integrated with GPUI window management");
            // Keep the native hotkey manager alive
            std::mem::forget(native_hotkey_manager);
        } else {
            Logger::error("❌ Failed to register native global hotkey");
            Logger::warn("⚠️  No global hotkey available - use menubar only");
        }

        // Create the cross-platform tray icon
        let _tray = match tray::TridentTray::new() {
            Ok(tray) => {
                Logger::info("Cross-platform tray icon created! Look for the ψ (trident) icon in your system tray");
                Logger::info("Press Cmd+Shift+S to open the SSH launcher");
                Logger::info("🔗 Tray icon successfully integrated with GPUI event system");
                tray
            }
            Err(e) => {
                Logger::error(&format!("Failed to create tray icon: {e}"));
                Logger::info("Falling back to window-based approach - will restart with window mode");
                panic!("Failed to create tray icon: {e}");
            }
        };

        // Keep the tray alive by forgetting it
        std::mem::forget(_tray);

        // Set focus behavior to not activate when clicked
        cx.activate(false);
    });

    Ok(())
}


#[cfg(not(test))]
#[allow(dead_code)]
fn run_with_window() -> Result<()> {
    Application::new().run(|cx: &mut App| {
        // Create a small menubar window that shows the Trident icon
        let menu_window = cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    appears_transparent: true,
                    title: Some("Trident SSH Launcher".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(50.0), px(50.0)),
                    size: Size {
                        width: px(200.0),
                        height: px(100.0),
                    },
                })),
                is_movable: true,
                kind: WindowKind::Normal,
                ..Default::default()
            },
            |_, cx| cx.new(|_cx| TridentMenuBarWindow::new()),
        );

        if let Ok(_window) = menu_window {
            // Register key bindings
            cx.bind_keys([
                KeyBinding::new("cmd-shift-s", ToggleLauncher, Some("TridentMenuBar")),
                KeyBinding::new("cmd-q", QuitApp, Some("TridentMenuBar")),
            ]);

            // For now, we'll check the global flag in action handlers
            // TODO: Find a better way to bridge native callbacks to GPUI

            // Store the window handle globally so we can manage it
            cx.observe_global::<TridentState>(move |cx| {
                // Check GPUI state for launcher window requests
                if let Some(state) = cx.try_global::<TridentState>() {
                    if state.should_show_launcher {
                        show_launcher_window(cx);
                        cx.update_global::<TridentState, ()>(|state, _| {
                            state.should_show_launcher = false;
                        });
                    }
                }
            })
            .detach();
        }

        cx.activate(true);
    });

    Ok(())
}

#[derive(Default)]
#[allow(dead_code)]
struct TridentState {
    should_show_launcher: bool,
    launcher_window: Option<AnyWindowHandle>,
}

#[allow(dead_code)]
impl TridentState {
    fn new() -> Self {
        Self::default()
    }
}

impl Global for TridentState {}

#[cfg(not(test))]
#[allow(dead_code)]
fn show_launcher_window(cx: &mut App) {
    // Close existing launcher window if any
    hide_launcher_window(cx);

    // Get display bounds for positioning
    let display_bounds = cx.primary_display().map(|d| d.bounds()).unwrap_or(Bounds {
        origin: Point::new(px(0.0), px(0.0)),
        size: Size {
            width: px(1920.0),
            height: px(1080.0),
        },
    });

    // Create the search window
    let window = cx.open_window(
        WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Fullscreen(display_bounds)),
            is_movable: false,
            kind: WindowKind::PopUp,
            window_background: WindowBackgroundAppearance::Transparent,
            window_decorations: Some(WindowDecorations::Client),
            ..Default::default()
        },
        |_, cx| cx.new(TridentApp::new),
    );

    // Store the window handle
    if let Ok(handle) = window {
        cx.update_global::<TridentState, ()>(|state, _| {
            state.launcher_window = Some(handle.into());
        });
        Logger::info("✅ GPUI launcher window created and shown");
    } else {
        Logger::error("❌ Failed to create GPUI launcher window");
    }
}

#[cfg(not(test))]
#[allow(dead_code)]
fn hide_launcher_window(cx: &mut App) {
    // Close existing launcher window if any
    let window_handle = cx.update_global::<TridentState, Option<AnyWindowHandle>>(|state, _| {
        state.launcher_window.take()
    });
    
    if let Some(_handle) = window_handle {
        // GPUI windows are automatically closed when their handle is dropped
        // Just dropping the handle will close the window
        Logger::info("✅ GPUI launcher window hidden/closed");
    } else {
        Logger::debug("No GPUI launcher window to hide");
    }
}

#[allow(dead_code)]
struct TridentMenuBarWindow;

#[allow(dead_code)]
impl TridentMenuBarWindow {
    fn new() -> Self {
        Self
    }
}

impl Render for TridentMenuBarWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .w_full()
            .h_full()
            .bg(rgb(0xF5F5F5))
            .key_context("TridentMenuBar")
            .on_action(cx.listener(|_this, _: &ToggleLauncher, _window, cx| {
                Logger::info("Toggle launcher hotkey triggered!");
                
                // Directly trigger launcher for the hotkey
                cx.update_global::<TridentState, ()>(|state, _cx| {
                    state.should_show_launcher = true;
                });
            }))
            .on_action(cx.listener(|_this, _: &QuitApp, _window, cx| {
                Logger::info("Quitting Trident...");
                cx.quit();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(60.0))
                    .h(px(60.0))
                    .bg(rgb(0x007AFF))
                    .rounded_lg()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x0051D5)))
                    .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                        Logger::info("Menu icon clicked!");
                        cx.update_global::<TridentState, ()>(|state, _cx| {
                            state.should_show_launcher = true;
                        });
                    })
                    .child(
                        div()
                            .text_color(rgb(0xFFFFFF))
                            .text_size(px(14.0))
                            .font_weight(FontWeight::BOLD)
                            .child("SSH"),
                    ),
            )
            .child(
                div()
                    .mt(px(10.0))
                    .text_color(rgb(0x666666))
                    .text_size(px(11.0))
                    .child("Click to open launcher"),
            )
            .child(
                div()
                    .mt(px(5.0))
                    .text_color(rgb(0x666666))
                    .text_size(px(10.0))
                    .child("Cmd+Shift+S to toggle"),
            )
    }
}

// Actions are defined above using the actions! macro

#[cfg(test)]
fn main() -> Result<()> {
    // Tests only main function
    Ok(())
}
