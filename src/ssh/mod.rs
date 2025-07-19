// ABOUTME: SSH file parsing and terminal launching module for SSH connections
// ABOUTME: Provides simple parsing focused on extracting hostnames for fuzzy search and safe terminal launching

pub mod parser;
pub mod launcher;

pub use parser::{HostEntry, parse_known_hosts, parse_ssh_config};
pub use launcher::TerminalLauncher;