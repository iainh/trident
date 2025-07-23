// ABOUTME: Unix terminal launcher supporting various Linux/FreeBSD terminals
// ABOUTME: Handles terminal detection, launching, and desktop environment integration

use crate::platform::TerminalLauncher as PlatformTerminalLauncher;
use crate::config::TerminalConfig;
use crate::ssh::HostEntry;
use anyhow::{Result, anyhow};
use std::process::Command;

pub struct UnixTerminalLauncher {
    config: TerminalConfig,
}

impl UnixTerminalLauncher {
    pub fn new(config: TerminalConfig) -> Self {
        Self { config }
    }

    fn escape_shell_command(command: &str) -> String {
        // Reuse the shell escaping logic from the existing launcher
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

    fn bring_terminal_to_front_unix(&self, app_name: &str) -> Result<()> {
        // Try various methods to bring the terminal to front on Unix
        
        // Method 1: Try wmctrl if available
        if let Ok(output) = Command::new("wmctrl").arg("-l").output() {
            if output.status.success() {
                // wmctrl is available, try to activate the window
                let _ = Command::new("wmctrl")
                    .args(["-a", app_name])
                    .output();
            }
        }

        // Method 2: Try xdotool if available
        if let Ok(output) = Command::new("xdotool").arg("--version").output() {
            if output.status.success() {
                // xdotool is available, try to search and activate
                let _ = Command::new("xdotool")
                    .args(["search", "--name", app_name, "windowactivate"])
                    .output();
            }
        }

        // Method 3: Desktop environment specific
        self.try_de_specific_activation(app_name)?;

        Ok(())
    }

    fn try_de_specific_activation(&self, app_name: &str) -> Result<()> {
        // Detect desktop environment and use appropriate activation method
        if let Ok(de) = std::env::var("XDG_CURRENT_DESKTOP") {
            match de.to_lowercase().as_str() {
                "gnome" | "ubuntu:gnome-shell" => {
                    // Use gdbus to interact with GNOME Shell
                    let _ = Command::new("gdbus")
                        .args([
                            "call", "--session", "--dest", "org.gnome.Shell",
                            "--object-path", "/org/gnome/Shell",
                            "--method", "org.gnome.Shell.Eval",
                            &format!("global.get_window_actors().find(a => a.get_meta_window().get_title().includes('{}')).get_meta_window().activate(global.get_current_time())", app_name)
                        ])
                        .output();
                }
                "kde" | "plasma" => {
                    // Use KDE's qdbus
                    let _ = Command::new("qdbus")
                        .args(["org.kde.kglobalaccel", "/component/kwin", "invokeShortcut", "Activate Window Demanding Attention"])
                        .output();
                }
                _ => {
                    // Unknown DE, no specific activation
                }
            }
        }
        Ok(())
    }
}

impl PlatformTerminalLauncher for UnixTerminalLauncher {
    fn launch_command(&self, command: &str, config: &TerminalConfig) -> Result<()> {
        let escaped_command = Self::escape_shell_command(command);
        
        // Substitute {ssh_command} placeholder in terminal arguments
        let args: Vec<String> = config
            .args
            .iter()
            .map(|arg| arg.replace("{ssh_command}", &escaped_command))
            .collect();

        println!("[DEBUG] Launching Unix terminal: {} with args: {:?}", config.program, args);

        // Spawn the terminal process
        let mut cmd = Command::new(&config.program);
        if !args.is_empty() {
            cmd.args(&args);
        }

        match cmd.spawn() {
            Ok(_) => {
                println!("[INFO] Successfully launched terminal with command: {}", command);
                
                // Try to bring terminal to front
                if let Some(app_name) = std::path::Path::new(&config.program).file_name() {
                    if let Some(name_str) = app_name.to_str() {
                        if let Err(e) = self.bring_terminal_to_front_unix(name_str) {
                            println!("[DEBUG] Failed to bring terminal to front: {}", e);
                        }
                    }
                }
                
                Ok(())
            }
            Err(e) => {
                Err(anyhow!(
                    "Failed to launch terminal '{}' with command '{}': {}",
                    config.program, command, e
                ))
            }
        }
    }

    fn bring_to_front(&self, app_name: &str) -> Result<()> {
        self.bring_terminal_to_front_unix(app_name)
    }

    fn launch_host(&self, host: &HostEntry) -> Result<()> {
        self.launch_command(&host.connection_string, &self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_shell_command() {
        let dangerous = "ssh user@host && rm -rf /";
        let escaped = UnixTerminalLauncher::escape_shell_command(dangerous);
        assert!(escaped.contains("\\&\\&"));
    }

    #[test]
    fn test_unix_launcher_creation() {
        let config = TerminalConfig {
            program: "/usr/bin/gnome-terminal".to_string(),
            args: vec!["--".to_string(), "{ssh_command}".to_string()],
        };
        let _launcher = UnixTerminalLauncher::new(config);
    }
}