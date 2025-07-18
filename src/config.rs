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
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TerminalConfig {
    pub program: String,
    pub args: Vec<String>,
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

fn default_skip_hashed_hosts() -> bool {
    true
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
        }
    }
}

impl Config {
    pub fn default_config_content() -> &'static str {
        r#"# Trident SSH Launcher Configuration

[terminal]
# Specify your terminal program and how to launch SSH commands
# Examples for different terminals:

# iTerm2 (default):
program = "/Applications/iTerm.app/Contents/MacOS/iTerm2"
args = ["-c", "tell application \"iTerm2\" to create window with default profile command \"{ssh_command}\""]

# Terminal.app:
# program = "/usr/bin/osascript"
# args = ["-e", "tell app \"Terminal\" to do script \"{ssh_command}\""]

# Alacritty:
# program = "/Applications/Alacritty.app/Contents/MacOS/alacritty"
# args = ["-e", "{ssh_command}"]

# Kitty:
# program = "/Applications/kitty.app/Contents/MacOS/kitty"
# args = ["--", "{ssh_command}"]

[ssh]
# SSH file locations
known_hosts_path = "~/.ssh/known_hosts"
config_path = "~/.ssh/config"
ssh_binary = "/usr/bin/ssh"

[parsing]
# What to parse and how
parse_known_hosts = true
parse_ssh_config = true
# Simple parsing only looks at Host entries, ignores Include directives
simple_config_parsing = true
# Skip hashed entries in known_hosts (recommended)
skip_hashed_hosts = true

[ui]
# User interface settings
max_results = 20
case_sensitive = false
"#
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
        let config_dir = dirs::config_dir()
            .context("Failed to determine config directory")?;
        Ok(config_dir.join("trident").join("config.toml"))
    }
    
    pub fn expand_path(&mut self) -> Result<()> {
        self.ssh.known_hosts_path = expand_tilde(&self.ssh.known_hosts_path)?;
        self.ssh.config_path = expand_tilde(&self.ssh.config_path)?;
        Ok(())
    }
    
    pub fn validate(&self) -> Result<()> {
        // Validate terminal configuration
        if self.terminal.program.is_empty() {
            anyhow::bail!("Terminal program cannot be empty");
        }
        
        // Check for {ssh_command} placeholder in args
        let has_placeholder = self.terminal.args.iter()
            .any(|arg| arg.contains("{ssh_command}"));
        if !has_placeholder && !self.terminal.args.is_empty() {
            anyhow::bail!("Terminal args must contain {{ssh_command}} placeholder");
        }
        
        // Validate UI configuration
        if self.ui.max_results == 0 {
            anyhow::bail!("max_results must be greater than 0");
        }
        
        // Validate parsing configuration
        if !self.parsing.parse_known_hosts && !self.parsing.parse_ssh_config {
            anyhow::bail!("At least one parsing source must be enabled");
        }
        
        Ok(())
    }
    
    pub fn save_default_config(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }
        
        fs::write(path, Self::default_config_content())
            .with_context(|| format!("Failed to write default config to: {}", path.display()))?;
        
        Ok(())
    }
}

fn expand_tilde(path: &str) -> Result<String> {
    if path.starts_with("~/") {
        let home = dirs::home_dir()
            .context("Failed to determine home directory")?;
        Ok(home.join(&path[2..]).to_string_lossy().into_owned())
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
program = "/Applications/iTerm.app/Contents/MacOS/iTerm2"
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
        
        assert_eq!(config.terminal.program, "/Applications/iTerm.app/Contents/MacOS/iTerm2");
        assert_eq!(config.terminal.args.len(), 0);
        assert_eq!(config.ssh.known_hosts_path, "~/.ssh/known_hosts");
        assert_eq!(config.parsing.parse_known_hosts, true);
        assert_eq!(config.parsing.skip_hashed_hosts, true); // Default value
        assert_eq!(config.ui.max_results, 20);
        assert_eq!(config.ui.case_sensitive, false);
    }

    #[test]
    fn test_parse_config_with_terminal_args() {
        let config_str = r#"
[terminal]
program = "/usr/bin/osascript"
args = ["-e", "tell app \"Terminal\" to do script \"{ssh_command}\""]

[ssh]
known_hosts_path = "~/.ssh/known_hosts"
config_path = "~/.ssh/config"
ssh_binary = "/usr/bin/ssh"

[parsing]
parse_known_hosts = true
parse_ssh_config = false
simple_config_parsing = true
skip_hashed_hosts = false

[ui]
max_results = 50
case_sensitive = true
"#;

        let config = Config::load_from_str(config_str).unwrap();
        
        assert_eq!(config.terminal.program, "/usr/bin/osascript");
        assert_eq!(config.terminal.args, vec!["-e", "tell app \"Terminal\" to do script \"{ssh_command}\""]);
        assert_eq!(config.parsing.parse_ssh_config, false);
        assert_eq!(config.parsing.skip_hashed_hosts, false);
        assert_eq!(config.ui.max_results, 50);
        assert_eq!(config.ui.case_sensitive, true);
    }

    #[test]
    fn test_parse_invalid_config_missing_section() {
        let config_str = r#"
[terminal]
program = "/Applications/iTerm.app/Contents/MacOS/iTerm2"
args = []

[ssh]
known_hosts_path = "~/.ssh/known_hosts"
config_path = "~/.ssh/config"
ssh_binary = "/usr/bin/ssh"
"#;

        let result = Config::load_from_str(config_str);
        assert!(result.is_err());
        // Our context message should be present
        assert!(result.unwrap_err().to_string().contains("Failed to parse configuration"));
    }

    #[test]
    fn test_parse_invalid_config_wrong_type() {
        let config_str = r#"
[terminal]
program = "/Applications/iTerm.app/Contents/MacOS/iTerm2"
args = []

[ssh]
known_hosts_path = "~/.ssh/known_hosts"
config_path = "~/.ssh/config"
ssh_binary = "/usr/bin/ssh"

[parsing]
parse_known_hosts = "yes"  # Should be boolean
parse_ssh_config = true
simple_config_parsing = true

[ui]
max_results = 20
case_sensitive = false
"#;

        let result = Config::load_from_str(config_str);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().unwrap();
        let home_str = home.to_string_lossy();
        
        assert_eq!(expand_tilde("~/test").unwrap(), format!("{}/test", home_str));
        assert_eq!(expand_tilde("/absolute/path").unwrap(), "/absolute/path");
        assert_eq!(expand_tilde("relative/path").unwrap(), "relative/path");
    }
    
    #[test]
    fn test_config_expand_paths() {
        let config_str = r#"
[terminal]
program = "/Applications/iTerm.app/Contents/MacOS/iTerm2"
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

        let mut config = Config::load_from_str(config_str).unwrap();
        config.expand_path().unwrap();
        
        let home = dirs::home_dir().unwrap();
        assert_eq!(config.ssh.known_hosts_path, home.join(".ssh/known_hosts").to_string_lossy());
        assert_eq!(config.ssh.config_path, home.join(".ssh/config").to_string_lossy());
        assert_eq!(config.ssh.ssh_binary, "/usr/bin/ssh");
    }
    
    #[test]
    fn test_default_config_path() {
        let path = Config::default_config_path().unwrap();
        assert!(path.to_string_lossy().contains("trident"));
        assert!(path.to_string_lossy().contains("config.toml"));
    }
    
    #[test]
    fn test_validate_empty_terminal_program() {
        let mut config = create_test_config();
        config.terminal.program = "".to_string();
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Terminal program cannot be empty"));
    }
    
    #[test]
    fn test_validate_missing_ssh_command_placeholder() {
        let mut config = create_test_config();
        config.terminal.args = vec!["-e".to_string(), "some command".to_string()];
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("{ssh_command} placeholder"));
    }
    
    #[test]
    fn test_validate_zero_max_results() {
        let mut config = create_test_config();
        config.ui.max_results = 0;
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_results must be greater than 0"));
    }
    
    #[test]
    fn test_validate_no_parsing_sources() {
        let mut config = create_test_config();
        config.parsing.parse_known_hosts = false;
        config.parsing.parse_ssh_config = false;
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("At least one parsing source"));
    }
    
    #[test]
    fn test_validate_valid_config() {
        let config = create_test_config();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_default_config_is_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_default_config_content_can_be_parsed() {
        let content = Config::default_config_content();
        let config = Config::load_from_str(content).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config, Config::default());
    }
    
    fn create_test_config() -> Config {
        Config {
            terminal: TerminalConfig {
                program: "/usr/bin/terminal".to_string(),
                args: vec!["-e".to_string(), "{ssh_command}".to_string()],
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
        }
    }
}