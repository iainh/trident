// ABOUTME: Cross-platform tray icon implementation using tray-icon crate
// ABOUTME: Provides system tray/menubar integration with event-based handling

use anyhow::Result;
use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

pub struct TridentTray {
    _tray_icon: TrayIcon,
}

// Menu item IDs - created at runtime
fn open_trident_id() -> MenuId {
    MenuId::new("open_trident")
}
fn start_at_login_id() -> MenuId {
    MenuId::new("start_at_login")
}
fn quit_trident_id() -> MenuId {
    MenuId::new("quit_trident")
}

impl TridentTray {
    pub fn new() -> Result<Self> {
        // Create the menu
        let menu = Menu::new();

        // Create menu items
        let open_item = MenuItem::with_id(open_trident_id(), "Open Trident", true, None);
        let separator1 = PredefinedMenuItem::separator();
        let login_item = MenuItem::with_id(start_at_login_id(), "Start at Login", true, None);
        let separator2 = PredefinedMenuItem::separator();
        let quit_item = MenuItem::with_id(quit_trident_id(), "Quit Trident", true, None);

        // Add items to menu
        menu.append(&open_item)?;
        menu.append(&separator1)?;
        menu.append(&login_item)?;
        menu.append(&separator2)?;
        menu.append(&quit_item)?;

        // Load the icon from embedded bytes
        let icon_bytes = include_bytes!("../assets/trident-icon-32.png");
        let icon = image::load_from_memory(icon_bytes)?;
        let icon = icon.to_rgba8();
        let (width, height) = icon.dimensions();

        let icon = tray_icon::Icon::from_rgba(icon.into_raw(), width, height)?;

        // Create the tray icon with template mode for macOS dark mode support
        let mut tray_builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Trident SSH Launcher")
            .with_icon(icon);

        // Enable template mode on macOS for automatic dark mode adaptation
        #[cfg(target_os = "macos")]
        {
            tray_builder = tray_builder.with_icon_as_template(true);
        }

        let tray_icon = tray_builder.build()?;

        println!("[INFO] Created cross-platform tray icon");

        Ok(Self {
            _tray_icon: tray_icon,
        })
    }

    /// Check for tray icon events and return the event type
    pub fn try_recv_tray_event() -> Option<TrayEvent> {
        // Check for tray icon click events
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            println!("[DEBUG] Tray icon event: {event:?}");
            match event {
                TrayIconEvent::Click { .. } => return Some(TrayEvent::Click),
                TrayIconEvent::DoubleClick { .. } => return Some(TrayEvent::DoubleClick),
                _ => {}
            }
        }

        // Check for menu events
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            println!("[DEBUG] Menu event: {event:?}");
            if event.id == open_trident_id() {
                return Some(TrayEvent::OpenTrident);
            } else if event.id == start_at_login_id() {
                return Some(TrayEvent::ToggleStartAtLogin);
            } else if event.id == quit_trident_id() {
                return Some(TrayEvent::Quit);
            }
        }

        None
    }
}

impl Default for TridentTray {
    fn default() -> Self {
        Self::new().expect("Failed to create tray icon")
    }
}

#[derive(Debug, Clone)]
pub enum TrayEvent {
    Click,
    DoubleClick,
    OpenTrident,
    ToggleStartAtLogin,
    Quit,
}
