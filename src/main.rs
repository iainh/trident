#![recursion_limit = "512"]

mod app;
mod config;
mod fuzzy;
mod objc2_hotkey;
mod platform;
mod ssh;
mod tray;
mod ui;

use anyhow::Result;
use app::AppState;
use config::{Config};
use gpui::*;
use platform::Platform;
use ssh::{HostEntry, TerminalLauncher, parse_known_hosts, parse_ssh_config};
use std::path::Path;
use ui::{HostList, SearchInput};
use tracing_subscriber::FmtSubscriber;
use tracing::{info, warn, error, Level};

actions!(trident, [ShowLauncher, QuitApp, ToggleLauncher]);

struct ZedTheme;

#[allow(dead_code)]
impl ZedTheme {
    fn elevated_surface_background() -> Hsla {
        rgb(0x282c34).into()
    }

    fn surface_background() -> Hsla {
        rgb(0x252930).into()
    }

    fn editor_background() -> Hsla {
        rgb(0x252930).into()
    }

    fn border() -> Hsla {
        rgb(0x3c4043).into()
    }

    fn text() -> Hsla {
        rgb(0xd4d4d4).into()
    }

    fn text_placeholder() -> Hsla {
        rgb(0x8c8c8c).into()
    }

    fn text_muted() -> Hsla {
        rgb(0xa5a5a5).into()
    }

    fn text_accent() -> Hsla {
        rgb(0x569cd6).into()
    }

    fn ghost_element_hover() -> Hsla {
        rgb(0x454a55).into()
    }

    fn ghost_element_selected() -> Hsla {
        hsla(207.0 / 360.0, 0.7, 0.25, 0.2)
    }

    fn cursor() -> Hsla {
        rgb(0xd4d4d4).into()
    }
}

struct TridentApp {
    state: AppState,
    search_input: SearchInput,
    host_list: HostList,
    terminal_launcher: TerminalLauncher,
    focus_handle: FocusHandle,
}

impl TridentApp {
    #[cfg(not(test))]
    fn new(cx: &mut Context<Self>) -> Self {
        let mut config = Self::load_config().unwrap_or_else(|e| {
            error!("Failed to load config: {}. Using defaults.", e);
            Config::default()
        });

        if let Err(e) = config.expand_path() {
            error!("Failed to expand config paths: {}. Using defaults.", e);
            config = Config::default();
        }

        if let Err(e) = config.validate() {
            error!("Invalid configuration: {}. Using defaults.", e);
            config = Config::default();
        }

        let mut state = AppState::new();
        state.config = config.clone();

        let hosts = Self::load_ssh_hosts(&config);
        state.hosts = hosts.clone();
        state.filtered_hosts = hosts.clone();

        let mut search_input = SearchInput::new("Search SSH hosts...".to_string());
        search_input.set_focused(true);

        let terminal_launcher = TerminalLauncher::new(config.terminal.clone());

        Self {
            state,
            search_input,
            host_list: HostList::new(hosts),
            terminal_launcher,
            focus_handle: cx.focus_handle(),
        }
    }

    #[cfg(test)]
    fn new(cx: &mut Context<Self>) -> Self {
        use config::{ParsingConfig, SshConfig, TerminalConfig, UiConfig};

        let config = Config {
            terminal: TerminalConfig {
                program: "/bin/echo".to_string(),
                args: vec!["test".to_string()],
                strategy: config::LaunchStrategy::ShellCommand,
            },
            ssh: SshConfig {
                known_hosts_path: "/tmp/test_known_hosts".to_string(),
                config_path: "/tmp/test_config".to_string(),
                ssh_binary: "/usr/bin/ssh".to_string(),
            },
            parsing: ParsingConfig {
                parse_known_hosts: false,
                parse_ssh_config: false,
                simple_config_parsing: true,
                skip_hashed_hosts: true,
            },
            ui: UiConfig {
                max_results: 10,
                case_sensitive: false,
            },
            hotkey: HotkeyConfig::default(),
        };

        let mut state = AppState::new();
        state.config = config.clone();

        let search_input = SearchInput::new("Test search...".to_string());
        let terminal_launcher = TerminalLauncher::new(config.terminal.clone());

        Self {
            state,
            search_input,
            host_list: HostList::new(Vec::new()),
            terminal_launcher,
            focus_handle: cx.focus_handle(),
        }
    }

    fn load_config() -> Result<Config> {
        let config_path = Config::default_config_path()?;

        if !config_path.exists() {
            Config::save_generated_config(&config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create configuration file: {}", e))?;
            info!("Created configuration with auto-detected terminal at: {}", config_path.display());
        }

        Config::load_from_file(&config_path)
    }

    fn load_ssh_hosts(config: &Config) -> Vec<HostEntry> {
        let mut all_hosts = Vec::new();

        if config.parsing.parse_known_hosts {
            let known_hosts_path = Path::new(&config.ssh.known_hosts_path);
            if !known_hosts_path.exists() {
                warn!("known_hosts file '{}' not found. Skipping known_hosts parsing.", config.ssh.known_hosts_path);
            } else {
                match parse_known_hosts(known_hosts_path, config.parsing.skip_hashed_hosts) {
                    Ok(hosts) => all_hosts.extend(hosts),
                    Err(e) => error!("Failed to parse known_hosts: {}", e),
                }
            }
        }

        if config.parsing.parse_ssh_config {
            let ssh_config_path = Path::new(&config.ssh.config_path);
            if !ssh_config_path.exists() {
                warn!("SSH config file '{}' not found. Skipping SSH config parsing.", config.ssh.config_path);
            } else {
                match parse_ssh_config(ssh_config_path, config.parsing.simple_config_parsing) {
                    Ok(hosts) => all_hosts.extend(hosts),
                    Err(e) => error!("Failed to parse SSH config: {}", e),
                }
            }
        }

        all_hosts.sort_by(|a, b| a.name.cmp(&b.name));
        all_hosts.dedup_by(|a, b| a.name == b.name);

        if all_hosts.is_empty() {
            warn!("No SSH hosts found, using examples");
            vec![
                HostEntry::new("example.com".to_string(), "ssh user@example.com".to_string()),
            ]
        } else {
            all_hosts
        }
    }

    fn handle_key_event(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "up" => {
                self.host_list.select_previous();
                cx.notify();
            }
            "down" => {
                self.host_list.select_next();
                cx.notify();
            }
            "enter" => {
                if let Some(host) = self.host_list.get_selected_host() {
                    if let Err(e) = self.launch_host(host) {
                        error!("Failed to launch host: {}", e);
                    }
                    self.close_launcher_window(window, cx);
                }
            }
            "escape" => {
                self.close_launcher_window(window, cx);
            }
            _ => {
                if let Some(text) = &event.keystroke.key_char {
                    self.search_input.handle_input(text);
                    self.update_search();
                    cx.notify();
                }
            }
        }
    }

    fn update_search(&mut self) {
        self.state.search_query = self.search_input.query.clone();
        let search_engine = fuzzy::SearchEngine::new(self.state.hosts.clone());
        let results = search_engine.search(
            &self.state.search_query,
            self.state.config.ui.case_sensitive,
            self.state.config.ui.max_results,
        );
        let filtered_hosts: Vec<HostEntry> = results.into_iter().cloned().collect();
        self.host_list.set_hosts(filtered_hosts);
    }

    fn launch_host(&self, host: &HostEntry) -> Result<()> {
        self.terminal_launcher.launch(host)
    }

    fn close_launcher_window(&self, _window: &mut Window, cx: &mut Context<Self>) {
        cx.hide();
        cx.update_global::<TridentState, ()>(|state, _| {
            state.launcher_window = None;
        });
    }
}

impl Render for TridentApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus_handle);
        div()
            .flex()
            .items_start()
            .justify_center()
            .w_full()
            .h_full()
            .pt(px(360.0))
            .track_focus(&self.focus_handle)
            .on_key_down(
                cx.listener(|this, event: &KeyDownEvent, window: &mut Window, cx| {
                    this.handle_key_event(event, window, cx);
                }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(600.0))
                    .max_h(px(500.0))
                    .bg(ZedTheme::elevated_surface_background().alpha(0.75))
                    .border_1()
                    .border_color(hsla(0.0, 0.0, 1.0, 0.15))
                    .rounded_lg()
                    .overflow_hidden()
                    .shadow(vec![BoxShadow {
                        color: hsla(0.0, 0.0, 0.0, 0.3),
                        offset: Point::new(px(0.0), px(8.0)),
                        blur_radius: px(24.0),
                        spread_radius: px(0.0),
                    }])
                    .p(px(4.0))
                    .child(self.search_input.clone())
                    .child(self.host_list.clone()),
            )
    }
}

#[cfg(not(test))]
fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Trident SSH Launcher...");
    run_menubar_app()
}

use std::sync::atomic::{AtomicBool, Ordering};
static GLOBAL_HOTKEY_TRIGGERED: AtomicBool = AtomicBool::new(false);

#[cfg(not(test))]
fn run_menubar_app() -> Result<()> {
    Application::new().run(|cx: &mut App| {
        let _config = TridentApp::load_config().unwrap_or_default();

        cx.set_global(TridentState { launcher_window: None });

        cx.observe_global::<TridentState>(move |cx| {
            if cx.global::<TridentState>().launcher_window.is_none() && GLOBAL_HOTKEY_TRIGGERED.load(Ordering::SeqCst) {
                GLOBAL_HOTKEY_TRIGGERED.store(false, Ordering::SeqCst);
                show_launcher_window(cx);
            }
        }).detach();

        let mut hotkey_manager = Platform::hotkey_manager();
        let hotkey_callback = move || {
            GLOBAL_HOTKEY_TRIGGERED.store(true, Ordering::SeqCst);
        };

        if let Err(e) = hotkey_manager.register_hotkey(Box::new(hotkey_callback)) {
            error!("Failed to register global hotkey: {}", e);
        }
        std::mem::forget(hotkey_manager);

        let _tray = tray::TridentTray::new().expect("Failed to create tray icon");
        std::mem::forget(_tray);

        cx.activate(false);
    });

    Ok(())
}

#[derive(Default)]
struct TridentState {
    launcher_window: Option<AnyWindowHandle>,
}
impl Global for TridentState {}

#[cfg(not(test))]
fn show_launcher_window(cx: &mut App) {
    if cx.global::<TridentState>().launcher_window.is_some() {
        return;
    }

    let display_bounds = cx.primary_display().map_or(Bounds::default(), |d| d.bounds());
    let window = cx.open_window(
        WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Fullscreen(display_bounds)),
            is_movable: false,
            kind: WindowKind::PopUp,
            window_background: WindowBackgroundAppearance::Transparent,
            ..Default::default()
        },
        |_, cx| cx.new(TridentApp::new),
    );

    if let Ok(handle) = window {
        cx.update_global::<TridentState, ()>(|state, _| {
            state.launcher_window = Some(handle.into());
        });
    }
}

#[cfg(test)]
fn main() -> Result<()> {
    Ok(())
}
