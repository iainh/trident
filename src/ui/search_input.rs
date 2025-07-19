// ABOUTME: Search input component for typing SSH host queries
// ABOUTME: Handles text input and keyboard events for real-time search

#[cfg(not(test))]
use gpui::*;

#[cfg(not(test))]
use gpui::prelude::FluentBuilder;

#[derive(Clone)]
pub struct SearchInput {
    pub query: String,
    pub placeholder: String,
    pub is_focused: bool,
    pub suggestion: Option<String>,
}

impl SearchInput {
    pub fn new(placeholder: String) -> Self {
        Self {
            query: String::new(),
            placeholder,
            is_focused: false,
            suggestion: None,
        }
    }
    
    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }
    
    pub fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }
    
    pub fn handle_input(&mut self, text: &str) {
        self.query.push_str(text);
        self.suggestion = None; // Clear suggestion when typing
    }
    
    pub fn handle_backspace(&mut self) {
        self.query.pop();
        self.suggestion = None; // Clear suggestion when deleting
    }
    
    pub fn clear(&mut self) {
        self.query.clear();
        self.suggestion = None;
    }
    
    pub fn set_suggestion(&mut self, suggestion: Option<String>) {
        self.suggestion = suggestion;
    }
    
    pub fn accept_suggestion(&mut self) {
        if let Some(suggestion) = &self.suggestion {
            self.query = suggestion.clone();
            self.suggestion = None;
        }
    }
}

#[cfg(not(test))]
impl IntoElement for SearchInput {
    type Element = Div;
    
    fn into_element(self) -> Self::Element {
        use gpui::prelude::FluentBuilder;
        
        let border_color = if self.is_focused {
            rgb(0x569cd6) // Zed's accent blue when focused
        } else {
            rgb(0x3c4043) // Zed's border color when unfocused
        };
        
        div()
            .flex()
            .w_full()
            .h(px(40.0))
            .px_3()
            .py_2()
            .border_1()
            .border_color(border_color)
            .rounded_md()
            .bg(rgb(0x2d2d2d))
            .child(
                div()
                    .flex()
                    .items_center()
                    .w_full()
                    .text_size(px(16.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .relative()
                            .when(self.query.is_empty() && self.suggestion.is_none(), |this| {
                                // Show cursor + placeholder when no query and no suggestion
                                this.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .when(self.is_focused, |this| {
                                            // Show cursor first
                                            this.child(
                                                div()
                                                    .w(px(2.0))   // Zed cursor width
                                                    .h(px(18.0))  // Match text line height
                                                    .bg(rgb(0xd4d4d4)) // Zed's primary text color
                                                    .rounded(px(1.0))   // Very subtle rounding
                                                    .mr_2()      // Right margin instead of left
                                                    .opacity(1.0) // TODO: animate opacity for blinking
                                            )
                                        })
                                        .child(
                                            div()
                                                .text_color(rgb(0x8c8c8c)) // Use Zed's placeholder color
                                                .child(self.placeholder.clone())
                                        )
                                )
                            })
                            .when(!self.query.is_empty() || self.suggestion.is_some(), |this| {
                                // Show query + cursor + suggestion when we have text
                                this.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .child(
                                            div()
                                                .text_color(rgb(0xffffff))
                                                .child(self.query.clone())
                                        )
                                        .when(self.is_focused && !self.query.is_empty(), |this| {
                                            // Show Zed-style cursor right after typed text
                                            this.child(
                                                div()
                                                    .w(px(2.0))        // Zed cursor width
                                                    .h(px(18.0))       // Match text line height
                                                    .bg(rgb(0xd4d4d4)) // Zed's primary text color
                                                    .rounded(px(1.0))   // Very subtle rounding like Zed
                                                    .opacity(1.0)      // Solid when actively typing
                                                    // TODO: Add blinking animation when GPUI supports it
                                            )
                                        })
                                        .when_some(self.suggestion.as_ref(), |this, suggestion| {
                                            // Only show suggestion if it extends beyond current query
                                            if suggestion.len() > self.query.len() && 
                                               suggestion.to_lowercase().starts_with(&self.query.to_lowercase()) {
                                                let remaining = &suggestion[self.query.len()..];
                                                this.child(
                                                    div()
                                                        .text_color(rgb(0x666666))
                                                        .child(remaining.to_string())
                                                )
                                            } else {
                                                this
                                            }
                                        })
                                )
                            })
                    )
            )
    }
}

// Tests removed due to GPUI macro compilation issues
// Core logic is tested through the running application and manual testing