# SSH Launcher - TDD Testing Strategy

## Testing Philosophy for GUI Applications

The key insight for TDD with GUI apps is to **separate business logic from presentation logic** and test them independently. We'll use a Model-View-Update (MVU) pattern where:

- **Model**: Application state (testable)
- **Update**: State transitions (testable) 
- **View**: GPUI rendering (integration tested)

## Success Metrics as Test Requirements

### Performance Metrics
- **Startup time < 500ms**: Benchmark tests
- **Search response < 50ms**: Unit tests with timing
- **Responsive UI**: Integration tests with simulated input

### Functional Metrics  
- **Accurate fuzzy search**: Property-based tests
- **Reliable terminal launching**: Mock-based tests
- **Keyboard-driven workflow**: Event simulation tests

## Test Architecture

### 1. Core Domain Logic (Unit Tests - TDD)

```rust
// tests/unit/search_engine_test.rs
#[cfg(test)]
mod search_engine_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn empty_query_returns_all_hosts() {
        // Red: Write failing test first
        let hosts = vec![
            HostEntry::new("server1", "ssh user@server1.com"),
            HostEntry::new("server2", "ssh admin@server2.com"),
        ];
        let engine = SearchEngine::new(hosts);
        
        let results = engine.search("");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_performance_under_50ms() {
        // Red: This will fail initially
        let hosts = generate_test_hosts(1000); // Large dataset
        let engine = SearchEngine::new(hosts);
        
        let start = Instant::now();
        let results = engine.search("serv");
        let duration = start.elapsed();
        
        assert!(duration.as_millis() < 50, "Search took {}ms", duration.as_millis());
        assert!(!results.is_empty());
    }

    #[test]
    fn fuzzy_search_ranks_exact_matches_first() {
        let hosts = vec![
            HostEntry::new("production-server", "ssh prod"),
            HostEntry::new("server", "ssh server"),  // Exact match
            HostEntry::new("test-server", "ssh test"),
        ];
        let engine = SearchEngine::new(hosts);
        
        let results = engine.search("server");
        assert_eq!(results[0].name, "server"); // Exact match first
    }

    #[test]
    fn case_insensitive_search_when_configured() {
        let config = SearchConfig { case_sensitive: false, ..Default::default() };
        let hosts = vec![HostEntry::new("SERVER", "ssh server")];
        let engine = SearchEngine::with_config(hosts, config);
        
        let results = engine.search("server");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn respects_max_results_configuration() {
        let hosts = (0..100).map(|i| 
            HostEntry::new(&format!("server{}", i), "ssh test")
        ).collect();
        let config = SearchConfig { max_results: 5, ..Default::default() };
        let engine = SearchEngine::with_config(hosts, config);
        
        let results = engine.search("server");
        assert_eq!(results.len(), 5);
    }
}
```

### 2. Application State Management (Unit Tests - TDD)

```rust
// tests/unit/app_state_test.rs
#[cfg(test)]
mod app_state_tests {
    use super::*;

    #[test]
    fn initial_state_has_empty_search() {
        let app_state = AppState::new(vec![], Config::default());
        assert_eq!(app_state.search_query(), "");
        assert_eq!(app_state.selected_index(), 0);
    }

    #[test]
    fn typing_updates_search_query_and_filters_results() {
        let hosts = vec![
            HostEntry::new("web-server", "ssh web"),
            HostEntry::new("db-server", "ssh db"),
        ];
        let mut app_state = AppState::new(hosts, Config::default());
        
        // Simulate typing
        app_state = app_state.handle_input(InputEvent::TypeChar('w'));
        app_state = app_state.handle_input(InputEvent::TypeChar('e'));
        app_state = app_state.handle_input(InputEvent::TypeChar('b'));
        
        assert_eq!(app_state.search_query(), "web");
        assert_eq!(app_state.filtered_results().len(), 1);
        assert_eq!(app_state.filtered_results()[0].name, "web-server");
    }

    #[test]
    fn arrow_keys_navigate_selection() {
        let hosts = vec![
            HostEntry::new("server1", "ssh s1"),
            HostEntry::new("server2", "ssh s2"),
            HostEntry::new("server3", "ssh s3"),
        ];
        let mut app_state = AppState::new(hosts, Config::default());
        
        // All results visible initially
        assert_eq!(app_state.selected_index(), 0);
        
        app_state = app_state.handle_input(InputEvent::ArrowDown);
        assert_eq!(app_state.selected_index(), 1);
        
        app_state = app_state.handle_input(InputEvent::ArrowDown);
        assert_eq!(app_state.selected_index(), 2);
        
        // Wrap around
        app_state = app_state.handle_input(InputEvent::ArrowDown);
        assert_eq!(app_state.selected_index(), 0);
    }

    #[test]
    fn enter_key_triggers_connection_attempt() {
        let hosts = vec![HostEntry::new("test-server", "ssh test")];
        let mut app_state = AppState::new(hosts, Config::default());
        
        let result = app_state.handle_input(InputEvent::Enter);
        match result {
            StateResult::ConnectToHost(host) => {
                assert_eq!(host.connection_string, "ssh test");
            }
            _ => panic!("Expected ConnectToHost result"),
        }
    }

    #[test]
    fn backspace_removes_characters_and_updates_results() {
        let hosts = vec![HostEntry::new("web-server", "ssh web")];
        let mut app_state = AppState::new(hosts, Config::default());
        
        // Type "web"
        app_state = app_state.handle_input(InputEvent::TypeChar('w'));
        app_state = app_state.handle_input(InputEvent::TypeChar('e'));
        app_state = app_state.handle_input(InputEvent::TypeChar('b'));
        assert_eq!(app_state.filtered_results().len(), 1);
        
        // Backspace to "we"
        app_state = app_state.handle_input(InputEvent::Backspace);
        assert_eq!(app_state.search_query(), "we");
        // Results should still match
        assert_eq!(app_state.filtered_results().len(), 1);
    }
}
```

### 3. SSH File Parsing (Unit Tests - TDD)

```rust
// tests/unit/ssh_parser_test.rs
#[cfg(test)]
mod ssh_parser_tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn parses_simple_known_hosts_format() {
        let content = "192.168.1.1 ssh-rsa AAAAB3NzaC1yc2E...\nserver.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5...";
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();
        
        let hosts = parse_known_hosts(temp_file.path().to_str().unwrap()).unwrap();
        
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].name, "192.168.1.1");
        assert_eq!(hosts[1].name, "server.com");
    }

    #[test]
    fn skips_hashed_hosts_when_configured() {
        let content = "|1|JfKTdHh|rNthvGl= ssh-rsa AAAAB3...\nserver.com ssh-rsa AAAAB3...";
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();
        
        let config = ParsingConfig { skip_hashed_hosts: true };
        let hosts = parse_known_hosts_with_config(temp_file.path().to_str().unwrap(), &config).unwrap();
        
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "server.com");
    }

    #[test]
    fn parses_simple_ssh_config_hosts() {
        let content = r#"
Host web-server
    HostName web.example.com
    User admin

Host db
    HostName db.internal
    Port 2222
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();
        
        let hosts = parse_ssh_config(temp_file.path().to_str().unwrap()).unwrap();
        
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].name, "web-server");
        assert_eq!(hosts[0].connection_string, "ssh admin@web.example.com");
        assert_eq!(hosts[1].name, "db");
        assert_eq!(hosts[1].connection_string, "ssh db.internal -p 2222");
    }

    #[test]
    fn handles_missing_files_gracefully() {
        let result = parse_known_hosts("/nonexistent/file");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
```

### 4. Configuration Loading (Unit Tests - TDD)

```rust
// tests/unit/config_test.rs
#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn loads_default_config_when_file_missing() {
        let config = Config::load("/nonexistent/path").unwrap();
        assert_eq!(config.terminal.program, "/Applications/Terminal.app/Contents/MacOS/Terminal");
        assert_eq!(config.ssh.ssh_binary, "/usr/bin/ssh");
    }

    #[test]
    fn validates_terminal_program_exists() {
        let config_toml = r#"
[terminal]
program = "/nonexistent/terminal"
args = ["-e"]
"#;
        let config: Config = toml::from_str(config_toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn expands_tilde_in_ssh_paths() {
        let config_toml = r#"
[ssh]
known_hosts_path = "~/.ssh/known_hosts"
config_path = "~/.ssh/config"
"#;
        let config: Config = toml::from_str(config_toml).unwrap();
        let expanded = config.expand_paths().unwrap();
        
        assert!(expanded.ssh.known_hosts_path.starts_with('/'));
        assert!(!expanded.ssh.known_hosts_path.contains('~'));
    }
}
```

### 5. Terminal Integration (Mock Tests)

```rust
// tests/unit/terminal_launcher_test.rs
#[cfg(test)]
mod terminal_launcher_tests {
    use super::*;
    use mockall::mock;

    mock! {
        ProcessSpawner {}
        
        impl ProcessSpawner for ProcessSpawner {
            fn spawn_command(&self, program: &str, args: &[String]) -> Result<(), LaunchError>;
        }
    }

    #[test]
    fn constructs_correct_terminal_command() {
        let mut mock_spawner = MockProcessSpawner::new();
        mock_spawner
            .expect_spawn_command()
            .with(
                eq("/Applications/iTerm.app/Contents/MacOS/iTerm2"),
                eq(vec!["-c".to_string(), "tell application \"iTerm2\" to create window with default profile command \"ssh user@server.com\"".to_string()])
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let config = TerminalConfig {
            program: "/Applications/iTerm.app/Contents/MacOS/iTerm2".to_string(),
            args: vec!["-c".to_string(), "tell application \"iTerm2\" to create window with default profile command \"{ssh_command}\"".to_string()],
        };
        let host = HostEntry::new("server", "ssh user@server.com");
        
        let launcher = TerminalLauncher::new(config, Box::new(mock_spawner));
        launcher.launch_connection(&host).unwrap();
    }

    #[test]
    fn escapes_special_characters_in_ssh_commands() {
        let mut mock_spawner = MockProcessSpawner::new();
        // Expect properly escaped command
        mock_spawner
            .expect_spawn_command()
            .withf(|_, args| {
                args[1].contains("ssh user@server.com\\;echo\\ hacked") // Escaped semicolon
            })
            .returning(|_, _| Ok(()));

        let config = TerminalConfig {
            program: "/usr/bin/osascript".to_string(),
            args: vec!["-e".to_string(), "tell app \"Terminal\" to do script \"{ssh_command}\"".to_string()],
        };
        let host = HostEntry::new("malicious", "ssh user@server.com;echo hacked");
        
        let launcher = TerminalLauncher::new(config, Box::new(mock_spawner));
        launcher.launch_connection(&host).unwrap();
    }
}
```

### 6. Performance Benchmarks

```rust
// benches/search_performance.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn search_benchmark(c: &mut Criterion) {
    let sizes = vec![100, 1000, 10000];
    
    for size in sizes {
        let hosts = generate_test_hosts(size);
        let engine = SearchEngine::new(hosts);
        
        c.bench_with_input(
            BenchmarkId::new("search_performance", size),
            &engine,
            |b, engine| {
                b.iter(|| engine.search("server"))
            }
        );
    }
}

fn startup_benchmark(c: &mut Criterion) {
    c.bench_function("app_startup", |b| {
        b.iter(|| {
            let config = Config::default();
            let hosts = load_test_hosts();
            AppState::new(hosts, config)
        })
    });
}

criterion_group!(benches, search_benchmark, startup_benchmark);
criterion_main!(benches);
```

### 7. Integration Tests (GPUI Event Simulation)

```rust
// tests/integration/gui_integration_test.rs
#[cfg(test)]
mod gui_integration_tests {
    use super::*;
    use gpui::TestAppContext;

    #[test]
    fn full_search_and_select_workflow() {
        let mut cx = TestAppContext::new();
        
        let hosts = vec![
            HostEntry::new("web-server", "ssh web"),
            HostEntry::new("db-server", "ssh db"),
        ];
        
        let app = cx.add_model(|_| AppModel::new(hosts, Config::default()));
        let window = cx.add_window(|cx| AppView::new(app.clone(), cx));
        
        // Simulate typing "web"
        window.simulate_key_event(KeyEvent::new('w'));
        window.simulate_key_event(KeyEvent::new('e'));
        window.simulate_key_event(KeyEvent::new('b'));
        
        // Verify filtered results
        let app_state = app.read(&cx);
        assert_eq!(app_state.filtered_results().len(), 1);
        assert_eq!(app_state.filtered_results()[0].name, "web-server");
        
        // Simulate Enter key
        let result = window.simulate_key_event(KeyEvent::enter());
        // Verify connection attempt (would be mocked in real test)
    }

    #[test]
    fn keyboard_navigation_works_correctly() {
        let mut cx = TestAppContext::new();
        
        let hosts = vec![
            HostEntry::new("server1", "ssh s1"),
            HostEntry::new("server2", "ssh s2"),
        ];
        
        let app = cx.add_model(|_| AppModel::new(hosts, Config::default()));
        let window = cx.add_window(|cx| AppView::new(app.clone(), cx));
        
        // Initial selection should be first item
        assert_eq!(app.read(&cx).selected_index(), 0);
        
        // Arrow down
        window.simulate_key_event(KeyEvent::arrow_down());
        assert_eq!(app.read(&cx).selected_index(), 1);
        
        // Arrow up
        window.simulate_key_event(KeyEvent::arrow_up());
        assert_eq!(app.read(&cx).selected_index(), 0);
    }
}
```

## TDD Workflow Example

### Red-Green-Refactor Cycle

```rust
// 1. RED - Write failing test first
#[test]
fn search_returns_results_within_50ms() {
    let hosts = generate_large_host_list(5000);
    let engine = SearchEngine::new(hosts);
    
    let start = Instant::now();
    let results = engine.search("server");
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 50); // This will fail initially
}

// 2. GREEN - Implement minimal code to pass
impl SearchEngine {
    pub fn search(&self, query: &str) -> Vec<HostEntry> {
        // Simple linear search - will be too slow
        self.hosts.iter()
            .filter(|host| host.name.contains(query))
            .cloned()
            .collect()
    }
}

// 3. REFACTOR - Optimize while keeping tests green
impl SearchEngine {
    pub fn search(&self, query: &str) -> Vec<HostEntry> {
        // Add indexing, better algorithms, etc.
        self.index.fuzzy_search(query)
            .into_iter()
            .take(self.config.max_results)
            .collect()
    }
}
```

## Test Organization Strategy

### Test Pyramid Structure
1. **Unit Tests (70%)**: Fast, isolated, TDD-driven
2. **Integration Tests (20%)**: GPUI components working together  
3. **End-to-End Tests (10%)**: Full application scenarios

### Performance Testing
- **Continuous benchmarking** in CI
- **Performance regression detection**
- **Memory usage monitoring**

### Mock Strategy
- Mock terminal launching for safety
- Mock file system for consistent tests
- Dependency injection for testability

This testing strategy ensures your success metrics are baked into the development process from day one, giving you confidence that performance and functionality requirements are met throughout development.
