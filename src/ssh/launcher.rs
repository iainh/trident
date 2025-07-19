// ABOUTME: Terminal launcher for SSH connections using user-configured terminal programs
// ABOUTME: Provides safe command substitution and process spawning for various terminal applications

use crate::Logger;
use crate::config::TerminalConfig;
use crate::ssh::parser::HostEntry;
use anyhow::{Context, Result};
use std::process::Command;

pub struct TerminalLauncher {
    config: TerminalConfig,
}

impl TerminalLauncher {
    pub fn new(config: TerminalConfig) -> Self {
        Self { config }
    }

    pub fn launch(&self, host: &HostEntry) -> Result<()> {
        Logger::debug(&format!("Launching SSH connection to host: {}", host.name));

        // Escape the SSH command for safe shell execution
        let escaped_command = escape_shell_command(&host.connection_string);
        Logger::debug(&format!("Escaped SSH command: {}", escaped_command));

        // Substitute {ssh_command} placeholder in terminal arguments
        let args: Vec<String> = self
            .config
            .args
            .iter()
            .map(|arg| arg.replace("{ssh_command}", &escaped_command))
            .collect();

        Logger::debug(&format!(
            "Launching terminal: {} with args: {:?}",
            self.config.program, args
        ));

        // Spawn the terminal process
        match Command::new(&self.config.program).args(&args).spawn() {
            Ok(_) => {
                Logger::info(&format!(
                    "Successfully launched terminal for host: {}",
                    host.name
                ));
                Ok(())
            }
            Err(e) => {
                Logger::error(&format!(
                    "Failed to launch terminal for host '{}': {}",
                    host.name, e
                ));
                Logger::error(&format!("  Terminal program: {}", self.config.program));
                Logger::error(&format!("  Terminal args: {:?}", args));
                Logger::error(
                    "  Check that the terminal program exists and the configuration is correct",
                );
                Err(e).with_context(|| {
                    format!(
                        "Failed to launch terminal: {} with args: {:?}",
                        self.config.program, args
                    )
                })
            }
        }
    }
}

fn escape_shell_command(command: &str) -> String {
    // Escape special shell characters to prevent command injection
    command
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("'", "\\'")
        .replace(";", "\\;")
        .replace("&", "\\&")
        .replace("|", "\\|")
        .replace("$", "\\$")
        .replace("`", "\\`")
        .replace("(", "\\(")
        .replace(")", "\\)")
        .replace("<", "\\<")
        .replace(">", "\\>")
        .replace("\n", "\\n")
        .replace("\t", "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_shell_command_basic() {
        let command = "ssh user@server.com";
        let escaped = escape_shell_command(command);
        assert_eq!(escaped, "ssh user@server.com");
    }

    #[test]
    fn test_escape_shell_command_with_special_chars() {
        let command = "ssh user@server.com; echo 'hacked'";
        let escaped = escape_shell_command(command);
        assert_eq!(escaped, "ssh user@server.com\\; echo \\'hacked\\'");
    }

    #[test]
    fn test_escape_shell_command_with_quotes() {
        let command = "ssh user@server.com -t \"sudo su\"";
        let escaped = escape_shell_command(command);
        assert_eq!(escaped, "ssh user@server.com -t \\\"sudo su\\\"");
    }

    #[test]
    fn test_escape_shell_command_with_dollar_and_backticks() {
        let command = "ssh user@server.com -t 'echo $HOME && `whoami`'";
        let escaped = escape_shell_command(command);
        assert_eq!(
            escaped,
            "ssh user@server.com -t \\'echo \\$HOME \\&\\& \\`whoami\\`\\'"
        );
    }

    #[test]
    fn test_launcher_substitutes_ssh_command() {
        let config = TerminalConfig {
            program: "/usr/bin/osascript".to_string(),
            args: vec![
                "-e".to_string(),
                "tell app \"Terminal\" to do script \"{ssh_command}\"".to_string(),
            ],
        };

        let _launcher = TerminalLauncher::new(config.clone());
        let host = HostEntry::new(
            "test-server".to_string(),
            "ssh user@test-server.com".to_string(),
        );

        // We can't easily test the actual launch without mocking, but we can test the escaping
        let escaped = escape_shell_command(&host.connection_string);
        assert_eq!(escaped, "ssh user@test-server.com");

        // Verify substitution would work
        let substituted = config.args[1].replace("{ssh_command}", &escaped);
        assert_eq!(
            substituted,
            "tell app \"Terminal\" to do script \"ssh user@test-server.com\""
        );
    }

    #[test]
    fn test_launcher_handles_multiple_placeholders() {
        let config = TerminalConfig {
            program: "/usr/bin/terminal".to_string(),
            args: vec![
                "--title".to_string(),
                "SSH: {ssh_command}".to_string(),
                "--execute".to_string(),
                "{ssh_command}".to_string(),
            ],
        };

        let _launcher = TerminalLauncher::new(config.clone());
        let host = HostEntry::new("server".to_string(), "ssh user@server".to_string());

        let escaped = escape_shell_command(&host.connection_string);

        // Test substitution in all args
        let args: Vec<String> = config
            .args
            .iter()
            .map(|arg| arg.replace("{ssh_command}", &escaped))
            .collect();

        assert_eq!(args[0], "--title");
        assert_eq!(args[1], "SSH: ssh user@server");
        assert_eq!(args[2], "--execute");
        assert_eq!(args[3], "ssh user@server");
    }

    #[test]
    fn test_escape_comprehensive_special_chars() {
        let dangerous_command =
            "ssh user@server && rm -rf / | echo \"gotcha\" > /tmp/evil; $(whoami)";
        let escaped = escape_shell_command(dangerous_command);

        // Verify all dangerous characters are escaped
        assert!(escaped.contains("\\&\\&"));
        assert!(escaped.contains("\\|"));
        assert!(escaped.contains("\\;"));
        assert!(escaped.contains("\\\""));
        assert!(escaped.contains("\\$"));
        assert!(escaped.contains("\\("));
        assert!(escaped.contains("\\)"));
        assert!(escaped.contains("\\>"));
    }
}
