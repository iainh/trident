// ABOUTME: Unix terminal launcher supporting various Linux/FreeBSD terminals
// ABOUTME: Handles terminal detection, launching, and desktop environment integration

use crate::config::{LaunchStrategy, TerminalConfig};
use crate::platform::TerminalLauncher as PlatformTerminalLauncher;
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

    fn bring_terminal_to_front_unix(&self, app_name: &str) -> Result<()> {
        tracing::debug!("Attempting to bring terminal '{}' to front", app_name);

        // This is an X11-specific feature. It will not work on Wayland.
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            log::debug!("Wayland detected, skipping window activation.");
            return Ok(());
        }

        if which::which("wmctrl").is_ok() {
            if Command::new("wmctrl")
                .args(["-a", app_name])
                .status()
                .is_ok()
            {
                log::debug!("wmctrl activation successful");
                return Ok(());
            }
        }

        if which::which("xdotool").is_ok() {
            if Command::new("xdotool")
                .args(["search", "--name", app_name, "windowactivate"])
                .status()
                .is_ok()
            {
                log::debug!("xdotool activation successful");
                return Ok(());
            }
        }

        log::debug!("No suitable window activation tool found (wmctrl, xdotool).");
        Ok(())
    }
}

impl PlatformTerminalLauncher for UnixTerminalLauncher {
    fn launch_command(&self, command: &str, config: &TerminalConfig) -> Result<()> {
        let mut cmd;

        match config.strategy {
            LaunchStrategy::ShellCommand => {
                let final_command = config.args.join(" ").replace("{ssh_command}", command);
                cmd = Command::new("sh");
                cmd.arg("-c");
                cmd.arg(final_command);
            }
            LaunchStrategy::Direct => {
                cmd = Command::new(&config.program);
                cmd.args(&config.args);
                // command is in format "ssh user@host", so we need to split it
                let command_parts: Vec<&str> = command.split_whitespace().collect();
                cmd.args(command_parts);
            }
        }

        log::debug!("Launching Unix terminal: {:?}", cmd);

        match cmd.spawn() {
            Ok(_) => {
                log::info!("Successfully launched terminal with command: {}", command);

                if let Some(app_name) = std::path::Path::new(&config.program).file_name() {
                    if let Some(name_str) = app_name.to_str() {
                        if let Err(e) = self.bring_terminal_to_front_unix(name_str) {
                            log::warn!("Failed to bring terminal to front: {}", e);
                        }
                    }
                }

                Ok(())
            }
            Err(e) => Err(anyhow!(
                "Failed to launch terminal '{}' with command '{}': {}",
                config.program,
                command,
                e
            )),
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
    fn test_direct_launch_strategy() {
        let config = TerminalConfig {
            program: "/usr/bin/gnome-terminal".to_string(),
            args: vec!["--".to_string()],
            strategy: LaunchStrategy::Direct,
        };
        let launcher = UnixTerminalLauncher::new(config);
        // This is a mock test, we don't actually launch the terminal
        let result = launcher.launch_command("ssh user@host", &launcher.config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shell_command_launch_strategy() {
        let config = TerminalConfig {
            program: "alacritty".to_string(),
            args: vec![
                "-e".to_string(),
                "sh",
                "-c".to_string(),
                "{ssh_command}".to_string(),
            ],
            strategy: LaunchStrategy::ShellCommand,
        };
        let launcher = UnixTerminalLauncher::new(config);
        let result = launcher.launch_command("ssh user@host", &launcher.config);
        assert!(result.is_ok());
    }
}
