use std::{
    cell::Cell,
    ffi::{c_void, CStr, CString},
};

use log::warn;

type SendPlatformMessage = extern "C" fn(usize, &Message) -> usize;

struct Message {
    size: isize,
    channel: *const i8,
    message: *const u8,
    message_size: isize,
    response_handle: isize,
}

extern "C" fn send_platform_message(engine: usize, message: &Message) -> usize {
    let channel = unsafe { CStr::from_ptr(message.channel) }
        .to_string_lossy()
        .to_string();
    let channel = if channel == "flutter/keyevent" {
        "nanoshell/keyevent".into()
    } else {
        channel
    };
    let channel = CString::new(channel.as_str()).unwrap();
    let message = Message {
        size: message.size,
        channel: channel.as_ptr(),
        message: message.message,
        message_size: message.message_size,
        response_handle: message.response_handle,
    };

    SEND_PLATFORM_MESSAGE.with(|f| f.get().unwrap()(engine, &message))
}

thread_local! {
    static SEND_PLATFORM_MESSAGE : Cell<Option<SendPlatformMessage>> = Cell::new(None);
}

#[repr(C)]
struct EngineProcTable {
    size: isize,
    create_aot_data: isize,
    collect_aot_data: isize,
    run: isize,
    shut_down: isize,
    inititalize: isize,
    deinitialize: isize,
    run_inititalized: isize,
    send_window_metric_event: isize,
    send_pointer_event: isize,
    send_key_event: isize,
    send_platform_message: SendPlatformMessage,
}

pub(super) fn override_key_event(proc_table: *mut c_void) {
    // Fragile as it may be, right now this seems to be the only reasonable way to intercept
    // keyboard events, which is absolutely required for menubar component

    let mut proc_table: &mut EngineProcTable = unsafe { std::mem::transmute(proc_table) };
    if proc_table.size != 280 {
        warn!(
            "Unexpected proc table size {}. Please update shell/platform/common/override_key_event",
            proc_table.size
        );
    }
    SEND_PLATFORM_MESSAGE.with(|v| {
        v.set(Some(proc_table.send_platform_message));
    });
    proc_table.send_platform_message = send_platform_message;
}
