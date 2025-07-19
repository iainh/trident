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

## Important Notes

- This is a menu bar application, not a standalone window
- Global hotkey registration is critical to the UX
- Terminal launching is delegated to user configuration - no hardcoded terminal logic
- Simple SSH config parsing by default (just Host entries, no Include directives)