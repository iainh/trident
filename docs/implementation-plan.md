# SSH Connection Launcher - macOS MVP Plan

## Project Overview
A macOS SSH connection launcher built with Rust and GPUI that provides fuzzy search over known hosts and launches terminal connections. User-configurable for simplicity.

## Core Architecture (macOS MVP)

### 1. Simplified Application Structure
```
ssh-launcher/
├── src/
│   ├── main.rs              # Application entry point
│   ├── app.rs               # Main application state and logic
│   ├── config.rs            # User configuration handling
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── search_box.rs    # Text input component
│   │   └── host_list.rs     # Results display component
│   ├── ssh/
│   │   ├── mod.rs
│   │   ├── parser.rs        # SSH file parsing (simplified)
│   │   └── launcher.rs      # Terminal launching
│   └── fuzzy.rs             # Simple fuzzy matching
├── config.toml              # User configuration file
├── Cargo.toml
└── README.md
```

## User Configuration (config.toml)

```toml
[terminal]
# User specifies their preferred terminal and how to invoke it
program = "/Applications/iTerm.app/Contents/MacOS/iTerm2"
args = ["-c", "tell application \"iTerm2\" to create window with default profile command \"{ssh_command}\""]

# Alternative examples user can choose from:
# Terminal.app:
# program = "/usr/bin/osascript"
# args = ["-e", "tell app \"Terminal\" to do script \"{ssh_command}\""]

[ssh]
# User specifies SSH file locations (defaults provided)
known_hosts_path = "~/.ssh/known_hosts"
config_path = "~/.ssh/config"
ssh_binary = "/usr/bin/ssh"

[parsing]
# User decides what to parse and how
parse_known_hosts = true
parse_ssh_config = true
# Simple: only parse Host entries, ignore Include directives
simple_config_parsing = true
# Skip hashed known_hosts entries (user can disable if needed)
skip_hashed_hosts = true

[ui]
# Simple UI preferences
max_results = 20
case_sensitive = false
```

## What We're Delegating to User Configuration

### 1. Terminal Integration
**User Specifies:**
- Exact terminal program path
- Command-line arguments for launching
- How to pass SSH command to terminal

**Benefits:**
- No terminal detection logic needed
- User gets exactly their preferred terminal
- Supports custom terminals and configurations
- User can test and configure their own setup

### 2. File Locations
**User Specifies:**
- SSH config file path
- Known hosts file path  
- SSH binary location

**Benefits:**
- No path discovery logic
- Works with non-standard SSH setups
- User controls which files to use
- Handles symlinks, custom locations naturally

### 3. Parsing Complexity
**User Chooses:**
- Whether to parse known_hosts, SSH config, or both
- Simple parsing mode (skip complex SSH config features)
- Whether to handle hashed known_hosts entries

**Benefits:**
- Avoid complex SSH config parsing edge cases
- User can disable problematic file types
- Incremental feature adoption

### 4. Search Behavior
**User Controls:**
- Maximum number of search results
- Case sensitivity
- Which sources to include in search

**Benefits:**
- Performance tuning by user
- Customizable search experience
- Easy to modify without code changes

## Simplified Implementation Phases

### Phase 1: Core MVP (Week 1)

#### 1.1 Basic Project Setup
- GPUI window with text input
- Simple config.toml loading
- Basic error handling

#### 1.2 Simple SSH Parsing
```rust
#[derive(Clone, Debug)]
pub struct HostEntry {
    pub name: String,           // What user types to match
    pub connection_string: String, // What gets passed to SSH
}

// Simple parsing - user configures what to parse
fn parse_simple_known_hosts(path: &str) -> Vec<HostEntry>
fn parse_simple_ssh_config(path: &str) -> Vec<HostEntry>
```

#### 1.3 Basic Fuzzy Search
- Simple substring matching
- Case-insensitive option from config
- Limit results by config setting

### Phase 2: UI Polish (Week 2)

#### 2.1 GPUI Interface
- Text input with real-time filtering
- Simple list display of results
- Keyboard navigation (up/down/enter)
- Basic styling

#### 2.2 Terminal Integration
- Read terminal config from user
- Simple string substitution for {ssh_command}
- Execute configured terminal command

### Phase 3: Configuration & Refinement

#### 3.1 Configuration Management
- Default config generation
- Config validation and error messages
- Runtime config reloading

#### 3.2 User Experience
- Clear error messages for configuration issues
- Simple setup wizard or documentation
- Basic logging for troubleshooting

## Key Dependencies (Simplified)

```toml
[dependencies]
gpui = "0.1"                    # UI framework
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"                   # Configuration parsing
dirs = "5.0"                   # For home directory
anyhow = "1.0"                 # Error handling
fuzzy-matcher = "0.3"          # Simple fuzzy matching
```

## What We're NOT Implementing (User Handles)

### 1. Terminal Detection
- User specifies exact terminal in config
- User provides the command-line invocation
- User tests their own terminal integration

### 2. Complex SSH Config Parsing
- No `Include` directive support
- No variable substitution (`%h`, `%p`)
- No `ProxyJump` parsing
- User can manually add complex hosts to config if needed

### 3. SSH Authentication Details
- User's SSH setup is assumed to work
- No SSH agent integration
- No key file management
- User handles authentication outside the app

### 4. File System Monitoring
- No real-time file watching
- User restarts app to pick up SSH file changes
- Static configuration on startup

### 5. Advanced UI Features
- No themes (simple default styling)
- No customizable keyboard shortcuts
- No host grouping or categorization
- No connection history

## Example User Configuration Scenarios

### iTerm2 User
```toml
[terminal]
program = "/Applications/iTerm.app/Contents/MacOS/iTerm2"
args = ["-c", "tell application \"iTerm2\" to create window with default profile command \"{ssh_command}\""]
```

### Terminal.app User  
```toml
[terminal]
program = "/usr/bin/osascript"
args = ["-e", "tell app \"Terminal\" to do script \"{ssh_command}\""]
```

### Custom SSH Setup User
```toml
[ssh]
known_hosts_path = "/custom/path/known_hosts"
config_path = "/custom/path/ssh_config"
ssh_binary = "/usr/local/bin/ssh"
```

This approach lets you build a working MVP much faster while still being useful. Users who need the advanced features can contribute them later or configure workarounds. The configuration-driven approach also makes the tool more flexible than trying to auto-detect everything.

