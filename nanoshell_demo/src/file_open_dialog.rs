use std::{
    cell::RefCell,
    rc::{Rc, Weak},
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
use objc::msg_send;
use objc::rc::{autoreleasepool, StrongPtr};

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
}

impl Drop for FileOpenDialogService {
    fn drop(&mut self) {
        self.context
            .message_manager
            .borrow_mut()
            .unregister_message_handler("file_open_dialog_channel");
    }
}
