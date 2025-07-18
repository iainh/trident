# SSH Launcher - Menu Bar App Strategy

## Why Menu Bar + Global Hotkey is Perfect

### Performance Benefits
- **Zero startup time**: App loads once at login, window appears instantly
- **Always-indexed hosts**: SSH files parsed once, kept in memory
- **Immediate response**: No file I/O on hotkey press
- **Background updates**: Can watch SSH files and update index without user intervention

### User Experience Benefits
- **Spotlight-like workflow**: Press hotkey anywhere, type, connect, back to work
- **No context switching**: Don't need to find/launch app in dock
- **Muscle memory**: Same hotkey from any application
- **Minimal visual footprint**: Just a small menu bar icon

## Architecture Changes for Menu Bar App

### 1. Application Lifecycle
```rust
// Previous: Traditional window app
fn main() {
    let app = App::new();
    app.run_with_window(|| AppWindow::new());
}

// New: Menu bar app with hidden window
fn main() {
    let app = App::new();
    app.set_activation_policy(ActivationPolicy::Accessory); // No dock icon
    
    // Create hidden window
    let window = app.new_window(WindowOptions {
        is_visible: false,
        titlebar: None,
        frame: false,
        level: WindowLevel::PopUp, // Above other windows
    });
    
    // Create menu bar item
    let menu_bar = MenuBar::new()
        .with_icon(include_bytes!("../assets/ssh-icon.png"))
        .with_tooltip("SSH Launcher");
    
    // Register global hotkey
    GlobalHotKey::register("cmd+shift+s", || {
        window.show_and_focus();
    });
    
    app.run();
}
```

### 2. Window Behavior Changes
```rust
pub struct LauncherWindow {
    state: AppState,
    is_visible: bool,
    position: WindowPosition,
}

impl LauncherWindow {
    pub fn show_and_focus(&mut self) {
        // Reset state when showing
        self.state.reset_search();
        
        // Position near cursor or center of active screen
        self.position = self.calculate_optimal_position();
        
        // Show with animation
        self.animate_in();
        self.focus_search_input();
    }
    
    pub fn hide(&mut self) {
        // Hide on Escape, Enter, or lost focus
        self.animate_out();
        self.is_visible = false;
    }
    
    fn calculate_optimal_position(&self) -> WindowPosition {
        // Option 1: Center of active display
        let display = Screen::main_display();
        WindowPosition::center_of(display)
        
        // Option 2: Near mouse cursor (Spotlight-style)
        // let cursor = Mouse::global_position();
        // WindowPosition::near_point(cursor, offset: (0, -100))
    }
}
```

### 3. Enhanced State Management
```rust
pub struct MenuBarApp {
    // Core SSH data - loaded once, updated in background
    host_index: Arc<RwLock<SearchIndex>>,
    config: Arc<RwLock<Config>>,
    
    // UI state - reset on each show
    window_state: LauncherWindowState,
    
    // Background services
    file_watcher: FileWatcher,
    hotkey_manager: GlobalHotKeyManager,
}

impl MenuBarApp {
    pub fn new() -> Self {
        let app = Self {
            host_index: Arc::new(RwLock::new(SearchIndex::build_from_config())),
            config: Arc::new(RwLock::new(Config::load_or_default())),
            window_state: LauncherWindowState::new(),
            file_watcher: FileWatcher::new(),
            hotkey_manager: GlobalHotKeyManager::new(),
        };
        
        // Start background services
        app.start_file_monitoring();
        app.register_global_hotkeys();
        
        app
    }
    
    fn start_file_monitoring(&self) {
        let index = Arc::clone(&self.host_index);
        let config = self.config.read().unwrap().clone();
        
        self.file_watcher.watch(vec![
            config.ssh.known_hosts_path,
            config.ssh.config_path,
        ], move |_event| {
            // Rebuild index in background
            let new_index = SearchIndex::build_from_config();
            *index.write().unwrap() = new_index;
        });
    }
}
```

## Key Implementation Considerations

### 1. Global Hotkey Management
```rust
// User configurable hotkeys
[hotkeys]
show_launcher = "cmd+shift+s"
quick_connect_last = "cmd+shift+l"  # Connect to last used host

// Implementation challenges:
// - Conflict detection with system/app hotkeys
// - Registering/unregistering on config changes
// - Cross-platform hotkey format differences
```

### 2. Window Positioning & Behavior
```rust
pub enum WindowShowBehavior {
    CenterOfActiveScreen,
    NearMouseCursor { offset_x: i32, offset_y: i32 },
    RememberLastPosition,
    ConfigurablePosition { x: i32, y: i32 },
}

pub struct WindowAppearance {
    show_behavior: WindowShowBehavior,
    animation: ShowAnimation, // Fade, Slide, Scale, None
    auto_hide_on_focus_lost: bool,
    always_on_top: bool,
}
```

### 3. Menu Bar Icon & Context Menu
```rust
pub struct MenuBarIcon {
    icon: IconState,
    context_menu: ContextMenu,
}

pub enum IconState {
    Default,
    Connecting,      // Show spinner when launching connection
    Error,           // Red tint when SSH files can't be parsed
    Disabled,        // Grayed when no hosts found
}

pub struct ContextMenu {
    items: Vec<MenuItem>,
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self {
            items: vec![
                MenuItem::action("Show Launcher", show_launcher_action),
                MenuItem::separator(),
                MenuItem::submenu("Recent Connections", recent_connections_menu()),
                MenuItem::separator(),
                MenuItem::action("Reload SSH Config", reload_config_action),
                MenuItem::action("Preferences...", show_preferences_action),
                MenuItem::separator(),
                MenuItem::action("Quit", quit_action),
            ],
        }
    }
}
```

### 4. Focus & Window Management
```rust
impl LauncherWindow {
    fn handle_focus_lost(&mut self) {
        if self.config.auto_hide_on_focus_lost {
            self.hide();
        }
    }
    
    fn handle_escape_key(&mut self) {
        self.hide();
    }
    
    fn handle_enter_key(&mut self) {
        if let Some(selected_host) = self.get_selected_host() {
            self.launch_connection(selected_host);
            self.hide(); // Hide after launching
        }
    }
}
```

## Enhanced Features Enabled by Always-Running

### 1. Background SSH File Monitoring
```rust
// Real-time updates without user intervention
impl FileWatcher {
    fn on_ssh_config_changed(&self, path: &Path) {
        log::info!("SSH config changed: {:?}", path);
        
        // Parse in background thread
        let new_hosts = parse_ssh_files_async();
        
        // Update search index atomically
        self.update_search_index(new_hosts);
        
        // Show notification if significant changes
        if self.should_notify_user() {
            Notification::show("SSH config updated - new hosts available");
        }
    }
}
```

### 2. Connection History & Analytics
```rust
pub struct ConnectionHistory {
    recent_connections: VecDeque<HostConnection>,
    frequency_map: HashMap<String, u32>,
    last_connection_time: HashMap<String, SystemTime>,
}

impl ConnectionHistory {
    pub fn get_smart_suggestions(&self, query: &str) -> Vec<HostEntry> {
        // Boost frequently used hosts in search results
        // Prioritize recently used hosts
        // Learn from user patterns
    }
    
    pub fn add_connection(&mut self, host: &str) {
        self.recent_connections.push_front(HostConnection::new(host));
        *self.frequency_map.entry(host.to_string()).or_insert(0) += 1;
        self.last_connection_time.insert(host.to_string(), SystemTime::now());
    }
}
```

### 3. Advanced Search Features
```rust
// Enabled by always-running indexing
pub struct EnhancedSearchIndex {
    // Pre-computed fuzzy search index
    fuzzy_index: FuzzyIndex,
    
    // Frequency-based ranking
    usage_weights: HashMap<String, f32>,
    
    // Tag-based categorization
    host_tags: HashMap<String, Vec<String>>,
    
    // Cached search results
    search_cache: LruCache<String, Vec<HostEntry>>,
}

impl EnhancedSearchIndex {
    pub fn search_with_learning(&mut self, query: &str) -> Vec<HostEntry> {
        // Check cache first
        if let Some(cached) = self.search_cache.get(query) {
            return cached.clone();
        }
        
        // Fuzzy search with frequency boosting
        let mut results = self.fuzzy_index.search(query);
        
        // Apply usage-based ranking
        results.sort_by(|a, b| {
            let weight_a = self.usage_weights.get(&a.name).unwrap_or(&0.0);
            let weight_b = self.usage_weights.get(&b.name).unwrap_or(&0.0);
            weight_b.partial_cmp(weight_a).unwrap_or(Ordering::Equal)
        });
        
        // Cache results
        self.search_cache.put(query.to_string(), results.clone());
        
        results
    }
}
```

## Configuration Updates

### Enhanced Config for Menu Bar App
```toml
[hotkeys]
show_launcher = "cmd+shift+s"
quick_connect_recent = "cmd+shift+r"

[window]
show_behavior = "center_of_active_screen"  # or "near_cursor"
animation = "fade"  # fade, slide, scale, none
auto_hide_on_focus_lost = true
always_on_top = true
width = 600
height = 400

[menubar]
show_icon = true
icon_style = "monochrome"  # monochrome, colored
show_recent_in_menu = true
max_recent_items = 10

[background]
monitor_ssh_files = true
update_frequency = "real_time"  # real_time, periodic, manual
cache_search_results = true
learn_from_usage = true

[notifications]
notify_on_config_changes = true
notify_on_connection_errors = false
```

## Technical Challenges & Solutions

### 1. Global Hotkey Conflicts
**Problem**: User's hotkey might conflict with other apps
**Solution**: 
- Conflict detection on registration
- Fallback hotkey suggestions
- Easy reconfiguration UI

### 2. Memory Usage
**Problem**: Always-running app must be lightweight
**Solution**:
- Lazy loading of search index
- Periodic memory cleanup
- LRU caches with size limits

### 3. macOS Permissions
**Problem**: Global hotkeys require accessibility permissions
**Solution**:
- Clear permission request flow
- Graceful degradation if denied
- Menu bar access as fallback

### 4. Window Focus Behavior
**Problem**: Managing focus across different spaces/desktops
**Solution**:
- Respect current space/desktop
- Don't force space switching
- Smart positioning based on active screen

## Implementation Priority

### Phase 1: Basic Menu Bar App
1. Menu bar icon with context menu
2. Global hotkey registration
3. Show/hide window behavior
4. Basic window positioning

### Phase 2: Enhanced Behavior  
1. File watching and background updates
2. Focus management and auto-hide
3. Connection history tracking
4. Smart search ranking

### Phase 3: Polish & Advanced Features
1. Window animations
2. Notification system
3. Advanced hotkey management
4. Usage analytics and learning

This menu bar approach transforms your SSH launcher from a "tool you open" into a "superpower you invoke" - exactly what developer tools should be.
