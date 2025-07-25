# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Trident is an SSH Connection Launcher for macOS - a developer productivity tool that provides fuzzy search over known SSH hosts and launches terminal connections via a global hotkey (Cmd+Shift+S).

## Architecture

The project follows a Model-View-Update (MVU) pattern:
- **Model**: Application state (search queries, filtered results)
- **Update**: State transitions based on user input
- **View**: UI rendering using GPUI framework
- **Business Logic**: SSH file parsing, fuzzy search, terminal launching

## Development Commands

### Cargo Commands
```bash
# Build the project
cargo build

# Run the application
cargo run

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run benchmarks
cargo bench

# Check code without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy

# Security audit
cargo audit
```

### Nix Commands
```bash
# Enter development environment
nix develop

# Build macOS .app bundle
nix build
nix run .

# Run comprehensive quality assurance checks
nix flake check

# Run individual QA checks
nix run .#test         # Run tests
nix run .#clippy       # Run linter (warnings as errors)
nix run .#fmt-check    # Check code formatting
nix run .#audit        # Security vulnerability audit
nix run .#deny         # License and dependency checking
nix run .#build-check  # Verify project builds

# Lima Linux development
nix run .#lima-start   # Start Lima VM
nix run .#lima-build   # Build for Linux via Lima
nix run .#lima-test    # Run tests via Lima
nix run .#lima-shell   # Enter Lima development shell
nix run .#lima-stop    # Stop Lima VM
```

## Project Structure

Key directories and their purposes:
- `src/app.rs` - Main application state and MVU logic
- `src/config.rs` - User configuration handling (config.toml parsing)
- `src/ui/` - GPUI UI components (search box, host list)
- `src/ssh/` - SSH file parsing and terminal launching
- `src/fuzzy.rs` - Fuzzy matching algorithms

## Configuration Approach

This project is **configuration-driven**. Users specify their terminal preferences in `config.toml` rather than auto-detecting. This avoids complex terminal detection logic and gives users full control.

Key configuration areas:
- Terminal program path and launch arguments
- SSH file locations (known_hosts, config)
- Parsing behavior (simple vs full SSH config parsing)

## Testing Strategy

Follow Test-Driven Development (TDD):
1. Write failing test first
2. Implement minimal code to pass
3. Refactor while keeping tests green

Test organization:
- Unit tests: Core logic (fuzzy search, SSH parsing, state management)
- Integration tests: GPUI event simulation
- Performance tests: Benchmark critical paths (< 50ms search response)

### Quality Assurance

The project includes comprehensive QA checks via `nix flake check`:
- **Tests**: All unit tests must pass (`cargo test`)
- **Clippy**: Linting with warnings treated as errors (`cargo clippy -- -D warnings`)
- **Formatting**: Code must be properly formatted (`cargo fmt --check`)
- **Security**: Vulnerability scanning with `cargo audit`
- **Dependencies**: License and dependency validation with `cargo deny` (if deny.toml exists)
- **Build**: Project must compile successfully (`cargo build`)

## Performance Requirements

- Startup time: < 500ms
- Search response: < 50ms for 1000+ hosts
- UI must remain responsive during file parsing

## Key Dependencies

- GPUI: UI framework from Zed
- Fuzzy matching: Custom implementation for performance
- Configuration: TOML parsing for user settings

## Development Workflow

1. Check `docs/implementation-plan.md` for the MVP roadmap
2. Follow the phased implementation approach outlined in docs
3. Use TDD - write tests before implementation
4. Ensure all performance benchmarks pass
5. Test with various terminal applications (iTerm2, Terminal.app, etc.)

### Cross-Platform Development

#### macOS Development
- Use standard `cargo` commands for local development
- Use `nix build` for creating .app bundles
- All development can be done natively

#### Linux Cross-Compilation via Lima

Lima provides lightweight Linux VMs with automatic file sharing, making cross-compilation seamless. This is the recommended approach for Linux development on macOS.

**Quick Start:**
```bash
# Start Lima development VM
nix run .#lima-start

# Build for Linux
nix run .#lima-build

# Run tests on Linux
nix run .#lima-test

# Enter development shell
nix run .#lima-shell

# Stop VM when done
nix run .#lima-stop
```

**Lima VM Details:**
- Configuration: `lima-nix-dev.yaml`
- VM Name: `nix-dev`
- Ubuntu 24.04 ARM64 base image
- Automatic project directory mounting with write permissions
- Determinate Nix pre-installed for fast, reliable package management
- Resource allocation: 4 CPUs, 8GB RAM, 20GB disk

**Development Environment Benefits:**
- **Performance**: Native-like speed using Apple's Virtualization.framework
- **Setup Time**: Fast VM startup (~30s)
- **Resource Usage**: Low overhead compared to traditional VMs
- **File Sharing**: Seamless project directory integration
- **Persistence**: VM state persists between sessions

#### Lima Development Tips

1. **First-time setup**: Initial VM creation downloads Ubuntu 24.04 ARM64 image (~800MB)
2. **File sharing**: Project directory automatically mounted with write permissions
3. **Performance**: Lima uses Apple's Virtualization.framework for near-native speed
4. **Persistence**: VM state and development environment persist between sessions
5. **Direct access**: Use `limactl shell nix-dev` for direct VM shell access
6. **Convenient aliases**: VM includes pre-configured aliases (`tcd`, `tbuild`, `ttest`, etc.)
7. **Nix integration**: Determinate Nix provides faster, more reliable package management

## Important Notes

- This is a menu bar application, not a standalone window
- Global hotkey registration is critical to the UX
- Terminal launching is delegated to user configuration - no hardcoded terminal logic
- Simple SSH config parsing by default (just Host entries, no Include directives)