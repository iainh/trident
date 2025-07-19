// ABOUTME: Native macOS menubar integration using objc2 and NSStatusItem
// ABOUTME: Provides proper system menubar icon with automatic dark mode support

#[cfg(target_os = "macos")]
use objc2::runtime::{AnyObject, ProtocolObject};
#[cfg(target_os = "macos")]
use objc2::{declare_class, msg_send, msg_send_id, mutability, ClassType, DeclaredClass};
#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSApplication, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
    NSVariableStatusItemLength,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSData, NSObject, NSObjectProtocol, NSString, MainThreadMarker};
use std::sync::{Arc, Mutex};

// For PNG image loading and processing
extern crate image;

pub struct TridentMenuBar {
    #[cfg(target_os = "macos")]
    status_item: Option<objc2::rc::Retained<NSStatusItem>>,
    callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
}

#[cfg(target_os = "macos")]
static GLOBAL_CALLBACK: std::sync::Mutex<Option<Arc<dyn Fn() + Send + Sync>>> = std::sync::Mutex::new(None);

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
            let _: () = msg_send![&button, setToolTip: &*NSString::from_str("Trident SSH Launcher")];
            
            // Create the menu delegate
            let delegate: objc2::rc::Retained<MenuBarDelegate> = msg_send_id![MenuBarDelegate::alloc(), init];
            
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
            let separator = NSMenuItem::separatorItem(mtm);
            menu.addItem(&separator);
            
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
    fn create_template_icon(&self, mtm: MainThreadMarker) -> Result<objc2::rc::Retained<NSImage>, Box<dyn std::error::Error>> {
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
            let size = objc2_foundation::NSSize { width: 16.0, height: 16.0 };
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