use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use super::{
    all_bindings::*,
    util::{direct_composition_supported, to_utf16},
};

use const_cstr::const_cstr;
use utf16_lit::utf16_null;

struct Global {
    window_class: RefCell<Weak<WindowClass>>,
}

unsafe impl Sync for Global {}

lazy_static! {
    static ref GLOBAL: Global = Global {
        window_class: RefCell::new(Weak::new()),
    };
}

struct WindowClass {
    pub class_name: Vec<u16>,
}

impl WindowClass {
    pub fn get() -> Rc<Self> {
        let res = GLOBAL.window_class.borrow().upgrade();
        match res {
            Some(class) => class,
            None => {
                let res = Rc::new(Self::new());
                GLOBAL.window_class.replace(Rc::downgrade(&res));
                res
            }
        }
    }

    fn new() -> Self {
        let mut res = WindowClass {
            class_name: to_utf16("nanoshell_FLUTTER_WINDOW"),
        };
        res.register();
        res
    }

    fn register(&mut self) {
        unsafe {
            let class = WNDCLASSW {
                style: (CS_HREDRAW | CS_VREDRAW) as u32,
                // style: (0) as u32,
                lpfn_wnd_proc: Some(wnd_proc),
                cb_cls_extra: 0,
                cb_wnd_extra: 0,
                h_instance: HINSTANCE(GetModuleHandleW(std::ptr::null_mut())),
                h_icon: Default::default(),
                h_cursor: LoadCursorW(HINSTANCE(0), IDC_ARROW as *const u16),
                hbr_background: HBRUSH(0),
                lpsz_menu_name: std::ptr::null_mut(),
                lpsz_class_name: self.class_name.as_mut_ptr(),
            };
            RegisterClassW(&class);
        }
    }

    fn unregister(&mut self) {
        unsafe {
            UnregisterClassW(self.class_name.as_ptr(), HINSTANCE(0));
        }
    }
}

impl Drop for WindowClass {
    fn drop(&mut self) {
        self.unregister();
    }
}

// Adapter for handling window message in rust object
pub trait WindowAdapter {
    fn wnd_proc(&self, h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT;

    fn default_wnd_proc(&self, h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        unsafe {
            return DefWindowProcW(h_wnd, msg, w_param, l_param);
        }
    }

    fn create_window(&self, title: &str) -> HWND
    where
        Self: Sized,
    {
        let mut ex_flags = WS_EX_APPWINDOW;
        if direct_composition_supported() {
            ex_flags |= WS_EX_NOREDIRECTIONBITMAP;
        }

        self.create_window_custom(
            title,
            (WS_OVERLAPPEDWINDOW | WS_THICKFRAME | WS_SYSMENU | WS_DLGFRAME) as u32,
            ex_flags as u32,
        )
    }

    fn create_window_custom(&self, title: &str, style: u32, ex_style: u32) -> HWND
    where
        Self: Sized,
    {
        let title = to_utf16(title);
        unsafe {
            let s = self as &dyn WindowAdapter;
            let class = WindowClass::get();
            let ptr = std::mem::transmute(s);
            let bridge = Box::new(EventBridge {
                handler: ptr,
                _class: class.clone(),
            });

            let res = CreateWindowExW(
                ex_style,
                class.class_name.as_ptr(),
                title.as_ptr(),
                style,
                100,
                100,
                200,
                200,
                HWND(0),
                HMENU(0),
                HINSTANCE(GetModuleHandleW(std::ptr::null_mut())),
                Box::into_raw(bridge) as *mut _,
            );
            res
        }
    }
}

struct EventBridge {
    handler: *const dyn WindowAdapter,
    _class: Rc<WindowClass>, // keep class alive
}

// Missing from metadata for now
#[link(name = "USER32")]
extern "system" {
    pub fn SetWindowLongPtrW(h_wnd: HWND, n_index: i32, dw_new_long: isize) -> isize;
    pub fn GetWindowLongPtrW(h_wnd: HWND, n_index: i32) -> isize;
}

extern "system" fn wnd_proc(h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    unsafe {
        match msg as i32 {
            WM_NCCREATE => {
                let create_struct = &*(l_param.0 as *const CREATESTRUCTW);
                SetWindowLongPtrW(
                    h_wnd,
                    GWLP_USERDATA,
                    create_struct.lp_create_params as isize,
                );
                enable_full_dpi_support(h_wnd);
            }
            _ => {}
        }

        let ptr = GetWindowLongPtrW(h_wnd, GWLP_USERDATA);
        if ptr != 0 {
            let bridge = &*(ptr as *const EventBridge);
            let handler = &*(bridge.handler);
            let res = handler.wnd_proc(h_wnd, msg, w_param, l_param);
            if msg == WM_NCDESTROY as u32 {
                // make sure bridge is dropped
                Box::<EventBridge>::from_raw(ptr as *mut EventBridge);
            }
            return res;
        }

        DefWindowProcW(h_wnd, msg, w_param, l_param)
    }
}

pub fn enable_full_dpi_support(hwnd: HWND) {
    unsafe {
        let module = LoadLibraryW(utf16_null!("User32.dll").as_ptr());
        if module == 0 {
            return;
        }
        let enable = GetProcAddress(module, const_cstr!("EnableNonClientDpiScaling").as_ptr());
        if let Some(enable) = enable {
            let fnn: extern "system" fn(HWND) -> ::windows::BOOL = std::mem::transmute(enable);
            fnn(hwnd);
        }

        FreeLibrary(module);
    }
}
