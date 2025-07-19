#![recursion_limit = "512"]

mod config;
mod ssh;
mod fuzzy;
mod app;
mod ui;

use anyhow::Result;
use app::AppState;
use gpui::*;
use gpui::prelude::FluentBuilder;
use ui::{SearchInput, HostList};
use ssh::{HostEntry, TerminalLauncher, parse_known_hosts, parse_ssh_config};
use config::Config;
use std::path::Path;

// Zed-like theme colors for dark mode
struct ZedTheme;

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
        hsla(207.0/360.0, 0.7, 0.25, 0.2) // Blue with transparency - try different hue format
    }
    
    fn cursor() -> Hsla {
        // Cursor color - same as primary text for consistency
        rgb(0xd4d4d4).into() // Light gray cursor like Zed
    }
}

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
            eprintln!("Failed to load config: {}. Using defaults.", e);
            Config::default()
        });
        
        // Expand tilde paths
        if let Err(e) = config.expand_path() {
            eprintln!("Failed to expand config paths: {}. Using defaults.", e);
            config = Config::default();
        }
        
        // Validate configuration
        if let Err(e) = config.validate() {
            eprintln!("Invalid configuration: {}. Using defaults.", e);
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
    
    #[cfg(not(test))]
    fn load_config() -> Result<Config> {
        // Try to load from default config path
        let config_path = Config::default_config_path()?;
        
        if !config_path.exists() {
            // Create default config file if it doesn't exist
            Config::save_default_config(&config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create default configuration file: {}", e))?;
            println!("Created default configuration at: {}", config_path.display());
        }
        
        Config::load_from_file(&config_path)
    }
    
    #[cfg(not(test))]
    fn load_ssh_hosts(config: &Config) -> Vec<HostEntry> {
        let mut all_hosts = Vec::new();
        
        // Parse known_hosts if enabled
        if config.parsing.parse_known_hosts {
            match parse_known_hosts(
                Path::new(&config.ssh.known_hosts_path),
                config.parsing.skip_hashed_hosts,
            ) {
                Ok(hosts) => {
                    println!("Loaded {} hosts from known_hosts", hosts.len());
                    all_hosts.extend(hosts);
                }
                Err(e) => {
                    eprintln!("Failed to parse known_hosts: {}", e);
                }
            }
        }
        
        // Parse SSH config if enabled
        if config.parsing.parse_ssh_config {
            match parse_ssh_config(
                Path::new(&config.ssh.config_path),
                config.parsing.simple_config_parsing,
            ) {
                Ok(hosts) => {
                    println!("Loaded {} hosts from SSH config", hosts.len());
                    all_hosts.extend(hosts);
                }
                Err(e) => {
                    eprintln!("Failed to parse SSH config: {}", e);
                }
            }
        }
        
        // Remove duplicates and sort
        all_hosts.sort_by(|a, b| a.name.cmp(&b.name));
        all_hosts.dedup_by(|a, b| a.name == b.name);
        
        // Fallback to examples if no hosts found
        if all_hosts.is_empty() {
            println!("No SSH hosts found, using examples");
            vec![
                HostEntry::new("example1.com".to_string(), "ssh user@example1.com".to_string()),
                HostEntry::new("example2.com".to_string(), "ssh user@example2.com".to_string()),
            ]
        } else {
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
                        eprintln!("Failed to launch host: {}", e);
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
                // cx.notify();
            }
            "backspace" => {
                self.search_input.handle_backspace();
                self.update_search();
                // cx.notify();
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
    
    fn handle_host_click(&mut self, host_index: usize, cx: &mut Context<Self>) {
        // Select and launch the clicked host
        self.host_list.select_index(host_index);
        if let Some(host) = self.host_list.get_selected_host() {
            if let Err(e) = self.launch_host(host) {
                eprintln!("Failed to launch host: {}", e);
            }
            // Close window after launching
            cx.quit();
        }
    }
    
    fn handle_host_double_click(&mut self, host_index: usize, _cx: &mut Context<Self>) {
        // Launch the double-clicked host
        if let Some(host) = self.host_list.hosts.get(host_index) {
            if let Err(e) = self.launch_host(host) {
                eprintln!("Failed to launch host: {}", e);
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
            .bg(ZedTheme::editor_background())
            .border_b_1()
            .border_color(ZedTheme::border())
            .child(self.search_input.clone())
    }
    
    #[cfg(not(test))]
    fn render_host_list_always(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Always show the host list container
        div()
            .flex()
            .flex_col()
            .bg(ZedTheme::surface_background())
            .child(self.render_host_list(cx))
    }

    #[cfg(not(test))]
    fn render_host_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Show all hosts (filtered or all) if query is empty, up to max_visible
        let hosts_to_show = if self.search_input.query.is_empty() {
            &self.state.hosts
        } else {
            &self.host_list.hosts
        };

        // Calculate visible hosts and their count for dynamic sizing
        // Limit to a reasonable number that fits well on screen
        let max_items_to_show = 8; // Show up to 8 items for good UX
        
        let hosts_to_render: Vec<_> = hosts_to_show.iter()
            .take(max_items_to_show)
            .collect();

        let visible_hosts = hosts_to_render.iter()
            .enumerate()
            .map(|(index, host)| {
                let is_selected = index == self.host_list.selected_index;
                
                div()
                    .flex()
                    .items_center()
                    .w_full()
                    .px_4()
                    .py_2()
                    .when(is_selected, |div| {
                        div.bg(ZedTheme::ghost_element_selected())
                    })
                    .when(!is_selected, |div| {
                        div.hover(|style| style.bg(ZedTheme::ghost_element_hover()))
                    })
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                        this.handle_host_click(index, cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_color(if is_selected {
                                        ZedTheme::text_accent()
                                    } else {
                                        ZedTheme::text()
                                    })
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(host.name.clone())
                            )
                            .child(
                                div()
                                    .text_color(ZedTheme::text_muted())
                                    .text_size(px(12.0))
                                    .child(host.connection_string.clone())
                            )
                    )
            })
            .collect::<Vec<_>>();
        
        let host_count = hosts_to_render.len();
        let is_empty = host_count == 0;
        
        div()
            .flex()
            .flex_col()
            .w_full()
            .when(is_empty, |this| {
                // Show minimal height when no results
                this.h(px(60.0))  // Fixed height for "no results" message
                    .bg(ZedTheme::surface_background())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .h_full()
                            .text_color(ZedTheme::text_muted())
                            .text_size(px(14.0))
                            .child("No hosts found")
                    )
            })
            .when(!is_empty, |this| {
                // Let GPUI calculate natural height, but constrain maximum height
                this.max_h(px(400.0)) // Prevent window from getting too tall
                    .bg(ZedTheme::surface_background())
                    .children(visible_hosts)
            })
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
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window: &mut Window, cx| {
                this.handle_key_event(event, cx);
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(600.0)) // Fixed width like Zed's command palette
                    .max_h(px(500.0)) // Reasonable max height to prevent overflow
                    .bg(ZedTheme::elevated_surface_background())
                    .border_1()
                    .border_color(ZedTheme::border())
                    .rounded_lg()
                    .shadow_lg()
                    .overflow_hidden()
                    .child(self.render_search_input(cx))
                    .child(self.render_host_list_always(cx))
            )
    }
}

#[cfg(not(test))]
fn main() -> Result<()> {
    Application::new().run(|cx: &mut App| {
        // Get display bounds for positioning
        let display_bounds = cx.primary_display()
            .map(|d| d.bounds())
            .unwrap_or(Bounds {
                origin: Point::new(px(0.0), px(0.0)),
                size: Size {
                    width: px(1920.0),
                    height: px(1080.0),
                },
            });
        
        // Create a fullscreen overlay window to avoid any window shadows
        let _ = cx.open_window(
            WindowOptions {
                titlebar: None,
                window_bounds: Some(WindowBounds::Fullscreen(display_bounds)),
                is_movable: false,
                kind: WindowKind::PopUp,
                window_background: WindowBackgroundAppearance::Transparent,
                window_decorations: Some(WindowDecorations::Client),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|cx| TridentApp::new(cx))
            }
        );
        cx.activate(true);
    });
    
    Ok(())
}

#[cfg(test)]
fn main() -> Result<()> {
    // Tests only main function
    Ok(())
}
