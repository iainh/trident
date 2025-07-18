// ABOUTME: SSH file parsing module for extracting host information from known_hosts and SSH config
// ABOUTME: Provides simple parsing focused on extracting hostnames for fuzzy search

pub mod parser;

pub use parser::{HostEntry, parse_known_hosts, parse_ssh_config};