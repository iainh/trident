// ABOUTME: Native macOS menubar integration using tao system tray
// ABOUTME: Provides proper system menubar icon and menu functionality

#[cfg(target_os = "macos")]
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    menu::{ContextMenu, MenuItemAttributes, MenuId},
    system_tray::{SystemTray, SystemTrayBuilder, Icon},
};

use std::sync::{Arc, Mutex};

pub struct TridentMenuBar {
    #[cfg(target_os = "macos")]
    event_loop: Option<EventLoop<()>>,
    #[cfg(target_os = "macos")]
    system_tray: Option<SystemTray>,
    callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
}

impl TridentMenuBar {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "macos")]
            event_loop: None,
            #[cfg(target_os = "macos")]
            system_tray: None,
            callback: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_click_callback<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self.callback.lock().unwrap() = Some(Box::new(callback));
    }

    #[cfg(target_os = "macos")]
    pub fn create_status_item(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new();
        
        // Create context menu for the system tray
        let mut tray_menu = ContextMenu::new();
        tray_menu.add_item(MenuItemAttributes::new("Open Trident").with_id(MenuId(1)));
        tray_menu.add_native_item(tao::menu::MenuItem::Separator);
        tray_menu.add_item(MenuItemAttributes::new("Quit Trident").with_id(MenuId(2)));

        // Create system tray with trident icon (ψ symbol)
        let icon_rgba = create_trident_icon();
        let icon = Icon::from_rgba(icon_rgba, 16, 16)?;
        let system_tray = SystemTrayBuilder::new(icon, Some(tray_menu))
            .build(&event_loop)?;

        self.system_tray = Some(system_tray);
        self.event_loop = Some(event_loop);
        
        println!("[INFO] Created native macOS menubar item");
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn create_status_item(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[INFO] Native menubar only supported on macOS");
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub fn run_event_loop(self) {
        if let Some(event_loop) = self.event_loop {
            let callback = self.callback.clone();
            
            event_loop.run(move |event, _, control_flow| {
                *control_flow = ControlFlow::Wait;

                match event {
                    Event::MenuEvent {
                        menu_id,
                        origin: tao::menu::MenuType::ContextMenu,
                        ..
                    } => {
                        match menu_id.0 {
                            1 => {
                                // Open Trident
                                if let Ok(callback_guard) = callback.lock() {
                                    if let Some(ref cb) = *callback_guard {
                                        cb();
                                    }
                                }
                            }
                            2 => {
                                // Quit
                                *control_flow = ControlFlow::Exit;
                            }
                            _ => {}
                        }
                    }
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            });
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn run_event_loop(self) {
        println!("[INFO] Event loop not needed on non-macOS platforms");
        std::thread::park();
    }
}

impl Default for TridentMenuBar {
    fn default() -> Self {
        Self::new()
    }
}

// Create a simple trident icon that adapts to system appearance
fn create_trident_icon() -> Vec<u8> {
    let mut icon_data = vec![0u8; 16 * 16 * 4]; // 16x16 RGBA
    
    // Use system appearance to determine icon color
    let is_dark_mode = is_dark_mode();
    let (icon_r, icon_g, icon_b) = if is_dark_mode {
        (255, 255, 255) // White for dark mode
    } else {
        (0, 0, 0) // Black for light mode
    };
    
    // Simple trident pattern - draw vertical line and three prongs at top
    for y in 0..16 {
        for x in 0..16 {
            let idx = (y * 16 + x) * 4;
            let (r, g, b, a) = if should_draw_trident_pixel(x, y) {
                (icon_r, icon_g, icon_b, 255) // Colored pixel
            } else {
                (0, 0, 0, 0) // Transparent pixel
            };
            
            icon_data[idx] = r;
            icon_data[idx + 1] = g;
            icon_data[idx + 2] = b;
            icon_data[idx + 3] = a;
        }
    }
    
    icon_data
}

#[cfg(target_os = "macos")]
fn is_dark_mode() -> bool {
    use std::process::Command;
    
    // Use macOS defaults command to check system appearance
    let output = Command::new("defaults")
        .args(&["read", "-g", "AppleInterfaceStyle"])
        .output();
    
    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            stdout.trim() == "Dark"
        }
        Err(_) => false, // Default to light mode if we can't determine
    }
}

#[cfg(not(target_os = "macos"))]
fn is_dark_mode() -> bool {
    false // Default to light mode on non-macOS platforms
}

fn should_draw_trident_pixel(x: usize, y: usize) -> bool {
    // Center vertical line
    if x == 8 && y >= 4 {
        return true;
    }
    
    // Left prong
    if y >= 1 && y <= 3 && x == 6 {
        return true;
    }
    
    // Center prong  
    if y >= 1 && y <= 5 && x == 8 {
        return true;
    }
    
    // Right prong
    if y >= 1 && y <= 3 && x == 10 {
        return true;
    }
    
    // Top connections
    if y == 1 && (x == 7 || x == 9) {
        return true;
    }
    
    false
}