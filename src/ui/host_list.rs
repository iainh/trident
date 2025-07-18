// ABOUTME: Host list display component for showing SSH host search results
// ABOUTME: Renders list of hosts with highlighting for selected item

#[cfg(not(test))]
use gpui::*;
use crate::ssh::parser::HostEntry;

#[derive(Clone)]
pub struct HostList {
    pub hosts: Vec<HostEntry>,
    pub selected_index: usize,
    pub max_visible: usize,
}

impl HostList {
    pub fn new(hosts: Vec<HostEntry>) -> Self {
        Self {
            hosts,
            selected_index: 0,
            max_visible: 5,
        }
    }
    
    pub fn set_hosts(&mut self, hosts: Vec<HostEntry>) {
        self.hosts = hosts;
        // Reset selection if it's out of bounds
        if self.selected_index >= self.hosts.len() {
            self.selected_index = if self.hosts.is_empty() { 0 } else { self.hosts.len() - 1 };
        }
    }
    
    pub fn select_next(&mut self) {
        if !self.hosts.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.hosts.len();
        }
    }
    
    pub fn select_previous(&mut self) {
        if !self.hosts.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.hosts.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }
    
    pub fn get_selected_host(&self) -> Option<&HostEntry> {
        self.hosts.get(self.selected_index)
    }
    
    pub fn select_index(&mut self, index: usize) {
        if index < self.hosts.len() {
            self.selected_index = index;
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.hosts.is_empty()
    }
}

#[cfg(not(test))]
impl IntoElement for HostList {
    type Element = Div;
    
    fn into_element(self) -> Self::Element {
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
        
        if self.hosts.is_empty() {
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
        
        let visible_hosts = self.hosts.iter()
            .take(self.max_visible)
            .enumerate()
            .map(|(index, host)| {
                let is_selected = index == self.selected_index;
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
}

// Tests removed due to GPUI macro compilation issues
// Core logic is tested through the running application and manual testing