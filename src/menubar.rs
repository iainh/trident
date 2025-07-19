// ABOUTME: Native macOS menubar integration using objc2 and NSStatusItem
// ABOUTME: Provides proper system menubar icon with automatic dark mode support

#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2::{ClassType, DeclaredClass, declare_class, msg_send, msg_send_id, mutability};
#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSApplication, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
    NSVariableStatusItemLength,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{MainThreadMarker, NSBundle, NSData, NSObject, NSObjectProtocol, NSString};
use std::sync::{Arc, Mutex};

// For PNG image loading and processing
extern crate image;

type CallbackFn = Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>;

pub struct TridentMenuBar {
    #[cfg(target_os = "macos")]
    status_item: Option<objc2::rc::Retained<NSStatusItem>>,
    callback: CallbackFn,
}

#[cfg(target_os = "macos")]
static GLOBAL_CALLBACK: std::sync::Mutex<Option<Arc<dyn Fn() + Send + Sync>>> =
    std::sync::Mutex::new(None);

#[cfg(target_os = "macos")]
declare_class!(
    struct MenuBarDelegate;

    unsafe impl ClassType for MenuBarDelegate {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "MenuBarDelegate";
    }

    impl DeclaredClass for MenuBarDelegate {}

    unsafe impl NSObjectProtocol for MenuBarDelegate {}


    unsafe impl MenuBarDelegate {
        #[method(openTrident:)]
        fn open_trident(&self, _sender: Option<&AnyObject>) {
            println!("[DEBUG] Menu item 'Open Trident' clicked");
            if let Ok(callback_guard) = GLOBAL_CALLBACK.lock() {
                if let Some(ref callback) = *callback_guard {
                    callback();
                }
            }
        }

        #[method(toggleStartAtLogin:)]
        fn toggle_start_at_login(&self, sender: Option<&AnyObject>) {
            println!("[DEBUG] Menu item 'Start at Login' clicked");
            if let Some(menu_item) = sender {
                unsafe {
                    let current_state: bool = msg_send![menu_item, state];
                    let new_state = !current_state;

                    if new_state {
                        Self::add_to_login_items();
                    } else {
                        Self::remove_from_login_items();
                    }

                    let _: () = msg_send![menu_item, setState: new_state as i64];
                }
            }
        }

        #[method(quitTrident:)]
        fn quit_trident(&self, _sender: Option<&AnyObject>) {
            println!("[DEBUG] Menu item 'Quit Trident' clicked");
            unsafe {
                let app = NSApplication::sharedApplication(MainThreadMarker::new_unchecked());
                app.terminate(None);
            }
        }
    }

);

#[cfg(target_os = "macos")]
impl MenuBarDelegate {
    fn add_to_login_items() {
        println!("[INFO] Adding Trident to login items...");
        match Self::call_osascript_add_login_item() {
            Ok(_) => println!("[INFO] Successfully added Trident to login items"),
            Err(e) => println!("[WARN] Failed to add to login items: {e}"),
        }
    }

    fn remove_from_login_items() {
        println!("[INFO] Removing Trident from login items...");
        match Self::call_osascript_remove_login_item() {
            Ok(_) => println!("[INFO] Successfully removed Trident from login items"),
            Err(e) => println!("[WARN] Failed to remove from login items: {e}"),
        }
    }

    fn is_login_item() -> bool {
        // For simplicity, just return false for now
        // In a full implementation, we'd check the actual login items
        false
    }

    fn call_osascript_add_login_item() -> Result<(), String> {
        use std::process::Command;

        let bundle_path = Self::get_bundle_path().ok_or("Could not get bundle path")?;

        let script = format!(
            r#"tell application "System Events"
                make login item at end with properties {{path:"{bundle_path}", hidden:false}}
            end tell"#
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| format!("Failed to execute osascript: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("osascript failed: {stderr}"))
        }
    }

    fn call_osascript_remove_login_item() -> Result<(), String> {
        use std::process::Command;

        let script = r#"tell application "System Events"
            delete login item "Trident"
        end tell"#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| format!("Failed to execute osascript: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("osascript failed: {stderr}"))
        }
    }

    fn get_bundle_path() -> Option<String> {
        unsafe {
            let bundle: objc2::rc::Retained<NSBundle> = msg_send_id![NSBundle::class(), mainBundle];
            let bundle_path: Option<objc2::rc::Retained<NSString>> =
                msg_send_id![&bundle, bundlePath];

            bundle_path.map(|path| path.to_string())
        }
    }
}

impl TridentMenuBar {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "macos")]
            status_item: None,
            callback: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_click_callback<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let callback_arc = Arc::new(callback);
        let callback_clone = callback_arc.clone();
        *self.callback.lock().unwrap() = Some(Box::new(move || callback_clone()));

        // Also set the global callback for the delegate
        #[cfg(target_os = "macos")]
        {
            if let Ok(mut global_callback) = GLOBAL_CALLBACK.lock() {
                *global_callback = Some(callback_arc);
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn create_status_item(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();

            // Get the system status bar
            let status_bar = NSStatusBar::systemStatusBar();

            // Create status item with variable length
            let status_item = status_bar.statusItemWithLength(NSVariableStatusItemLength);

            // Create the trident icon as an NSImage template
            let icon_image = self.create_template_icon(mtm)?;

            // Set the icon on the status item button
            let button: objc2::rc::Retained<NSObject> = msg_send_id![&status_item, button];
            let _: () = msg_send![&button, setImage: &*icon_image];
            let _: () =
                msg_send![&button, setToolTip: &*NSString::from_str("Trident SSH Launcher")];

            // Create the menu delegate
            let delegate: objc2::rc::Retained<MenuBarDelegate> =
                msg_send_id![MenuBarDelegate::alloc(), init];

            // Create the context menu
            let menu = NSMenu::new(mtm);
            menu.setAutoenablesItems(false);

            // Create "Open Trident" menu item
            let open_item = NSMenuItem::new(mtm);
            open_item.setTitle(&NSString::from_str("Open Trident"));
            open_item.setTarget(Some(&*delegate));
            open_item.setAction(Some(objc2::sel!(openTrident:)));
            open_item.setEnabled(true);
            menu.addItem(&open_item);

            // Add separator
            let separator1 = NSMenuItem::separatorItem(mtm);
            menu.addItem(&separator1);

            // Create "Start at Login" menu item with checkbox
            let login_item = NSMenuItem::new(mtm);
            login_item.setTitle(&NSString::from_str("Start at Login"));
            login_item.setTarget(Some(&*delegate));
            login_item.setAction(Some(objc2::sel!(toggleStartAtLogin:)));
            login_item.setEnabled(true);

            // Set initial checkbox state based on current login item status
            let is_login_item = MenuBarDelegate::is_login_item();
            let _: () = msg_send![&login_item, setState: is_login_item as i64];

            menu.addItem(&login_item);

            // Add separator
            let separator2 = NSMenuItem::separatorItem(mtm);
            menu.addItem(&separator2);

            // Create "Quit Trident" menu item
            let quit_item = NSMenuItem::new(mtm);
            quit_item.setTitle(&NSString::from_str("Quit Trident"));
            quit_item.setTarget(Some(&*delegate));
            quit_item.setAction(Some(objc2::sel!(quitTrident:)));
            quit_item.setEnabled(true);
            menu.addItem(&quit_item);

            // Set the menu on the status item
            status_item.setMenu(Some(&menu));

            // Store the status item and delegate to keep them alive
            self.status_item = Some(status_item);

            // Keep the delegate alive by storing it in a static
            // This is a bit of a hack but necessary to prevent deallocation
            std::mem::forget(delegate);

            println!("[INFO] Created native macOS menubar item with NSStatusItem");
            Ok(())
        }
    }

    #[cfg(target_os = "macos")]
    fn create_template_icon(
        &self,
        _mtm: MainThreadMarker,
    ) -> Result<objc2::rc::Retained<NSImage>, Box<dyn std::error::Error>> {
        unsafe {
            // Load the PNG icon from embedded bytes
            let png_bytes = include_bytes!("../assets/trident-icon-32.png");

            // Create NSData from the PNG bytes
            let ns_data = NSData::with_bytes(png_bytes);

            // Create NSImage from the data
            let ns_image = NSImage::initWithData(NSImage::alloc(), &ns_data)
                .ok_or("Failed to create NSImage from PNG data")?;

            // Set the image as a template image for automatic dark mode support
            ns_image.setTemplate(true);

            // Set the size to 16x16 for menubar
            let size = objc2_foundation::NSSize {
                width: 16.0,
                height: 16.0,
            };
            ns_image.setSize(size);

            Ok(ns_image)
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn create_status_item(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[INFO] Native menubar only supported on macOS");
        Ok(())
    }

    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    pub fn run_event_loop(self) {
        // No need to run NSApplication.run() - the menubar item is already created
        // and will respond to clicks. The main application event loop handles everything.
        println!("[INFO] Native menubar event handling integrated with main app");

        // Keep the menubar alive by moving it into a static
        std::mem::forget(self);
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
