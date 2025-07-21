// ABOUTME: Native NSTableView-based host list component
// ABOUTME: Replaces GPUI HostList with native macOS table view and selection handling

use crate::ssh::parser::HostEntry;
use anyhow::Result;
use std::sync::{Arc, RwLock};

#[cfg(target_os = "macos")]
use objc2_app_kit::{NSTableView, NSScrollView, NSView};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSString, MainThreadMarker, NSRect, NSInteger, NSObject};
#[cfg(target_os = "macos")]
use objc2::{rc::Retained, runtime::AnyObject, MainThreadOnly};

// Shared state for the host list
#[derive(Clone, Debug)]
pub struct HostListState {
    pub hosts: Vec<HostEntry>,
    pub selected_index: usize,
}

impl HostListState {
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

    pub fn select_index(&mut self, index: usize) {
        if index < self.hosts.len() {
            self.selected_index = index;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.hosts.is_empty()
    }
}

// Native macOS host list using NSTableView
pub struct NativeHostList {
    #[cfg(target_os = "macos")]
    scroll_view: Option<Retained<NSScrollView>>,
    #[cfg(target_os = "macos")]
    table_view: Option<Retained<NSTableView>>,
    state: Arc<RwLock<HostListState>>,
    // Callback for when selection changes
    on_selection_change: Option<Box<dyn Fn(usize) + Send + Sync>>,
    // Callback for when host is double-clicked
    on_host_activate: Option<Box<dyn Fn(&HostEntry) + Send + Sync>>,
}

impl NativeHostList {
    pub fn new(hosts: Vec<HostEntry>) -> Self {
        let state = Arc::new(RwLock::new(HostListState::new(hosts)));
        
        Self {
            #[cfg(target_os = "macos")]
            scroll_view: None,
            #[cfg(target_os = "macos")]
            table_view: None,
            state,
            on_selection_change: None,
            on_host_activate: None,
        }
    }

    pub fn set_selection_change_callback<F>(&mut self, callback: F)
    where
        F: Fn(usize) + Send + Sync + 'static,
    {
        self.on_selection_change = Some(Box::new(callback));
    }

    pub fn set_host_activate_callback<F>(&mut self, callback: F)
    where
        F: Fn(&HostEntry) + Send + Sync + 'static,
    {
        self.on_host_activate = Some(Box::new(callback));
    }

    pub fn get_state(&self) -> Arc<RwLock<HostListState>> {
        self.state.clone()
    }

    #[cfg(target_os = "macos")]
    pub fn create_native_view(&mut self, frame: NSRect) -> Result<Retained<NSScrollView>> {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            
            // Create NSScrollView to contain the table
            let scroll_view = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), frame);
            
            // Configure scroll view
            scroll_view.setHasVerticalScroller(true);
            scroll_view.setHasHorizontalScroller(false);
            scroll_view.setAutohidesScrollers(true);
            scroll_view.setBorderType(objc2_app_kit::NSBorderType::BezelBorder);
            
            // Create NSTableView with basic frame
            let table_frame = objc2_foundation::NSRect::new(
                objc2_foundation::NSPoint::new(0.0, 0.0),
                objc2_foundation::NSSize::new(frame.size.width, frame.size.height)
            );
            let table_view = NSTableView::initWithFrame(NSTableView::alloc(mtm), table_frame);
            
            // Configure table view basic properties
            table_view.setRowHeight(32.0);
            
            // Set the table view as the document view of the scroll view
            scroll_view.setDocumentView(Some(&table_view));
            
            // Store references
            self.scroll_view = Some(scroll_view.clone());
            self.table_view = Some(table_view);
            
            Ok(scroll_view)
        }
    }

    #[cfg(target_os = "macos")]
    pub fn get_scroll_view(&self) -> Option<&Retained<NSScrollView>> {
        self.scroll_view.as_ref()
    }

    #[cfg(target_os = "macos")]
    pub fn get_table_view(&self) -> Option<&Retained<NSTableView>> {
        self.table_view.as_ref()
    }

    pub fn update_hosts(&self, hosts: Vec<HostEntry>) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            state.set_hosts(hosts);
        }
        
        // Reload the table view data
        #[cfg(target_os = "macos")]
        if let Some(table_view) = &self.table_view {
            unsafe {
                table_view.reloadData();
            }
        }
        
        Ok(())
    }

    pub fn select_next(&self) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            state.select_next();
        }
        
        // Update table view selection
        #[cfg(target_os = "macos")]
        if let Some(table_view) = &self.table_view {
            let state = self.state.read().unwrap();
            unsafe {
                table_view.selectRowIndexes_byExtendingSelection(
                    &objc2_foundation::NSIndexSet::indexSetWithIndex(state.selected_index as usize),
                    false
                );
                table_view.scrollRowToVisible(state.selected_index as isize);
            }
        }
        
        // Trigger callback
        if let Some(ref callback) = self.on_selection_change {
            let state = self.state.read().unwrap();
            callback(state.selected_index);
        }
        
        Ok(())
    }

    pub fn select_previous(&self) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            state.select_previous();
        }
        
        // Update table view selection
        #[cfg(target_os = "macos")]
        if let Some(table_view) = &self.table_view {
            let state = self.state.read().unwrap();
            unsafe {
                table_view.selectRowIndexes_byExtendingSelection(
                    &objc2_foundation::NSIndexSet::indexSetWithIndex(state.selected_index as usize),
                    false
                );
                table_view.scrollRowToVisible(state.selected_index as isize);
            }
        }
        
        // Trigger callback
        if let Some(ref callback) = self.on_selection_change {
            let state = self.state.read().unwrap();
            callback(state.selected_index);
        }
        
        Ok(())
    }

    pub fn activate_selected_host(&self) -> Result<()> {
        let state = self.state.read().unwrap();
        if let Some(host) = state.get_selected_host() {
            if let Some(ref callback) = self.on_host_activate {
                callback(host);
            }
        }
        Ok(())
    }

    pub fn set_selected_index(&self, index: usize) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            state.select_index(index);
        }
        
        // Update table view selection
        #[cfg(target_os = "macos")]
        if let Some(table_view) = &self.table_view {
            unsafe {
                table_view.selectRowIndexes_byExtendingSelection(
                    &objc2_foundation::NSIndexSet::indexSetWithIndex(index as usize),
                    false
                );
                table_view.scrollRowToVisible(index as isize);
            }
        }
        
        // Trigger callback
        if let Some(ref callback) = self.on_selection_change {
            callback(index);
        }
        
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub fn set_frame(&self, frame: NSRect) -> Result<()> {
        if let Some(scroll_view) = &self.scroll_view {
            unsafe {
                scroll_view.setFrame(frame);
            }
        }
        Ok(())
    }

    // Non-macOS stub implementations
    #[cfg(not(target_os = "macos"))]
    pub fn create_native_view(&mut self, _frame: (f64, f64, f64, f64)) -> Result<()> {
        Ok(())
    }

    pub fn get_selected_host(&self) -> Option<HostEntry> {
        let state = self.state.read().unwrap();
        state.get_selected_host().cloned()
    }

    pub fn is_empty(&self) -> bool {
        let state = self.state.read().unwrap();
        state.is_empty()
    }

    pub fn host_count(&self) -> usize {
        let state = self.state.read().unwrap();
        state.hosts.len()
    }
}

// TODO: Implement proper NSTableView data source when objc2 API is stable

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_host_list_creation() {
        let hosts = vec![
            HostEntry::new("test1".to_string(), "ssh test1".to_string()),
            HostEntry::new("test2".to_string(), "ssh test2".to_string()),
        ];
        
        let host_list = NativeHostList::new(hosts.clone());
        let state = host_list.state.read().unwrap();
        
        assert_eq!(state.hosts.len(), 2);
        assert_eq!(state.selected_index, 0);
        assert!(!state.is_empty());
    }

    #[test]
    fn test_host_list_navigation() {
        let hosts = vec![
            HostEntry::new("host1".to_string(), "ssh host1".to_string()),
            HostEntry::new("host2".to_string(), "ssh host2".to_string()),
            HostEntry::new("host3".to_string(), "ssh host3".to_string()),
        ];
        
        let host_list = NativeHostList::new(hosts);
        
        // Test next selection
        host_list.select_next().unwrap();
        {
            let state = host_list.state.read().unwrap();
            assert_eq!(state.selected_index, 1);
        }
        
        // Test previous selection
        host_list.select_previous().unwrap();
        {
            let state = host_list.state.read().unwrap();
            assert_eq!(state.selected_index, 0);
        }
    }

    #[test]
    fn test_host_list_state_operations() {
        let mut state = HostListState::new(vec![]);
        
        // Test empty state
        assert!(state.is_empty());
        assert!(state.get_selected_host().is_none());
        
        // Test with hosts
        let hosts = vec![
            HostEntry::new("host1".to_string(), "ssh host1".to_string()),
            HostEntry::new("host2".to_string(), "ssh host2".to_string()),
        ];
        
        state.set_hosts(hosts);
        assert!(!state.is_empty());
        assert_eq!(state.get_selected_host().unwrap().name, "host1");
        
        // Test selection
        state.select_next();
        assert_eq!(state.get_selected_host().unwrap().name, "host2");
        
        state.select_previous();
        assert_eq!(state.get_selected_host().unwrap().name, "host1");
    }
}