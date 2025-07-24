// ABOUTME: Configuration structures and parsing for user-defined terminal and SSH settings
// ABOUTME: Implements the configuration-driven approach where users specify their exact setup

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Config {
    pub terminal: TerminalConfig,
    pub ssh: SshConfig,
    pub parsing: ParsingConfig,
    pub ui: UiConfig,
    #[serde(default)]
    pub hotkey: HotkeyConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TerminalConfig {
    pub program: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub strategy: LaunchStrategy,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub enum LaunchStrategy {
    #[default]
    ShellCommand,
    Direct,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SshConfig {
    pub known_hosts_path: String,
    pub config_path: String,
    pub ssh_binary: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ParsingConfig {
    pub parse_known_hosts: bool,
    pub parse_ssh_config: bool,
    pub simple_config_parsing: bool,
    #[serde(default = "default_skip_hashed_hosts")]
    pub skip_hashed_hosts: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct UiConfig {
    pub max_results: usize,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct HotkeyConfig {
    #[serde(default = "default_hotkey_combination")]
    pub combination: String,
}

fn default_hotkey_combination() -> String {
    "Super+Shift+S".to_string()
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            combination: default_hotkey_combination(),
        }
    }
}

fn default_skip_hashed_hosts() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DetectedTerminal {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
    pub strategy: LaunchStrategy,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            terminal: TerminalConfig {
                program: "/Applications/iTerm.app/Contents/MacOS/iTerm2".to_string(),
                args: vec![
                    "-c".to_string(),
                    "tell application \"iTerm2\" to create window with default profile command \"{ssh_command}\"".to_string(),
                ],
                strategy: LaunchStrategy::ShellCommand,
            },
            ssh: SshConfig {
                known_hosts_path: "~/.ssh/known_hosts".to_string(),
                config_path: "~/.ssh/config".to_string(),
                ssh_binary: "/usr/bin/ssh".to_string(),
            },
            parsing: ParsingConfig {
                parse_known_hosts: true,
                parse_ssh_config: true,
                simple_config_parsing: true,
                skip_hashed_hosts: true,
            },
            ui: UiConfig {
                max_results: 20,
                case_sensitive: false,
            },
            hotkey: HotkeyConfig::default(),
        }
    }
}

impl Config {
    pub fn generate_default_config() -> String {
        let terminal_config = Self::detect_best_terminal();
        let home = dirs::home_dir()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|| "~".to_string());

        let strategy_str = match terminal_config.strategy {
            LaunchStrategy::Direct => "direct",
            LaunchStrategy::ShellCommand => "shellCommand",
        };

        let mut content = String::new();
        content.push_str("# Trident SSH Launcher Configuration\n");
        content.push_str(&format!("# Generated automatically with detected terminal: {}\n", terminal_config.name));
        content.push_str("\n[terminal]\n");
        content.push_str(&format!("program = \"{}\"\n", terminal_config.program));
        content.push_str(&format!("args = {}\n", Self::format_args_for_toml(&terminal_config.args)));
        content.push_str(&format!("strategy = \"{strategy_str}\"\n"));
        content.push_str(&Self::generate_terminal_examples(&terminal_config.name));
        content.push_str("\n[ssh]\n");
        content.push_str(&format!("known_hosts_path = \"{home}/.ssh/known_hosts\"\n"));
        content.push_str(&format!("config_path = \"{home}/.ssh/config\"\n"));
        content.push_str("ssh_binary = \"/usr/bin/ssh\"\n");
        content.push_str("\n[parsing]\n");
        content.push_str("parse_known_hosts = true\n");
        content.push_str("parse_ssh_config = true\n");
        content.push_str("simple_config_parsing = true\n");
        content.push_str("skip_hashed_hosts = true\n");
        content.push_str("\n[ui]\n");
        content.push_str("max_results = 20\n");
        content.push_str("case_sensitive = false\n");
        content.push_str("\n[hotkey]\n");
        content.push_str("combination = \"Super+Shift+S\"\n");
        content
    }

    

    pub fn load_from_str(content: &str) -> Result<Self> {
        toml::from_str(content).context("Failed to parse configuration")
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration file: {}", path.display()))?;
        Self::load_from_str(&content)
    }

    pub fn default_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Failed to determine config directory")?;
        Ok(config_dir.join("trident").join("config.toml"))
    }

    pub fn expand_path(&mut self) -> Result<()> {
        self.ssh.known_hosts_path = expand_tilde(&self.ssh.known_hosts_path)?;
        self.ssh.config_path = expand_tilde(&self.ssh.config_path)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_with_file_checks(true)
    }

    pub fn validate_with_file_checks(&self, check_files: bool) -> Result<()> {
        if self.terminal.program.is_empty() {
            anyhow::bail!("Terminal program cannot be empty.");
        }

        // Check for {ssh_command} placeholder in args when using ShellCommand strategy
        if self.terminal.strategy == LaunchStrategy::ShellCommand {
            let has_placeholder = self
                .terminal
                .args
                .iter()
                .any(|arg| arg.contains("{ssh_command}"));
            if !has_placeholder && !self.terminal.args.is_empty() {
                anyhow::bail!(
                    "Terminal args must contain {{ssh_command}} placeholder when using shellCommand strategy.\n\
                    Current args: {:?}\n\
                    Example: [\"-e\", \"{{ssh_command}}\"]",
                    self.terminal.args
                );
            }
        }

        if check_files && which::which(&self.terminal.program).is_err() {
            anyhow::bail!(
                "Terminal program '{}' not found in PATH or as an absolute path.",
                self.terminal.program
            );
        }

        if self.ssh.ssh_binary.is_empty() {
            anyhow::bail!("SSH binary path cannot be empty");
        }

        if check_files && which::which(&self.ssh.ssh_binary).is_err() {
            anyhow::bail!(
                "SSH binary '{}' not found in PATH or as an absolute path.",
                self.ssh.ssh_binary
            );
        }

        if self.ui.max_results == 0 {
            anyhow::bail!("max_results must be greater than 0.");
        }

        if self.ui.max_results > 100 {
            tracing::warn!(
                "max_results ({}) is very high, this may impact performance",
                self.ui.max_results
            );
        }

        if !self.parsing.parse_known_hosts && !self.parsing.parse_ssh_config {
            anyhow::bail!("At least one parsing source must be enabled.");
        }

        if check_files {
            if self.parsing.parse_known_hosts && !Path::new(&self.ssh.known_hosts_path).exists() {
                tracing::warn!(
                    "known_hosts file '{}' does not exist.",
                    self.ssh.known_hosts_path
                );
            }

            if self.parsing.parse_ssh_config && !Path::new(&self.ssh.config_path).exists() {
                tracing::warn!(
                    "SSH config file '{}' does not exist.",
                    self.ssh.config_path
                );
            }
        }

        Ok(())
    }

    pub fn save_generated_config(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        fs::write(path, Self::generate_default_config())
            .with_context(|| format!("Failed to write generated config to: {}", path.display()))?;

        Ok(())
    }

    pub fn detect_best_terminal() -> DetectedTerminal {
        crate::platform::Platform::config_detector().detect_terminals().into_iter().next().unwrap_or_else(|| {
            DetectedTerminal {
                name: "Terminal.app".to_string(),
                program: "/usr/bin/osascript".to_string(),
                args: vec![
                    "-e".to_string(),
                    "tell app \"Terminal\" to do script \"{ssh_command}\"".to_string(),
                ],
                strategy: LaunchStrategy::ShellCommand,
            }
        })
    }

    fn format_args_for_toml(args: &[String]) -> String {
        let quoted_args: Vec<String> = args
            .iter()
            .map(|arg| format!("\"{}\"", arg.replace('"', "\\\"")))
            .collect();
        format!("[{}]", quoted_args.join(", "))
    }

    fn generate_terminal_examples(_current_terminal: &str) -> String {
        // This is now a simplified placeholder. The main generation logic is in `generate_default_config`.
        "".to_string()
    }
}

fn expand_tilde(path: &str) -> Result<String> {
    if let Some(stripped) = path.strip_prefix("~/") {
        let home = dirs::home_dir().context("Failed to determine home directory")?;
        Ok(home.join(stripped).to_string_lossy().into_owned())
    } else {
        Ok(path.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let config_str = r#"
[terminal]
program = "my-terminal"
args = []

[ssh]
known_hosts_path = "~/.ssh/known_hosts"
config_path = "~/.ssh/config"
ssh_binary = "/usr/bin/ssh"

[parsing]
parse_known_hosts = true
parse_ssh_config = true
simple_config_parsing = true

[ui]
max_results = 20
case_sensitive = false
"#;

        let config = Config::load_from_str(config_str).unwrap();
        assert_eq!(config.terminal.program, "my-terminal");
        assert_eq!(config.terminal.strategy, LaunchStrategy::ShellCommand);
        assert_eq!(config.hotkey.combination, "Super+Shift+S");
    }
}