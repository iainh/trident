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
        Logger::debug(&format!("Escaped SSH command: {escaped_command}"));

        // Check if we should use macOS 'open' command for app bundles
        if self.should_use_open_command() {
            self.launch_with_open_command(&escaped_command, host)
        } else {
            self.launch_with_direct_execution(&escaped_command, host)
        }
    }

    /// Determine if we should use the 'open' command instead of direct execution
    fn should_use_open_command(&self) -> bool {
        // Use 'open' for app bundles (contains .app/) but not for osascript
        self.config.program.contains(".app/") && !self.config.program.contains("osascript")
    }

    /// Launch using macOS 'open' command (automatically brings app to foreground)
    fn launch_with_open_command(&self, escaped_command: &str, host: &HostEntry) -> Result<()> {
        let app_name = extract_app_name(&self.config.program)?;
        
        // Substitute {ssh_command} placeholder in terminal arguments
        let args: Vec<String> = self
            .config
            .args
            .iter()
            .map(|arg| arg.replace("{ssh_command}", escaped_command))
            .collect();

        Logger::debug(&format!(
            "Launching terminal with open command: {} with args: {:?}",
            app_name, args
        ));

        // Build command: open -a "AppName" --args <terminal_args>
        let mut cmd = Command::new("open");
        cmd.args(["-a", &app_name]);
        if !args.is_empty() {
            cmd.arg("--args");
            cmd.args(&args);
        }

        match cmd.spawn() {
            Ok(_) => {
                Logger::info(&format!(
                    "Successfully launched terminal for host: {} (using open command)",
                    host.name
                ));
                Ok(())
            }
            Err(e) => {
                Logger::error(&format!(
                    "Failed to launch terminal with open command for host '{}': {}",
                    host.name, e
                ));
                Logger::error(&format!("  App name: {}", app_name));
                Logger::error(&format!("  Terminal args: {args:?}"));
                Err(e).with_context(|| {
                    format!("Failed to launch terminal with open command: {} with args: {:?}", app_name, args)
                })
            }
        }
    }

    /// Launch using direct binary execution with AppleScript activation fallback
    fn launch_with_direct_execution(&self, escaped_command: &str, host: &HostEntry) -> Result<()> {
        // Substitute {ssh_command} placeholder in terminal arguments
        let args: Vec<String> = self
            .config
            .args
            .iter()
            .map(|arg| arg.replace("{ssh_command}", escaped_command))
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

                // Bring the terminal window to front (unless using osascript which handles this)
                if !self.config.program.contains("osascript") {
                    if let Err(e) = self.bring_terminal_to_front() {
                        Logger::debug(&format!(
                            "Failed to bring terminal to front (terminal still launched): {e}"
                        ));
                    }
                }

                Ok(())
            }
            Err(e) => {
                Logger::error(&format!(
                    "Failed to launch terminal for host '{}': {}",
                    host.name, e
                ));
                Logger::error(&format!("  Terminal program: {}", self.config.program));
                Logger::error(&format!("  Terminal args: {args:?}"));
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

    /// Bring the terminal application to front using AppleScript
    fn bring_terminal_to_front(&self) -> Result<()> {
        let app_name = extract_app_name(&self.config.program)?;
        
        Logger::debug(&format!("Attempting to bring '{}' to front", app_name));

        let script = format!("tell application \"{}\" to activate", app_name);
        
        match Command::new("osascript")
            .args(["-e", &script])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    Logger::debug(&format!("Successfully brought '{}' to front", app_name));
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Logger::debug(&format!("AppleScript failed to activate '{}': {}", app_name, stderr));
                }
                Ok(())
            }
            Err(e) => {
                Logger::debug(&format!("Failed to run AppleScript to activate '{}': {}", app_name, e));
                Err(e.into())
            }
        }
    }
}

/// Extract application name from terminal program path for AppleScript activation
fn extract_app_name(program_path: &str) -> Result<String> {
    // Handle common macOS application patterns
    if let Some(app_bundle_end) = program_path.find(".app/") {
        // Extract app name from path like "/Applications/iTerm.app/Contents/MacOS/iTerm2"
        let app_path = &program_path[..app_bundle_end + 4]; // Include ".app"
        let start = app_path.rfind('/').map(|i| i + 1).unwrap_or(0);
        let app_name = &app_path[start..];
        
        // Remove .app suffix to get clean name
        let clean_name = app_name.strip_suffix(".app").unwrap_or(app_name);
        
        // Handle special case for Ghostty (lowercase process name)
        let final_name = if clean_name.eq_ignore_ascii_case("ghostty") {
            "ghostty"
        } else {
            clean_name
        };
            
        return Ok(final_name.to_string());
    }
    
    // For non-standard paths, try to extract from the final component
    if let Some(last_slash) = program_path.rfind('/') {
        let binary_name = &program_path[last_slash + 1..];
        let lower_name = binary_name.to_lowercase();
        
        // Map common terminal binary names to application names
        let app_name = match lower_name.as_str() {
            "iterm2" => "iTerm2",
            "alacritty" => "Alacritty", 
            "kitty" => "kitty",
            "ghostty" => "ghostty", // Note: lowercase for process name
            "wezterm" => "WezTerm",
            "hyper" => "Hyper",
            _ => binary_name, // Use original case for unknown binaries
        };
        
        Ok(app_name.to_string())
    } else {
        // Fallback: use the program path as-is
        Ok(program_path.to_string())
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

    #[test]
    fn test_extract_app_name_from_app_bundle() {
        assert_eq!(
            extract_app_name("/Applications/iTerm.app/Contents/MacOS/iTerm2").unwrap(),
            "iTerm"
        );
        assert_eq!(
            extract_app_name("/Applications/Alacritty.app/Contents/MacOS/alacritty").unwrap(),
            "Alacritty"
        );
        assert_eq!(
            extract_app_name("/Applications/Ghostty.app/Contents/MacOS/ghostty").unwrap(),
            "ghostty"
        );
    }

    #[test]
    fn test_extract_app_name_from_binary_name() {
        assert_eq!(extract_app_name("/usr/bin/iterm2").unwrap(), "iTerm2");
        assert_eq!(extract_app_name("/usr/local/bin/alacritty").unwrap(), "Alacritty");
        assert_eq!(extract_app_name("/opt/bin/kitty").unwrap(), "kitty");
        assert_eq!(extract_app_name("/usr/bin/ghostty").unwrap(), "ghostty");
        assert_eq!(extract_app_name("/Applications/WezTerm.app/Contents/MacOS/wezterm").unwrap(), "WezTerm");
    }

    #[test]
    fn test_extract_app_name_fallback() {
        assert_eq!(extract_app_name("some-terminal").unwrap(), "some-terminal");
        assert_eq!(extract_app_name("/custom/path/custom-term").unwrap(), "custom-term");
    }

    #[test]
    fn test_should_use_open_command() {
        // Should use open for app bundles
        let config1 = TerminalConfig {
            program: "/Applications/Ghostty.app/Contents/MacOS/ghostty".to_string(),
            args: vec!["-e".to_string(), "{ssh_command}".to_string()],
        };
        let launcher1 = TerminalLauncher::new(config1);
        assert!(launcher1.should_use_open_command());

        // Should use open for iTerm
        let config2 = TerminalConfig {
            program: "/Applications/iTerm.app/Contents/MacOS/iTerm2".to_string(),
            args: vec!["-c".to_string(), "{ssh_command}".to_string()],
        };
        let launcher2 = TerminalLauncher::new(config2);
        assert!(launcher2.should_use_open_command());

        // Should NOT use open for osascript (even though it's for an app)
        let config3 = TerminalConfig {
            program: "/usr/bin/osascript".to_string(),
            args: vec!["-e".to_string(), "tell app \"Terminal\" to do script \"{ssh_command}\"".to_string()],
        };
        let launcher3 = TerminalLauncher::new(config3);
        assert!(!launcher3.should_use_open_command());

        // Should NOT use open for direct binary paths
        let config4 = TerminalConfig {
            program: "/usr/local/bin/alacritty".to_string(),
            args: vec!["-e".to_string(), "{ssh_command}".to_string()],
        };
        let launcher4 = TerminalLauncher::new(config4);
        assert!(!launcher4.should_use_open_command());
    }
}
