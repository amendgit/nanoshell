pub mod binary_messenger;
pub mod display;
pub mod dpi;
pub mod drag_com;
pub mod drag_context;
pub mod drag_data;
pub mod drag_util;
pub mod dxgi_hook;
pub mod engine;
pub mod error;
pub mod flutter_api;
pub mod init;
pub mod menu;
pub mod run_loop;
pub mod util;
pub mod window;
pub mod window_adapter;
pub mod window_base;
pub mod window_menu;

#[allow(dead_code)]
mod bindings {
    ::windows::include_bindings!();
}

// This bit of a lie, it doesn't have dxgi
mod all_bindings {
    pub use super::bindings::{
        windows::win32::com::*, windows::win32::controls::*, windows::win32::data_exchange::*,
        windows::win32::debug::*, windows::win32::direct_show::*,
        windows::win32::display_devices::*, windows::win32::dwm::*, windows::win32::gdi::*,
        windows::win32::keyboard_and_mouse_input::*, windows::win32::menus_and_resources::*,
        windows::win32::shell::*, windows::win32::structured_storage::*,
        windows::win32::system_services::*, windows::win32::windows_and_messaging::*,
    };
    pub use windows::*;
}
