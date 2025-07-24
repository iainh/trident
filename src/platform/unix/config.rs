// ABOUTME: Unix config detector for terminal discovery and SSH path detection
// ABOUTME: Supports desktop file parsing, package manager detection, and XDG compliance

use crate::config::{DetectedTerminal, LaunchStrategy};
use crate::platform::{ConfigDetector, DesktopEnvironment, SshPaths};
use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use which::which;

pub struct UnixConfigDetector;

impl UnixConfigDetector {
    pub fn new() -> Self {
        Self
    }

    fn detect_desktop_environment() -> DesktopEnvironment {
        if let Ok(de) = std::env::var("XDG_CURRENT_DESKTOP") {
            match de.to_lowercase().as_str() {
                "gnome" | "ubuntu:gnome-shell" => DesktopEnvironment::Gnome,
                "kde" | "plasma" => DesktopEnvironment::Kde,
                "xfce" => DesktopEnvironment::Xfce,
                "i3" => DesktopEnvironment::I3,
                "sway" => DesktopEnvironment::Sway,
                _ => DesktopEnvironment::Unknown,
            }
        } else {
            DesktopEnvironment::Unknown
        }
    }

    fn get_desktop_file_paths() -> Vec<PathBuf> {
        let mut paths = vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
        ];

        if let Ok(home) = std::env::var("HOME") {
            paths.push(PathBuf::from(format!("{}/.local/share/applications", home)));
        }

        if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
            for dir in xdg_data_dirs.split(':') {
                if !dir.is_empty() {
                    paths.push(PathBuf::from(dir).join("applications"));
                }
            }
        }

        paths
    }

    #[cfg(target_os = "linux")]
    fn parse_desktop_file(file_path: &Path) -> Option<DetectedTerminal> {
        use freedesktop_desktop_entry::{DesktopEntry, Type as DesktopEntryType};

        if let Ok(bytes) = fs::read(file_path) {
            if let Ok(desktop_entry) = DesktopEntry::from_bytes(&bytes) {
                if desktop_entry.type_() != Some(DesktopEntryType::Application) {
                    return None;
                }

                let name = desktop_entry.name(None)?.to_string();
                let exec = desktop_entry.exec()?.to_string();

                let exec_clean = exec.split(' ').next().unwrap_or("");
                if exec_clean.is_empty() {
                    return None;
                }

                let lower_name = name.to_lowercase();
                let lower_exec = exec.to_lowercase();

                if Self::is_terminal_application(&lower_name, &lower_exec, exec_clean) {
                    let (args, strategy) = Self::get_terminal_args_and_strategy(exec_clean);
                    return Some(DetectedTerminal {
                        name,
                        program: exec_clean.to_string(),
                        args,
                        strategy,
                    });
                }
            }
        }
        None
    }

    fn is_terminal_application(name: &str, exec: &str, program: &str) -> bool {
        let terminal_keywords = [
            "terminal",
            "console",
            "term",
            "shell",
            "prompt",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "alacritty",
            "kitty",
            "wezterm",
            "tilix",
            "terminator",
            "urxvt",
            "rxvt",
            "xterm",
            "eterm",
            "aterm",
            "hyper",
            "terminus",
            "tabby",
        ];

        terminal_keywords.iter().any(|keyword| {
            name.contains(keyword) || exec.contains(keyword) || program.contains(keyword)
        })
    }

    fn get_terminal_args_and_strategy(program: &str) -> (Vec<String>, LaunchStrategy) {
        let lower_program = program.to_lowercase();

        match lower_program.as_str() {
            p if p.contains("gnome-terminal") => (vec!["--".to_string()], LaunchStrategy::Direct),
            p if p.contains("konsole") => (vec!["-e".to_string()], LaunchStrategy::Direct),
            p if p.contains("xfce4-terminal") => (vec!["-e".to_string()], LaunchStrategy::Direct),
            p if p.contains("tilix") => (vec!["-e".to_string()], LaunchStrategy::Direct),
            p if p.contains("terminator") => (vec!["-e".to_string()], LaunchStrategy::Direct),
            p if p.contains("wezterm") => (vec!["start".to_string()], LaunchStrategy::Direct),
            p if p.contains("alacritty") => (
                vec![
                    "-e".to_string(),
                    "sh",
                    "-c".to_string(),
                    "{ssh_command}".to_string(),
                ],
                LaunchStrategy::ShellCommand,
            ),
            p if p.contains("kitty") => (
                vec!["sh", "-c".to_string(), "{ssh_command}".to_string()],
                LaunchStrategy::ShellCommand,
            ),
            p if p.contains("hyper") => (
                vec!["-e".to_string(), "{ssh_command}".to_string()],
                LaunchStrategy::ShellCommand,
            ),
            p if p.contains("tabby") => (
                vec!["run".to_string(), "{ssh_command}".to_string()],
                LaunchStrategy::ShellCommand,
            ),
            p if p.contains("urxvt") || p.contains("rxvt") => (
                vec![
                    "-e".to_string(),
                    "sh",
                    "-c".to_string(),
                    "{ssh_command}".to_string(),
                ],
                LaunchStrategy::ShellCommand,
            ),
            p if p.contains("xterm") => (
                vec![
                    "-e".to_string(),
                    "sh",
                    "-c".to_string(),
                    "{ssh_command}".to_string(),
                ],
                LaunchStrategy::ShellCommand,
            ),
            _ => (vec!["-e".to_string()], LaunchStrategy::Direct), // Generic fallback
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn parse_desktop_file(_file_path: &Path) -> Option<DetectedTerminal> {
        None
    }

    fn get_common_unix_terminals() -> Vec<DetectedTerminal> {
        vec![
            DetectedTerminal {
                name: "GNOME Terminal".to_string(),
                program: "gnome-terminal".to_string(),
                args: vec!["--".to_string()],
                strategy: LaunchStrategy::Direct,
            },
            DetectedTerminal {
                name: "Konsole".to_string(),
                program: "konsole".to_string(),
                args: vec!["-e".to_string()],
                strategy: LaunchStrategy::Direct,
            },
            DetectedTerminal {
                name: "Alacritty".to_string(),
                program: "alacritty".to_string(),
                args: vec![
                    "-e".to_string(),
                    "sh",
                    "-c".to_string(),
                    "{ssh_command}".to_string(),
                ],
                strategy: LaunchStrategy::ShellCommand,
            },
            DetectedTerminal {
                name: "Kitty".to_string(),
                program: "kitty".to_string(),
                args: vec!["sh", "-c".to_string(), "{ssh_command}".to_string()],
                strategy: LaunchStrategy::ShellCommand,
            },
            DetectedTerminal {
                name: "WezTerm".to_string(),
                program: "wezterm".to_string(),
                args: vec!["start".to_string()],
                strategy: LaunchStrategy::Direct,
            },
            DetectedTerminal {
                name: "XFCE Terminal".to_string(),
                program: "xfce4-terminal".to_string(),
                args: vec!["-e".to_string()],
                strategy: LaunchStrategy::Direct,
            },
            DetectedTerminal {
                name: "Tilix".to_string(),
                program: "tilix".to_string(),
                args: vec!["-e".to_string()],
                strategy: LaunchStrategy::Direct,
            },
            DetectedTerminal {
                name: "Terminator".to_string(),
                program: "terminator".to_string(),
                args: vec!["-e".to_string()],
                strategy: LaunchStrategy::Direct,
            },
            // ... add other common terminals here
        ]
    }
}

impl ConfigDetector for UnixConfigDetector {
    fn detect_terminals(&self) -> Vec<DetectedTerminal> {
        let mut detected = HashSet::new();

        // 1. Detect from common terminals list
        for terminal in Self::get_common_unix_terminals() {
            if which(&terminal.program).is_ok() {
                detected.insert(terminal);
            }
        }

        // 2. Detect via desktop files (on Linux)
        if cfg!(target_os = "linux") {
            for terminal in self.detect_via_desktop_files() {
                if which(&terminal.program).is_ok() {
                    detected.insert(terminal);
                }
            }
        }

        // 3. FreeBSD-specific detection
        if cfg!(target_os = "freebsd") {
            for terminal in self.detect_freebsd_ports_terminals() {
                detected.insert(terminal);
            }
        }

        detected.into_iter().collect()
    }

    #[cfg(target_os = "freebsd")]
    fn detect_freebsd_ports_terminals(&self) -> Vec<DetectedTerminal> {
        let mut terminals = Vec::new();
        let ports_terminals = [
            ("xterm", "/usr/local/bin/xterm"),
            ("rxvt-unicode", "/usr/local/bin/urxvt"),
            ("alacritty", "/usr/local/bin/alacritty"),
            ("kitty", "/usr/local/bin/kitty"),
        ];

        for (name, path) in ports_terminals {
            if Path::new(path).exists() {
                let (args, strategy) = Self::get_terminal_args_and_strategy(name);
                terminals.push(DetectedTerminal {
                    name: name.to_string(),
                    program: path.to_string(),
                    args,
                    strategy,
                });
            }
        }
        terminals
    }

    #[cfg(not(target_os = "freebsd"))]
    fn detect_freebsd_ports_terminals(&self) -> Vec<DetectedTerminal> {
        Vec::new()
    }

    fn get_default_ssh_paths(&self) -> SshPaths {
        let ssh_binary =
            which("ssh").map_or_else(|_| "ssh".to_string(), |p| p.to_string_lossy().into_owned());

        SshPaths {
            known_hosts_path: "~/.ssh/known_hosts".to_string(),
            config_path: "~/.ssh/config".to_string(),
            ssh_binary,
        }
    }

    fn detect_via_desktop_files(&self) -> Vec<DetectedTerminal> {
        let mut detected = Vec::new();

        for apps_dir in Self::get_desktop_file_paths() {
            if let Ok(entries) = fs::read_dir(&apps_dir) {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            if let Some(ext) = entry.path().extension() {
                                if ext == "desktop" {
                                    if let Some(terminal) = Self::parse_desktop_file(&entry.path())
                                    {
                                        detected.push(terminal);
                                    }
                                }
                            }
                        }
                        Err(e) => tracing::warn!(
                            "Failed to read directory entry in {}: {}",
                            apps_dir.display(),
                            e
                        ),
                    }
                }
            }
        }

        detected
    }

    fn handle_desktop_environment(&self, de: &DesktopEnvironment) -> Result<()> {
        match de {
            DesktopEnvironment::Gnome => {
                tracing::info!("Detected GNOME desktop environment");
            }
            DesktopEnvironment::Kde => {
                tracing::info!("Detected KDE desktop environment");
            }
            DesktopEnvironment::Xfce => {
                tracing::info!("Detected XFCE desktop environment");
            }
            DesktopEnvironment::I3 | DesktopEnvironment::Sway => {
                tracing::info!("Detected tiling window manager");
            }
            DesktopEnvironment::Unknown => {
                tracing::info!("Unknown desktop environment");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_desktop_environment() {
        let _de = UnixConfigDetector::detect_desktop_environment();
    }

    #[test]
    fn test_get_default_ssh_paths() {
        let paths = UnixConfigDetector::get_default_ssh_paths();
        assert!(!paths.ssh_binary.is_empty());
        assert!(paths.known_hosts_path.contains(".ssh/known_hosts"));
    }

    #[test]
    fn test_common_terminals_list() {
        let terminals = UnixConfigDetector::get_common_unix_terminals();
        assert!(!terminals.is_empty());

        assert!(terminals.iter().any(|t| t.name == "GNOME Terminal"));
    }
}
