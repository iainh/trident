// ABOUTME: SSH file parsing and terminal launching module for SSH connections
// ABOUTME: Provides simple parsing focused on extracting hostnames for fuzzy search and safe terminal launching

pub mod launcher;
pub mod parser;

pub use launcher::TerminalLauncher;
pub use parser::{HostEntry, parse_known_hosts, parse_ssh_config};
