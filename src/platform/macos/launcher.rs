// ABOUTME: macOS terminal launcher implementation using existing launcher functionality
// ABOUTME: Wraps the existing TerminalLauncher in the platform abstraction trait

use crate::platform::TerminalLauncher as PlatformTerminalLauncher;
use crate::ssh::launcher::TerminalLauncher as ExistingTerminalLauncher;
use crate::config::TerminalConfig;
use crate::ssh::HostEntry;
use anyhow::Result;

pub struct MacOSTerminalLauncher {
    launcher: ExistingTerminalLauncher,
}

impl MacOSTerminalLauncher {
    pub fn new(config: TerminalConfig) -> Self {
        Self {
            launcher: ExistingTerminalLauncher::new(config),
        }
    }
}

impl PlatformTerminalLauncher for MacOSTerminalLauncher {
    fn launch_command(&self, command: &str, config: &TerminalConfig) -> Result<()> {
        // Create a temporary host entry to use existing launch logic
        let host = HostEntry::new("manual".to_string(), command.to_string());
        let temp_launcher = ExistingTerminalLauncher::new(config.clone());
        temp_launcher.launch(&host)
    }
    
    fn bring_to_front(&self, app_name: &str) -> Result<()> {
        // Use AppleScript to bring app to front
        use std::process::Command;
        let script = format!("tell application \"{}\" to activate", app_name);
        Command::new("osascript")
            .args(["-e", &script])
            .output()?;
        Ok(())
    }
    
    fn launch_host(&self, host: &HostEntry) -> Result<()> {
        self.launcher.launch(host)
    }
}