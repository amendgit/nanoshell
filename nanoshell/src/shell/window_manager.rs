use std::{collections::HashMap, rc::Rc};

use velcro::hash_map;

use crate::{
    codec::{
        value::{from_value, to_value},
        MessageCodec, MessageSender, MethodCallError, StandardMethodCodec, Value,
    },
    util::OkLog,
};

use super::{
    constants::*, platform::window::PlatformWindow, Context, EngineHandle, PlatformWindowDelegate,
    Window, WindowHandle, WindowMethodCall, WindowMethodCallReply,
};

pub struct WindowManager {
    context: Rc<Context>,
    windows: HashMap<WindowHandle, Rc<Window>>,
    next_handle: WindowHandle,
    engine_to_window: HashMap<EngineHandle, WindowHandle>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowCreateRequest {
    parent: WindowHandle,
    init_data: Value,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowCreateResponse {
    window_handle: WindowHandle,
}

impl WindowManager {
    pub(super) fn new(context: Rc<Context>) -> Self {
        let context_copy = context.clone();
        context
            .window_method_channel
            .borrow_mut()
            .register_method_handler(channel::win::WINDOW_MANAGER, move |call, reply, engine| {
                Self::on_method_call(context_copy.clone(), call, reply, engine);
            });

        let context_copy = context.clone();
        context
            .window_method_channel
            .borrow_mut()
            .register_method_handler(channel::win::DRAG_SOURCE, move |call, reply, engine| {
                Self::on_method_call(context_copy.clone(), call, reply, engine);
            });

        WindowManager {
            context,
            windows: HashMap::new(),
            next_handle: WindowHandle(1),
            engine_to_window: HashMap::new(),
        }
    }

    pub fn create_window(
        &mut self,
        init_data: Value,
        parent: Option<WindowHandle>,
    ) -> WindowHandle {
        let window_handle = self.next_handle;
        self.next_handle.0 += 1;

        let engine_handle = self.context.engine_manager.borrow_mut().create_engine();

        self.engine_to_window.insert(engine_handle, window_handle);

        let window = Rc::new(Window::new(
            self.context.clone(),
            window_handle,
            engine_handle,
            init_data,
            parent,
        ));

        window.assign_weak_self(Rc::downgrade(&window));

        let parent_platform_window = parent
            .and_then(|h| self.windows.get(&h))
            .map(|w| w.platform_window.borrow().clone());

        let platform_window = Rc::new(PlatformWindow::new(
            self.context.clone(),
            Rc::downgrade(&(window.clone() as Rc<dyn PlatformWindowDelegate>)),
            parent_platform_window,
        ));

        self.windows.insert(window_handle, window.clone());

        platform_window.assign_weak_self(
            Rc::downgrade(&platform_window),
            &self
                .context
                .engine_manager
                .borrow()
                .get_engine(engine_handle)
                .unwrap()
                .platform_engine,
        );
        window.platform_window.set(platform_window);

        self.context
            .engine_manager
            .borrow_mut()
            .launch_engine(engine_handle)
            .ok_log();

        window_handle
    }

    pub(super) fn remove_window(&mut self, window: &Window) {
        self.context
            .engine_manager
            .borrow_mut()
            .remove_engine(window.engine_handle)
            .ok_log();
        self.windows.remove(&window.window_handle);
    }

    fn on_init(&self, window: WindowHandle) -> Value {
        let all_handles = self.windows.keys().map(|h| Value::I64(h.0));
        let all_handles: Vec<Value> = all_handles.collect();
        let window = self.windows.get(&window).unwrap();
        window.initialized.replace(true);
        let parent = window
            .parent
            .map(|h| h.0.into())
            .unwrap_or_else(|| Value::Null);
        Value::Map(hash_map!(
            "allWindows".into() : all_handles.into(),
            "currentWindow".into() : window.window_handle.0.into(),
            "initData".into(): window.init_data.clone(),
            "parentWindow".into(): parent,
        ))
    }

    fn on_create_window(&mut self, argument: Value, parent: WindowHandle) -> Value {
        let win = self.create_window(argument, Some(parent));
        to_value(&WindowCreateResponse { window_handle: win }).unwrap()
    }

    pub(crate) fn message_sender_for_window(
        &self,
        handle: WindowHandle,
        channel_name: &str,
    ) -> Option<MessageSender<Value>> {
        let manager = self.context.message_manager.borrow();
        self.windows
            .get(&handle)
            .and_then(|w| manager.get_message_sender(w.engine_handle, channel_name))
    }

    fn on_method_call(
        context: Rc<Context>,
        call: WindowMethodCall,
        reply: WindowMethodCallReply,
        engine: EngineHandle,
    ) {
        match call.method.as_str() {
            method::window::INIT => {
                let window = context
                    .window_manager
                    .borrow()
                    .engine_to_window
                    .get(&engine)
                    .map(|w| w.clone());
                match window {
                    Some(window) => {
                        reply.send(Ok(context.window_manager.borrow().on_init(window)));
                        context
                            .window_method_channel
                            .borrow()
                            .get_message_broadcaster(window, channel::win::WINDOW_MANAGER)
                            .broadcast_message(event::window::INITIALIZE, Value::Null);
                    }
                    None => reply.send(Err(MethodCallError {
                        code: "no-window".into(),
                        message: Some("No window associated with engine".into()),
                        details: Value::Null,
                    })),
                }
            }
            method::window::CREATE => {
                let create_request: WindowCreateRequest = from_value(&call.arguments).unwrap();
                reply.send(Ok(context
                    .window_manager
                    .borrow_mut()
                    .on_create_window(create_request.init_data, create_request.parent)));
            }
            _ => {
                let window = {
                    context
                        .window_manager
                        .borrow()
                        .windows
                        .get(&call.target_window_handle)
                        .map(|c| c.clone())
                };
                if let Some(window) = window {
                    window.on_message(&call.method, call.arguments, reply);
                } else {
                    reply.send(Err(MethodCallError {
                        code: "no-window".into(),
                        message: Some("Target window not found".into()),
                        details: Value::Null,
                    }));
                }
            }
        }
    }

    pub(crate) fn broadcast_message(&self, message: Value) {
        let codec: &'static dyn MessageCodec<Value> = &StandardMethodCodec;
        // we use binary messenger directly to be able to encode the message only once
        let message = codec.encode_message(&message);
        for window in self.windows.values() {
            if !window.initialized.get() {
                continue;
            }
            let manager = self.context.engine_manager.borrow();
            let engine = manager.get_engine(window.engine_handle);
            if let Some(engine) = engine {
                engine
                    .binary_messenger()
                    .post_message(channel::DISPATCHER, &message)
                    .ok_log();
            }
        }
    }
}
