// ABOUTME: Core application state and Model-View-Update logic for the SSH launcher
// ABOUTME: Coordinates configuration, SSH parsing, and fuzzy search in a testable way

use crate::config::Config;
use crate::fuzzy::SearchEngine;
use crate::ssh::parser::{HostEntry, parse_known_hosts, parse_ssh_config};
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Config,
    pub hosts: Vec<HostEntry>,
    pub search_query: String,
    pub filtered_hosts: Vec<HostEntry>,
    pub selected_index: usize,
    pub is_loading: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    LoadConfig(Config),
    UpdateSearchQuery(String),
    SelectNext,
    SelectPrevious,
    SelectHost(usize),
    LaunchSelectedHost,
    RefreshHosts,
    ShowError(String),
    ClearError,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            hosts: Vec::new(),
            search_query: String::new(),
            filtered_hosts: Vec::new(),
            selected_index: 0,
            is_loading: false,
            error_message: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Result<()> {
        match message {
            Message::LoadConfig(config) => {
                self.config = config;
                self.update(Message::RefreshHosts)?;
            }

            Message::UpdateSearchQuery(query) => {
                self.search_query = query;
                self.update_filtered_hosts();
                self.selected_index = 0;
            }

            Message::SelectNext => {
                if !self.filtered_hosts.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.filtered_hosts.len();
                }
            }

            Message::SelectPrevious => {
                if !self.filtered_hosts.is_empty() {
                    self.selected_index = if self.selected_index == 0 {
                        self.filtered_hosts.len() - 1
                    } else {
                        self.selected_index - 1
                    };
                }
            }

            Message::SelectHost(index) => {
                if index < self.filtered_hosts.len() {
                    self.selected_index = index;
                }
            }

            Message::LaunchSelectedHost => {
                if let Some(host) = self.get_selected_host() {
                    self.launch_host(host)?;
                }
            }

            Message::RefreshHosts => {
                self.is_loading = true;
                match self.load_hosts() {
                    Ok(hosts) => {
                        self.hosts = hosts;
                        self.update_filtered_hosts();
                        self.is_loading = false;
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.is_loading = false;
                        self.error_message = Some(e.to_string());
                    }
                }
            }

            Message::ShowError(message) => {
                self.error_message = Some(message);
            }

            Message::ClearError => {
                self.error_message = None;
            }
        }

        Ok(())
    }

    fn load_hosts(&mut self) -> Result<Vec<HostEntry>> {
        let mut all_hosts = Vec::new();

        // Parse known_hosts if enabled
        if self.config.parsing.parse_known_hosts {
            let known_hosts_path = Path::new(&self.config.ssh.known_hosts_path);
            if known_hosts_path.exists() {
                let hosts =
                    parse_known_hosts(known_hosts_path, self.config.parsing.skip_hashed_hosts)?;
                all_hosts.extend(hosts);
            }
        }

        // Parse SSH config if enabled
        if self.config.parsing.parse_ssh_config {
            let config_path = Path::new(&self.config.ssh.config_path);
            if config_path.exists() {
                let hosts =
                    parse_ssh_config(config_path, self.config.parsing.simple_config_parsing)?;
                all_hosts.extend(hosts);
            }
        }

        // Remove duplicates
        all_hosts.sort_by(|a, b| a.name.cmp(&b.name));
        all_hosts.dedup_by(|a, b| a.name == b.name);

        Ok(all_hosts)
    }

    fn update_filtered_hosts(&mut self) {
        let search_engine = SearchEngine::new(self.hosts.clone());
        let results = search_engine.search(
            &self.search_query,
            self.config.ui.case_sensitive,
            self.config.ui.max_results,
        );

        self.filtered_hosts = results.into_iter().cloned().collect();

        // Ensure selected index is valid
        if self.selected_index >= self.filtered_hosts.len() {
            self.selected_index = if self.filtered_hosts.is_empty() {
                0
            } else {
                self.filtered_hosts.len() - 1
            };
        }
    }

    pub fn get_selected_host(&self) -> Option<&HostEntry> {
        self.filtered_hosts.get(self.selected_index)
    }

    fn launch_host(&self, host: &HostEntry) -> Result<()> {
        use std::process::Command;

        // Build the SSH command
        let ssh_command = format!("{} {}", self.config.ssh.ssh_binary, host.name);

        // Replace placeholder in terminal args
        let mut terminal_args = self.config.terminal.args.clone();
        for arg in &mut terminal_args {
            *arg = arg.replace("{ssh_command}", &ssh_command);
        }

        // Launch the terminal
        Command::new(&self.config.terminal.program)
            .args(&terminal_args)
            .spawn()?;

        Ok(())
    }

    pub fn has_hosts(&self) -> bool {
        !self.filtered_hosts.is_empty()
    }

    pub fn get_display_hosts(&self) -> &[HostEntry] {
        &self.filtered_hosts
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ParsingConfig, SshConfig, TerminalConfig, UiConfig};
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &TempDir) -> Config {
        let known_hosts_path = temp_dir.path().join("known_hosts");
        let ssh_config_path = temp_dir.path().join("config");

        // Create test known_hosts file
        let mut known_hosts_file = fs::File::create(&known_hosts_path).unwrap();
        writeln!(
            known_hosts_file,
            "example.com ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEA..."
        )
        .unwrap();
        writeln!(
            known_hosts_file,
            "server.local ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI..."
        )
        .unwrap();

        // Create test SSH config file
        let mut ssh_config_file = fs::File::create(&ssh_config_path).unwrap();
        writeln!(ssh_config_file, "Host production").unwrap();
        writeln!(ssh_config_file, "    HostName prod.example.com").unwrap();
        writeln!(ssh_config_file, "Host staging").unwrap();
        writeln!(ssh_config_file, "    HostName staging.example.com").unwrap();

        Config {
            terminal: TerminalConfig {
                program: "/bin/echo".to_string(),
                args: vec!["Launching:".to_string(), "{ssh_command}".to_string()],
            },
            ssh: SshConfig {
                known_hosts_path: known_hosts_path.to_string_lossy().to_string(),
                config_path: ssh_config_path.to_string_lossy().to_string(),
                ssh_binary: "/usr/bin/ssh".to_string(),
            },
            parsing: ParsingConfig {
                parse_known_hosts: true,
                parse_ssh_config: true,
                simple_config_parsing: true,
                skip_hashed_hosts: true,
            },
            ui: UiConfig {
                max_results: 10,
                case_sensitive: false,
            },
        }
    }

    #[test]
    fn test_new_app_state() {
        let app = AppState::new();

        assert!(app.hosts.is_empty());
        assert!(app.search_query.is_empty());
        assert!(app.filtered_hosts.is_empty());
        assert_eq!(app.selected_index, 0);
        assert!(!app.is_loading);
        assert!(app.error_message.is_none());
    }

    #[test]
    fn test_update_search_query() {
        let mut app = AppState::new();
        app.hosts = vec![
            HostEntry::new("production".to_string(), "ssh production".to_string()),
            HostEntry::new("staging".to_string(), "ssh staging".to_string()),
        ];

        app.update(Message::UpdateSearchQuery("prod".to_string()))
            .unwrap();

        assert_eq!(app.search_query, "prod");
        assert_eq!(app.filtered_hosts.len(), 1);
        assert_eq!(app.filtered_hosts[0].name, "production");
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_select_navigation() {
        let mut app = AppState::new();
        app.filtered_hosts = vec![
            HostEntry::new("host1".to_string(), "ssh host1".to_string()),
            HostEntry::new("host2".to_string(), "ssh host2".to_string()),
            HostEntry::new("host3".to_string(), "ssh host3".to_string()),
        ];

        assert_eq!(app.selected_index, 0);

        app.update(Message::SelectNext).unwrap();
        assert_eq!(app.selected_index, 1);

        app.update(Message::SelectNext).unwrap();
        assert_eq!(app.selected_index, 2);

        // Should wrap around
        app.update(Message::SelectNext).unwrap();
        assert_eq!(app.selected_index, 0);

        // Test previous
        app.update(Message::SelectPrevious).unwrap();
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn test_get_selected_host() {
        let mut app = AppState::new();
        app.filtered_hosts = vec![
            HostEntry::new("host1".to_string(), "ssh host1".to_string()),
            HostEntry::new("host2".to_string(), "ssh host2".to_string()),
        ];

        assert_eq!(app.get_selected_host().unwrap().name, "host1");

        app.selected_index = 1;
        assert_eq!(app.get_selected_host().unwrap().name, "host2");

        app.selected_index = 99;
        assert!(app.get_selected_host().is_none());
    }

    #[test]
    fn test_load_config_and_hosts() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);

        let mut app = AppState::new();
        app.update(Message::LoadConfig(config)).unwrap();

        // Should have loaded hosts from both files
        assert!(app.hosts.len() >= 2);
        assert!(app.hosts.iter().any(|h| h.name == "example.com"));
        assert!(app.hosts.iter().any(|h| h.name == "production"));
    }

    #[test]
    fn test_has_hosts() {
        let mut app = AppState::new();
        assert!(!app.has_hosts());

        app.filtered_hosts = vec![HostEntry::new("host1".to_string(), "ssh host1".to_string())];
        assert!(app.has_hosts());
    }
}
