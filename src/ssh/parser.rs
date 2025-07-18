// ABOUTME: Simple SSH file parsers for extracting host entries from known_hosts and SSH config files
// ABOUTME: Implements configuration-driven parsing with support for skipping complex features

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostEntry {
    pub name: String,           // What user types to match
    pub connection_string: String, // What gets passed to SSH
}

impl HostEntry {
    pub fn new(name: String, connection_string: String) -> Self {
        Self { name, connection_string }
    }
}

pub fn parse_known_hosts(path: &Path, skip_hashed: bool) -> Result<Vec<HostEntry>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read known_hosts file: {}", path.display()))?;
    
    Ok(parse_known_hosts_content(&content, skip_hashed))
}

fn parse_known_hosts_content(content: &str, skip_hashed: bool) -> Vec<HostEntry> {
    let mut entries = Vec::new();
    
    for line in content.lines() {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Skip hashed entries if configured
        if skip_hashed && line.starts_with('|') {
            continue;
        }
        
        // Extract hostname(s) from the line
        if let Some(hosts_part) = line.split_whitespace().next() {
            // Handle comma-separated hosts
            for host in hosts_part.split(',') {
                let host = host.trim();
                
                // Skip IP addresses (simple check)
                if host.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    continue;
                }
                
                // Skip ports specified with brackets like [hostname]:port
                let clean_host = if host.starts_with('[') && host.contains("]:") {
                    if let Some(end) = host.find("]:") {
                        &host[1..end]
                    } else {
                        host
                    }
                } else {
                    host
                };
                
                if !clean_host.is_empty() && !clean_host.starts_with('|') {
                    entries.push(HostEntry::new(
                        clean_host.to_string(),
                        format!("ssh {}", clean_host),
                    ));
                }
            }
        }
    }
    
    // Remove duplicates
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries.dedup_by(|a, b| a.name == b.name);
    
    entries
}

pub fn parse_ssh_config(path: &Path, simple_parsing: bool) -> Result<Vec<HostEntry>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read SSH config file: {}", path.display()))?;
    
    Ok(parse_ssh_config_content(&content, simple_parsing))
}

fn parse_ssh_config_content(content: &str, _simple_parsing: bool) -> Vec<HostEntry> {
    let mut entries = Vec::new();
    let mut current_host: Option<String> = None;
    
    for line in content.lines() {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Split by whitespace to get key-value pairs
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        
        let key = parts[0].to_lowercase();
        let value = parts[1..].join(" ");
        
        match key.as_str() {
            "host" => {
                // Process the previous host if any
                if let Some(host) = current_host.take() {
                    if !host.contains('*') && !host.contains('?') {
                        entries.push(HostEntry::new(
                            host.clone(),
                            format!("ssh {}", host),
                        ));
                    }
                }
                
                // Start new host
                current_host = Some(value);
            }
            _ => {
                // For simple parsing, we ignore all other directives
            }
        }
    }
    
    // Process the last host
    if let Some(host) = current_host {
        if !host.contains('*') && !host.contains('?') {
            entries.push(HostEntry::new(
                host.clone(),
                format!("ssh {}", host),
            ));
        }
    }
    
    // Remove duplicates
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries.dedup_by(|a, b| a.name == b.name);
    
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_known_hosts_simple() {
        let content = "example.com ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEA...
server1.local ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI...
192.168.1.100 ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEA...";
        
        let entries = parse_known_hosts_content(content, false);
        
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "example.com");
        assert_eq!(entries[0].connection_string, "ssh example.com");
        assert_eq!(entries[1].name, "server1.local");
        assert_eq!(entries[1].connection_string, "ssh server1.local");
    }
    
    #[test]
    fn test_parse_known_hosts_with_ports() {
        let content = "[example.com]:2222 ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEA...
[server.local]:22 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI...";
        
        let entries = parse_known_hosts_content(content, false);
        
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "example.com");
        assert_eq!(entries[1].name, "server.local");
    }
    
    #[test]
    fn test_parse_known_hosts_skip_hashed() {
        let content = "|1|hash1= ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEA...
example.com ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEA...
|1|hash2= ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI...";
        
        let entries = parse_known_hosts_content(content, true);
        
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "example.com");
    }
    
    #[test]
    fn test_parse_known_hosts_comma_separated() {
        let content = "example.com,alias.example.com,10.0.0.1 ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEA...";
        
        let entries = parse_known_hosts_content(content, false);
        
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.name == "example.com"));
        assert!(entries.iter().any(|e| e.name == "alias.example.com"));
    }
    
    #[test]
    fn test_parse_ssh_config_simple() {
        let content = "# SSH Config file
Host myserver
    HostName example.com
    User admin
    Port 2222

Host production
    HostName prod.example.com
    
Host *.internal
    User root
    
Host github.com
    User git";
        
        let entries = parse_ssh_config_content(content, true);
        
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().any(|e| e.name == "myserver"));
        assert!(entries.iter().any(|e| e.name == "production"));
        assert!(entries.iter().any(|e| e.name == "github.com"));
        // Wildcard hosts should be skipped
        assert!(!entries.iter().any(|e| e.name.contains('*')));
    }
    
    #[test]
    fn test_parse_ssh_config_empty() {
        let content = "# Empty config with only comments
# and blank lines

";
        
        let entries = parse_ssh_config_content(content, true);
        
        assert_eq!(entries.len(), 0);
    }
    
    #[test]
    fn test_host_entry_deduplication() {
        let content = "example.com ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEA...
example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI...";
        
        let entries = parse_known_hosts_content(content, false);
        
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "example.com");
    }
}