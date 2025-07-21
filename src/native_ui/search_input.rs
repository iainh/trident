// ABOUTME: Native NSTextField-based search input component
// ABOUTME: Replaces GPUI SearchInput with native macOS text field and keyboard handling

use anyhow::Result;
use std::sync::{Arc, RwLock};

#[cfg(target_os = "macos")]
use objc2_app_kit::{NSTextField, NSView, NSControl, NSText};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSString, MainThreadMarker, NSRect, NSPoint, NSSize};
#[cfg(target_os = "macos")]
use objc2::{rc::Retained, runtime::AnyObject, msg_send_id, sel, MainThreadOnly};

// Shared state for the search input
#[derive(Clone, Debug)]
pub struct SearchInputState {
    pub query: String,
    pub placeholder: String,
    pub is_focused: bool,
    pub suggestion: Option<String>,
}

impl SearchInputState {
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
        self.suggestion = None;
    }

    pub fn handle_backspace(&mut self) {
        self.query.pop();
        self.suggestion = None;
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

// Native macOS search input using NSTextField
pub struct NativeSearchInput {
    #[cfg(target_os = "macos")]
    text_field: Option<Retained<NSTextField>>,
    state: Arc<RwLock<SearchInputState>>,
    // Callback for when text changes
    on_text_change: Option<Box<dyn Fn(&str) + Send + Sync>>,
}

impl NativeSearchInput {
    pub fn new(placeholder: String) -> Self {
        let state = Arc::new(RwLock::new(SearchInputState::new(placeholder)));
        
        Self {
            #[cfg(target_os = "macos")]
            text_field: None,
            state,
            on_text_change: None,
        }
    }

    pub fn set_text_change_callback<F>(&mut self, callback: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.on_text_change = Some(Box::new(callback));
    }

    pub fn get_state(&self) -> Arc<RwLock<SearchInputState>> {
        self.state.clone()
    }

    #[cfg(target_os = "macos")]
    pub fn create_native_view(&mut self, frame: NSRect) -> Result<Retained<NSTextField>> {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            
            // Create NSTextField
            let text_field = NSTextField::initWithFrame(NSTextField::alloc(mtm), frame);
            
            // Configure the text field
            let placeholder_text = {
                let state = self.state.read().unwrap();
                NSString::from_str(&state.placeholder)
            };
            text_field.setPlaceholderString(Some(&placeholder_text));
            
            // Set styling for dark mode appearance
            text_field.setBordered(true);
            
            // Make it focusable and editable
            text_field.setEditable(true);
            text_field.setSelectable(true);
            
            // Store reference
            self.text_field = Some(text_field.clone());
            
            Ok(text_field)
        }
    }

    #[cfg(target_os = "macos")]
    pub fn get_text_field(&self) -> Option<&Retained<NSTextField>> {
        self.text_field.as_ref()
    }

    #[cfg(target_os = "macos")]
    pub fn update_text(&self, new_text: &str) -> Result<()> {
        if let Some(text_field) = &self.text_field {
            unsafe {
                let ns_string = NSString::from_str(new_text);
                text_field.setStringValue(&ns_string);
            }
        }
        
        // Update state
        {
            let mut state = self.state.write().unwrap();
            state.query = new_text.to_string();
        }
        
        // Trigger callback
        if let Some(ref callback) = self.on_text_change {
            callback(new_text);
        }
        
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub fn get_text(&self) -> String {
        if let Some(text_field) = &self.text_field {
            unsafe {
                let ns_string = text_field.stringValue();
                ns_string.to_string()
            }
        } else {
            let state = self.state.read().unwrap();
            state.query.clone()
        }
    }

    #[cfg(target_os = "macos")]
    pub fn focus(&self) -> Result<()> {
        if let Some(text_field) = &self.text_field {
            unsafe {
                // Make the text field the first responder to focus it
                if let Some(window) = text_field.window() {
                    window.makeFirstResponder(Some(text_field));
                }
            }
        }
        
        // Update state
        {
            let mut state = self.state.write().unwrap();
            state.is_focused = true;
        }
        
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub fn set_frame(&self, frame: NSRect) -> Result<()> {
        if let Some(text_field) = &self.text_field {
            unsafe {
                text_field.setFrame(frame);
            }
        }
        Ok(())
    }

    // Non-macOS stub implementations
    #[cfg(not(target_os = "macos"))]
    pub fn create_native_view(&mut self, _frame: (f64, f64, f64, f64)) -> Result<()> {
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn update_text(&self, new_text: &str) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.query = new_text.to_string();
        
        if let Some(ref callback) = self.on_text_change {
            callback(new_text);
        }
        
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn get_text(&self) -> String {
        let state = self.state.read().unwrap();
        state.query.clone()
    }

    #[cfg(not(target_os = "macos"))]
    pub fn focus(&self) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.is_focused = true;
        Ok(())
    }

    pub fn handle_key_event(&self, key: &str) -> Result<bool> {
        match key {
            "backspace" => {
                let mut state = self.state.write().unwrap();
                state.handle_backspace();
                
                // Update the native text field
                #[cfg(target_os = "macos")]
                if let Some(text_field) = &self.text_field {
                    unsafe {
                        let ns_string = NSString::from_str(&state.query);
                        text_field.setStringValue(&ns_string);
                    }
                }
                
                // Trigger callback
                if let Some(ref callback) = self.on_text_change {
                    callback(&state.query);
                }
                
                Ok(true) // Event handled
            }
            "tab" => {
                let mut state = self.state.write().unwrap();
                state.accept_suggestion();
                
                // Update the native text field
                #[cfg(target_os = "macos")]
                if let Some(text_field) = &self.text_field {
                    unsafe {
                        let ns_string = NSString::from_str(&state.query);
                        text_field.setStringValue(&ns_string);
                    }
                }
                
                // Trigger callback
                if let Some(ref callback) = self.on_text_change {
                    callback(&state.query);
                }
                
                Ok(true) // Event handled
            }
            text if text.len() == 1 => {
                // Handle single character input
                if let Some(ch) = text.chars().next() {
                    if ch.is_ascii_graphic() || ch == ' ' {
                        let mut state = self.state.write().unwrap();
                        state.handle_input(text);
                        
                        // Update the native text field
                        #[cfg(target_os = "macos")]
                        if let Some(text_field) = &self.text_field {
                            unsafe {
                                let ns_string = NSString::from_str(&state.query);
                                text_field.setStringValue(&ns_string);
                            }
                        }
                        
                        // Trigger callback
                        if let Some(ref callback) = self.on_text_change {
                            callback(&state.query);
                        }
                        
                        return Ok(true); // Event handled
                    }
                }
                Ok(false) // Event not handled
            }
            _ => Ok(false) // Event not handled
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_search_input_creation() {
        let search_input = NativeSearchInput::new("Test placeholder".to_string());
        let state = search_input.state.read().unwrap();
        
        assert_eq!(state.placeholder, "Test placeholder");
        assert!(state.query.is_empty());
        assert!(!state.is_focused);
        assert!(state.suggestion.is_none());
    }

    #[test]
    fn test_search_input_state_operations() {
        let mut state = SearchInputState::new("Test".to_string());
        
        // Test input handling
        state.handle_input("hello");
        assert_eq!(state.query, "hello");
        
        // Test backspace
        state.handle_backspace();
        assert_eq!(state.query, "hell");
        
        // Test suggestion
        state.set_suggestion(Some("hello world".to_string()));
        assert_eq!(state.suggestion, Some("hello world".to_string()));
        
        // Test accepting suggestion
        state.accept_suggestion();
        assert_eq!(state.query, "hello world");
        assert!(state.suggestion.is_none());
        
        // Test clear
        state.clear();
        assert!(state.query.is_empty());
        assert!(state.suggestion.is_none());
    }

    #[test]
    fn test_key_event_handling() {
        let search_input = NativeSearchInput::new("Test".to_string());
        
        // Test character input
        search_input.handle_key_event("a").unwrap();
        let state = search_input.state.read().unwrap();
        assert_eq!(state.query, "a");
        drop(state);
        
        // Test backspace
        search_input.handle_key_event("backspace").unwrap();
        let state = search_input.state.read().unwrap();
        assert!(state.query.is_empty());
    }
}