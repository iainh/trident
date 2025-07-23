// ABOUTME: Unix config detector for terminal discovery and SSH path detection
// ABOUTME: Supports desktop file parsing, package manager detection, and XDG compliance

use crate::platform::{ConfigDetector, DetectedTerminal, SshPaths, DesktopEnvironment};
use anyhow::Result;
use std::path::Path;
use std::fs;

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

    fn get_desktop_file_paths() -> Vec<String> {
        let mut paths = vec![
            "/usr/share/applications".to_string(),
            "/usr/local/share/applications".to_string(),
        ];

        // Add user-specific paths
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{}/.local/share/applications", home));
        }

        // XDG_DATA_DIRS
        if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
            for dir in xdg_data_dirs.split(':') {
                if !dir.is_empty() {
                    paths.push(format!("{}/applications", dir));
                }
            }
        }

        paths
    }

    fn check_program_exists(program: &str) -> bool {
        // Check if program exists in PATH or as absolute path
        if Path::new(program).is_absolute() {
            Path::new(program).exists()
        } else {
            // Check in PATH
            if let Ok(path_var) = std::env::var("PATH") {
                for path_dir in path_var.split(':') {
                    let full_path = Path::new(path_dir).join(program);
                    if full_path.exists() {
                        return true;
                    }
                }
            }
            false
        }
    }

    #[cfg(target_os = "linux")]
    fn parse_desktop_file(file_path: &Path) -> Option<DetectedTerminal> {
        use freedesktop_desktop_entry::DesktopEntry;

        if let Ok(bytes) = fs::read(file_path) {
            if let Ok(desktop_entry) = DesktopEntry::from_bytes(&bytes) {
                let name = desktop_entry.name(None)?.to_string();
                let exec = desktop_entry.exec()?.to_string();
                
                // Parse exec string to extract program and args
                // This is simplified - full implementation would handle Exec field properly
                let parts: Vec<&str> = exec.split_whitespace().collect();
                if let Some(program) = parts.first() {
                    // Check if this is a terminal emulator
                    let lower_name = name.to_lowercase();
                    if lower_name.contains("terminal") || 
                       lower_name.contains("console") ||
                       program.contains("terminal") ||
                       program.contains("term") {
                        
                        let detection_paths = vec![program.to_string()];
                        
                        // Generate appropriate args based on known terminals
                        let args = match program.to_lowercase().as_str() {
                            p if p.contains("gnome-terminal") => vec!["--".to_string(), "{ssh_command}".to_string()],
                            p if p.contains("konsole") => vec!["-e".to_string(), "{ssh_command}".to_string()],
                            p if p.contains("xfce4-terminal") => vec!["-e".to_string(), "{ssh_command}".to_string()],
                            p if p.contains("alacritty") => vec!["-e".to_string(), "sh".to_string(), "-c".to_string(), "{ssh_command}".to_string()],
                            p if p.contains("kitty") => vec!["sh".to_string(), "-c".to_string(), "{ssh_command}".to_string()],
                            p if p.contains("wezterm") => vec!["start".to_string(), "{ssh_command}".to_string()],
                            _ => vec!["-e".to_string(), "{ssh_command}".to_string()], // Generic fallback
                        };

                        return Some(DetectedTerminal {
                            name,
                            program: program.to_string(),
                            args,
                            desktop_file: Some(file_path.to_string_lossy().to_string()),
                            detection_paths,
                        });
                    }
                }
            }
        }
        None
    }

    #[cfg(not(target_os = "linux"))]
    fn parse_desktop_file(_file_path: &Path) -> Option<DetectedTerminal> {
        None // FreeBSD doesn't use freedesktop_desktop_entry
    }

    fn get_common_unix_terminals() -> Vec<DetectedTerminal> {
        vec![
            DetectedTerminal {
                name: "GNOME Terminal".to_string(),
                program: "gnome-terminal".to_string(),
                args: vec!["--".to_string(), "{ssh_command}".to_string()],
                desktop_file: Some("org.gnome.Terminal.desktop".to_string()),
                detection_paths: vec![
                    "/usr/bin/gnome-terminal".to_string(),
                    "/usr/local/bin/gnome-terminal".to_string(),
                ],
            },
            DetectedTerminal {
                name: "Konsole".to_string(),
                program: "konsole".to_string(),
                args: vec!["-e".to_string(), "{ssh_command}".to_string()],
                desktop_file: Some("org.kde.konsole.desktop".to_string()),
                detection_paths: vec![
                    "/usr/bin/konsole".to_string(),
                    "/usr/local/bin/konsole".to_string(),
                ],
            },
            DetectedTerminal {
                name: "Alacritty".to_string(),
                program: "alacritty".to_string(),
                args: vec!["-e".to_string(), "sh".to_string(), "-c".to_string(), "{ssh_command}".to_string()],
                desktop_file: Some("Alacritty.desktop".to_string()),
                detection_paths: vec![
                    "/usr/bin/alacritty".to_string(),
                    "/usr/local/bin/alacritty".to_string(),
                    "/home/.cargo/bin/alacritty".to_string(),
                ],
            },
            DetectedTerminal {
                name: "Kitty".to_string(),
                program: "kitty".to_string(),
                args: vec!["sh".to_string(), "-c".to_string(), "{ssh_command}".to_string()],
                desktop_file: Some("kitty.desktop".to_string()),
                detection_paths: vec![
                    "/usr/bin/kitty".to_string(),
                    "/usr/local/bin/kitty".to_string(),
                ],
            },
            DetectedTerminal {
                name: "WezTerm".to_string(),
                program: "wezterm".to_string(),
                args: vec!["start".to_string(), "{ssh_command}".to_string()],
                desktop_file: Some("org.wezfurlong.wezterm.desktop".to_string()),
                detection_paths: vec![
                    "/usr/bin/wezterm".to_string(),
                    "/usr/local/bin/wezterm".to_string(),
                ],
            },
            DetectedTerminal {
                name: "XFCE Terminal".to_string(),
                program: "xfce4-terminal".to_string(),
                args: vec!["-e".to_string(), "{ssh_command}".to_string()],
                desktop_file: Some("xfce4-terminal.desktop".to_string()),
                detection_paths: vec![
                    "/usr/bin/xfce4-terminal".to_string(),
                    "/usr/local/bin/xfce4-terminal".to_string(),
                ],
            },
            DetectedTerminal {
                name: "xterm".to_string(),
                program: "xterm".to_string(),
                args: vec!["-e".to_string(), "{ssh_command}".to_string()],
                desktop_file: None,
                detection_paths: vec![
                    "/usr/bin/xterm".to_string(),
                    "/usr/local/bin/xterm".to_string(),
                ],
            },
        ]
    }
}

impl ConfigDetector for UnixConfigDetector {
    fn detect_terminals(&self) -> Vec<DetectedTerminal> {
        let mut detected = Vec::new();
        let common_terminals = Self::get_common_unix_terminals();

        // Check which common terminals are available
        for terminal in common_terminals {
            for path in &terminal.detection_paths {
                if Self::check_program_exists(path) {
                    detected.push(terminal.clone());
                    break; // Found this terminal, move to next
                }
            }
        }

        detected
    }

    fn get_default_ssh_paths(&self) -> SshPaths {
        let ssh_binary = if Self::check_program_exists("/usr/bin/ssh") {
            "/usr/bin/ssh".to_string()
        } else if Self::check_program_exists("/usr/local/bin/ssh") {
            "/usr/local/bin/ssh".to_string()
        } else {
            "ssh".to_string() // Hope it's in PATH
        };

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
                for entry in entries.flatten() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "desktop" {
                            if let Some(terminal) = Self::parse_desktop_file(&entry.path()) {
                                detected.push(terminal);
                            }
                        }
                    }
                }
            }
        }

        detected
    }

    fn handle_desktop_environment(&self, de: &DesktopEnvironment) -> Result<()> {
        match de {
            DesktopEnvironment::Gnome => {
                println!("Detected GNOME desktop environment");
                println!("Recommended terminal: GNOME Terminal");
            }
            DesktopEnvironment::Kde => {
                println!("Detected KDE desktop environment"); 
                println!("Recommended terminal: Konsole");
            }
            DesktopEnvironment::Xfce => {
                println!("Detected XFCE desktop environment");
                println!("Recommended terminal: XFCE Terminal");
            }
            DesktopEnvironment::I3 | DesktopEnvironment::Sway => {
                println!("Detected tiling window manager");
                println!("Recommended terminals: Alacritty, Kitty");
            }
            DesktopEnvironment::Unknown => {
                println!("Unknown desktop environment");
                println!("Using generic terminal detection");
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
        // This test will vary based on the actual environment
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
        
        // Check that GNOME Terminal is in the list
        assert!(terminals.iter().any(|t| t.name == "GNOME Terminal"));
    }
}