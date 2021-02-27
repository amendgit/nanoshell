use std::{
    cell::RefCell,
    mem::size_of,
    ptr::null_mut,
    rc::{Rc, Weak},
    time::Duration,
};

use nanoshell::{
    codec::{value::from_value, MethodCall, MethodCallReply, Value},
    shell::{Context, WindowHandle},
};

#[cfg(target_os = "macos")]
use block::ConcreteBlock;

#[cfg(target_os = "macos")]
use cocoa::{
    base::id,
    foundation::{NSArray, NSString, NSUInteger},
};

#[cfg(target_os = "macos")]
use objc::{
    msg_send,
    rc::{autoreleasepool, StrongPtr},
};

#[cfg(target_os = "windows")]
mod win_imports {
    mod bindings {
        ::windows::include_bindings!();
    }

    pub use bindings::windows::win32::{system_services::*, windows_and_messaging::*};
    pub use windows::TRUE;
}

use widestring::WideStr;
#[cfg(target_os = "windows")]
use win_imports::*;

pub struct FileOpenDialogService {
    context: Rc<Context>,
    weak_self: RefCell<Weak<FileOpenDialogService>>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct FileOpenRequest {
    parent_window: WindowHandle,
}

impl FileOpenDialogService {
    pub fn new(context: Rc<Context>) -> Rc<Self> {
        let res = Rc::new(Self {
            context: context.clone(),
            weak_self: RefCell::new(Default::default()),
        });
        *res.weak_self.borrow_mut() = Rc::downgrade(&res);
        res.initialize();
        res
    }

    fn initialize(&self) {
        let weak_self = self.weak_self.borrow().clone();
        self.context
            .message_manager
            .borrow_mut()
            .register_method_handler(
                "file_open_dialog_channel", //
                move |call, reply, _engine| {
                    if let Some(s) = weak_self.upgrade() {
                        s.on_method_call(call, reply);
                    }
                },
            );
    }

    fn on_method_call(&self, call: MethodCall<Value>, reply: MethodCallReply<Value>) {
        match call.method.as_str() {
            "showFileOpenDialog" => {
                let request: FileOpenRequest = from_value(&call.args).unwrap();
                self.open_file_dialog(request, reply);
            }
            _ => {
                reply.send_error("invalid_method", Some("Invalid method"), Value::Null);
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn open_file_dialog(&self, request: FileOpenRequest, reply: MethodCallReply<Value>) {
        let win = self
            .context
            .window_manager
            .borrow()
            .get_platform_window(request.parent_window);

        if let Some(win) = win {
            autoreleasepool(|| unsafe {
                let panel = StrongPtr::retain(msg_send![class!(NSOpenPanel), openPanel]);

                // We know that the callback will be called only once, but rust doesn't;
                let reply = RefCell::new(Some(reply));

                let panel_copy = panel.clone();
                let cb = move |response: NSUInteger| {
                    let reply = reply.take();
                    if let Some(reply) = reply {
                        if response == 1 {
                            let urls: id = msg_send![*panel_copy, URLs];
                            if NSArray::count(urls) > 0 {
                                let url = NSArray::objectAtIndex(urls, 0);
                                let string: id = msg_send![url, absoluteString];
                                let path = Self::from_nsstring(string);
                                reply.send_ok(Value::String(path));
                                return;
                            }
                        }
                        reply.send_ok(Value::Null);
                    }
                };

                let handler = ConcreteBlock::new(cb).copy();
                let () =
                    msg_send![*panel, beginSheetModalForWindow: win completionHandler:&*handler];
            });
        } else {
            reply.send_error("no_window", Some("Platform window not found"), Value::Null);
        }
    }

    #[cfg(target_os = "macos")]
    fn from_nsstring(ns_string: id) -> String {
        use std::os::raw::c_char;
        use std::slice;
        unsafe {
            let bytes: *const c_char = msg_send![ns_string, UTF8String];
            let bytes = bytes as *const u8;
            let len = NSString::len(ns_string);
            let bytes = slice::from_raw_parts(bytes, len);
            std::str::from_utf8(bytes).unwrap().into()
        }
    }

    #[cfg(target_os = "windows")]
    fn open_file_dialog(&self, request: FileOpenRequest, reply: MethodCallReply<Value>) {
        let win = self
            .context
            .window_manager
            .borrow()
            .get_platform_window(request.parent_window);

        if let Some(win) = win {
            let cb = move || {
                let mut file = Vec::<u16>::new();
                file.resize(4096, 0);

                let mut ofn = OPENFILENAMEW {
                    l_struct_size: size_of::<OPENFILENAMEW>() as u32,
                    hwnd_owner: HWND(win.0),
                    h_instance: HINSTANCE(0),
                    lpstr_filter: null_mut(),
                    lpstr_custom_filter: null_mut(),
                    n_max_cust_filter: 0,
                    n_filter_index: 0,
                    lpstr_file: file.as_mut_ptr(),
                    n_max_file: file.len() as u32,
                    lpstr_file_title: null_mut(),
                    n_max_file_title: 0,
                    lpstr_initial_dir: null_mut(),
                    lpstr_title: null_mut(),
                    flags: 0,
                    n_file_offset: 0,
                    n_file_extension: 0,
                    lpstr_def_ext: null_mut(),
                    l_cust_data: LPARAM(0),
                    lpfn_hook: None,
                    lp_template_name: null_mut(),
                    pv_reserved: null_mut(),
                    dw_reserved: 0,
                    flags_ex: 0,
                };

                let res = unsafe { GetOpenFileNameW(&mut ofn as *mut _) == TRUE };
                if !res {
                    reply.send_ok(Value::Null);
                } else {
                    let name = WideStr::from_slice(&file).to_string_lossy();
                    reply.send_ok(Value::String(name));
                }
            };
            self.context
                .run_loop
                .borrow()
                .schedule(cb, Duration::from_secs(0))
                .detach();
        } else {
            reply.send_error("no_window", Some("Platform window not found"), Value::Null);
        }
    }
}

impl Drop for FileOpenDialogService {
    fn drop(&mut self) {
        self.context
            .message_manager
            .borrow_mut()
            .unregister_message_handler("file_open_dialog_channel");
    }
}
