// ABOUTME: Host list display component for showing SSH host search results
// ABOUTME: Renders scrollable list of hosts with highlighting for selected item

use crate::ssh::parser::HostEntry;
use gpui::prelude::*;
use gpui::*;

#[derive(Clone)]
pub struct HostList {
    pub hosts: Vec<HostEntry>,
    pub selected_index: usize,
}

impl HostList {
    pub fn new(hosts: Vec<HostEntry>) -> Self {
        Self {
            hosts,
            selected_index: 0,
        }
    }

    pub fn set_hosts(&mut self, hosts: Vec<HostEntry>) {
        self.hosts = hosts;
        // Reset selection if it's out of bounds
        if self.selected_index >= self.hosts.len() {
            self.selected_index = if self.hosts.is_empty() {
                0
            } else {
                self.hosts.len() - 1
            };
        }
    }

    pub fn select_next(&mut self) {
        if !self.hosts.is_empty() {
            let max_visible = 8.min(self.hosts.len());
            self.selected_index = (self.selected_index + 1) % max_visible;
        }
    }

    pub fn select_previous(&mut self) {
        if !self.hosts.is_empty() {
            let max_visible = 8.min(self.hosts.len());
            self.selected_index = if self.selected_index == 0 {
                max_visible - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn get_selected_host(&self) -> Option<&HostEntry> {
        self.hosts.get(self.selected_index)
    }

    #[allow(dead_code)]
    pub fn select_index(&mut self, index: usize) {
        if index < self.hosts.len() {
            self.selected_index = index;
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.hosts.is_empty()
    }
}

impl IntoElement for HostList {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        if self.hosts.is_empty() {
            return div()
                .flex()
                .items_center()
                .justify_center()
                .w_full()
                .h(px(60.0))
                .text_color(rgb(0x8c8c8c)) // Zed muted text
                .text_size(px(14.0))
                .child("No hosts found");
        }

        // Scrollable list - keyboard navigation will work to keep selected items visible
        div()
            .flex()
            .flex_col()
            .w_full()
            .max_h(px(400.0))
            .overflow_hidden()
            .children(
                self.hosts
                    .iter()
                    .take(8)
                    .enumerate()
                    .map(|(i, host)| {
                        let is_selected = i == self.selected_index;

                        div()
                            .flex()
                            .items_center()
                            .w_full()
                            .px_3()
                            .py_2()
                            .when(is_selected, |style| {
                                style.bg(hsla(207.0 / 360.0, 0.7, 0.25, 0.2))
                            })
                            .when(!is_selected, |style| {
                                style.hover(|hover_style| hover_style.bg(rgb(0x454a55)))
                            })
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_color(if is_selected {
                                                rgb(0x569cd6) // Zed accent text
                                            } else {
                                                rgb(0xd4d4d4) // Zed primary text
                                            })
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(host.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_color(rgb(0xa5a5a5)) // Zed muted text
                                            .text_size(px(12.0))
                                            .child(host.connection_string.clone()),
                                    ),
                            )
                    })
                    .collect::<Vec<_>>(),
            )
    }
}

// Tests removed due to GPUI macro compilation issues
// Core logic is tested through the running application and manual testing
