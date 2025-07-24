// ABOUTME: macOS config detector wrapping existing terminal detection logic
// ABOUTME: Implements the ConfigDetector trait for macOS platform

use crate::config::DetectedTerminal;
use crate::platform::{ConfigDetector, DesktopEnvironment, SshPaths};
use anyhow::Result;

// Reuse the existing DetectedTerminal from config.rs temporarily
// until we fully migrate the detection logic
use crate::config::Config;

pub struct MacOSConfigDetector;

impl MacOSConfigDetector {
    pub fn new() -> Self {
        Self
    }
}

impl ConfigDetector for MacOSConfigDetector {
    fn detect_terminals(&self) -> Vec<DetectedTerminal> {
        // Convert from existing detection logic
        let detected = Config::detect_best_terminal();

        vec![DetectedTerminal {
            name: detected.name,
            program: detected.program,
            args: detected.args,
            strategy: detected.strategy,
        }]
    }

    fn get_default_ssh_paths(&self) -> SshPaths {
        SshPaths {
            known_hosts_path: "~/.ssh/known_hosts".to_string(),
            config_path: "~/.ssh/config".to_string(),
            ssh_binary: "/usr/bin/ssh".to_string(),
        }
    }

    fn detect_via_desktop_files(&self) -> Vec<DetectedTerminal> {
        // macOS doesn't use desktop files
        vec![]
    }

    fn handle_desktop_environment(&self, _de: &DesktopEnvironment) -> Result<()> {
        // macOS doesn't have multiple desktop environments
        Ok(())
    }
}
