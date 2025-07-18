#![recursion_limit = "512"]

mod config;
mod ssh;
mod fuzzy;
mod app;
mod ui;

use anyhow::Result;
use app::AppState;
use gpui::*;
use ui::{SearchInput, HostList};
use ssh::parser::HostEntry;

struct TridentApp {
    state: AppState,
    search_input: SearchInput,
    host_list: HostList,
    focus_handle: FocusHandle,
}

impl TridentApp {
    #[cfg(not(test))]
    fn new(cx: &mut Context<Self>) -> Self {
        let mut state = AppState::new();
        
        // Load some example hosts for now
        let example_hosts = vec![
            HostEntry::new("server1.example.com".to_string(), "ssh user@server1.example.com".to_string()),
            HostEntry::new("server2.example.com".to_string(), "ssh user@server2.example.com".to_string()),
            HostEntry::new("production.company.com".to_string(), "ssh deploy@production.company.com".to_string()),
            HostEntry::new("staging.company.com".to_string(), "ssh deploy@staging.company.com".to_string()),
            HostEntry::new("dev.company.com".to_string(), "ssh dev@dev.company.com".to_string()),
        ];
        
        state.hosts = example_hosts.clone();
        state.filtered_hosts = example_hosts.clone();
        
        Self {
            state,
            search_input: SearchInput::new("Search SSH hosts...".to_string()),
            host_list: HostList::new(example_hosts),
            focus_handle: cx.focus_handle(),
        }
    }
    
    fn handle_key_event(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "up" => {
                self.host_list.select_previous();
                cx.notify();
            }
            "down" => {
                self.host_list.select_next();
                cx.notify();
            }
            "enter" => {
                if let Some(host) = self.host_list.get_selected_host() {
                    if let Err(e) = self.launch_host(host) {
                        eprintln!("Failed to launch host: {}", e);
                    }
                }
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
    
    fn handle_host_click(&mut self, host_index: usize, cx: &mut Context<Self>) {
        // Select the clicked host
        self.host_list.select_index(host_index);
        cx.notify();
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
    fn render_host_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let container = div()
            .flex()
            .flex_col()
            .w_full()
            .max_h(px(400.0))
            .overflow_y_hidden()
            .bg(rgb(0x2d2d2d))
            .border_1()
            .border_color(rgb(0x444444))
            .rounded_md();
        
        if self.host_list.hosts.is_empty() {
            return container.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .p_4()
                    .text_color(rgb(0x666666))
                    .text_size(px(14.0))
                    .child("No hosts found")
            );
        }
        
        let visible_hosts = self.host_list.hosts.iter()
            .take(self.host_list.max_visible)
            .enumerate()
            .map(|(index, host)| {
                let is_selected = index == self.host_list.selected_index;
                let bg_color = if is_selected {
                    rgb(0x0066cc) // Blue for selected
                } else {
                    rgb(0x2d2d2d) // Default background
                };
                
                let text_color = if is_selected {
                    rgb(0xffffff) // White text for selected
                } else {
                    rgb(0xcccccc) // Light gray for unselected
                };
                
                div()
                    .flex()
                    .items_center()
                    .w_full()
                    .px_3()
                    .py_2()
                    .bg(bg_color)
                    .hover(|style| style.bg(rgb(0x404040)))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                        this.handle_host_click(index, cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_color(text_color)
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(host.name.clone())
                            )
                            .child(
                                div()
                                    .text_color(rgb(0x888888))
                                    .text_size(px(12.0))
                                    .child(host.connection_string.clone())
                            )
                    )
            })
            .collect::<Vec<_>>();
        
        container.children(visible_hosts)
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
        self.host_list.set_hosts(filtered_hosts);
    }
    
    fn launch_host(&self, host: &HostEntry) -> Result<()> {
        use std::process::Command;
        
        // Simple terminal launch for now
        Command::new("osascript")
            .arg("-e")
            .arg(format!("tell app \"Terminal\" to do script \"{}\"", host.connection_string))
            .spawn()?;
        
        Ok(())
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
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .p_4()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                this.handle_key_event(event, cx);
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .text_color(rgb(0xffffff))
                            .text_size(px(24.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Trident SSH Launcher")
                    )
                    .child(self.search_input.clone())
                    .child(self.render_host_list(cx))
            )
    }
}

#[cfg(not(test))]
fn main() -> Result<()> {
    Application::new().run(|cx: &mut App| {
        let _ = cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Trident SSH Launcher".into()),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
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
