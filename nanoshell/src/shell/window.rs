use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

use crate::{
    codec::{
        value::{from_value, to_value},
        Value,
    },
    util::{LateRefCell, OkLog},
    Result,
};

use super::{
    constants::*, platform::window::PlatformWindow, Context, DragEffect, DragRequest, DragResult,
    DraggingInfo, EngineHandle, PopupMenuRequest, WindowGeometry, WindowGeometryFlags,
    WindowGeometryRequest, WindowMethodCallReply, WindowMethodCallResult, WindowMethodInvoker,
    WindowStyle,
};

#[derive(serde::Serialize, serde::Deserialize, Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct WindowHandle(pub(super) i64);

pub(super) struct Window {
    context: Rc<Context>,
    pub(super) window_handle: WindowHandle,
    pub(super) engine_handle: EngineHandle,
    pub(super) platform_window: LateRefCell<Rc<PlatformWindow>>,
    pub(super) init_data: Value,
    pub(super) parent: Option<WindowHandle>,
    pub(super) initialized: Cell<bool>,
    weak_self: LateRefCell<Weak<Self>>,
}

impl Window {
    pub(crate) fn new(
        context: Rc<Context>,
        window_handle: WindowHandle,
        engine_handle: EngineHandle,
        init_data: Value,
        parent: Option<WindowHandle>,
    ) -> Self {
        Self {
            context,
            window_handle,
            engine_handle: engine_handle,
            platform_window: LateRefCell::new(),
            init_data,
            parent,
            initialized: Cell::new(false),
            weak_self: LateRefCell::new(),
        }
    }

    pub(crate) fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    // fn invoke_method<F>(&self, method: &str, arg: Value, reply: F)
    // where
    //     F: FnOnce(Result<Value, PlatformError>) -> () + 'static,
    // {
    //     let message = encode_method(&self.window_handle, method, arg);
    //     self.message_sender.send_message(&message, |r| {
    //         reply(decode_result(r));
    //     });
    // }

    fn broadcast_message(&self, message: &str, arguments: Value) {
        let broadcaster = self
            .context
            .window_method_channel
            .borrow()
            .get_message_broadcaster(self.window_handle, channel::win::WINDOW_MANAGER);
        broadcaster.broadcast_message(message, arguments);
    }

    fn drop_target_invoker(&self) -> WindowMethodInvoker {
        self.context
            .window_method_channel
            .borrow()
            .get_method_invoker(
                &self.context.window_manager.borrow(),
                self.window_handle,
                channel::win::DROP_TARGET,
            )
            .unwrap()
    }

    fn drag_source_invoker(&self) -> WindowMethodInvoker {
        self.context
            .window_method_channel
            .borrow()
            .get_method_invoker(
                &self.context.window_manager.borrow(),
                self.window_handle,
                channel::win::DRAG_SOURCE,
            )
            .unwrap()
    }

    fn platform_window(&self) -> Rc<PlatformWindow> {
        self.platform_window.borrow().clone()
    }

    fn show(&self) -> Result<()> {
        self.platform_window().show().map_err(|e| e.into())
    }

    fn ready_to_show(&self) -> Result<()> {
        self.platform_window().ready_to_show().map_err(|e| e.into())
    }

    fn close(&self) -> Result<()> {
        self.platform_window().close().map_err(|e| e.into())
    }

    fn close_with_result(&self, result: Value) -> Result<()> {
        self.platform_window()
            .close_with_result(result)
            .map_err(|e| e.into())
    }

    fn hide(&self) -> Result<()> {
        self.platform_window().hide().map_err(|e| e.into())
    }

    fn set_geometry(&self, geometry: WindowGeometryRequest) -> Result<WindowGeometryFlags> {
        self.platform_window()
            .set_geometry(geometry)
            .map_err(|e| e.into())
    }

    fn get_geometry(&self) -> Result<WindowGeometry> {
        self.platform_window().get_geometry().map_err(|e| e.into())
    }

    fn supported_geometry(&self) -> Result<WindowGeometryFlags> {
        self.platform_window()
            .supported_geometry()
            .map_err(|e| e.into())
    }

    fn set_style(&self, style: WindowStyle) -> Result<()> {
        self.platform_window()
            .set_style(style)
            .map_err(|e| e.into())
    }

    fn perform_window_drag(&self) -> Result<()> {
        self.platform_window()
            .perform_window_drag()
            .map_err(|e| e.into())
    }

    fn begin_drag_session(&self, request: DragRequest) -> Result<()> {
        self.platform_window()
            .begin_drag_session(request)
            .map_err(|e| e.into())
    }

    fn show_popup_menu<F>(&self, request: PopupMenuRequest, on_done: F)
    where
        F: FnOnce(Result<()>) -> () + 'static,
    {
        self.platform_window().show_popup_menu(
            self.context
                .menu_manager
                .borrow()
                .get_platform_menu(request.handle)
                .unwrap(),
            request,
            |r| on_done(r.map_err(|e| e.into())),
        )
    }

    fn map_result<T>(result: Result<T>) -> WindowMethodCallResult
    where
        T: serde::Serialize,
    {
        result.map(|v| to_value(v).unwrap()).map_err(|e| e.into())
    }

    fn reply<'a, T, F, A>(reply: WindowMethodCallReply, arg: &'a Value, c: F)
    where
        F: FnOnce(A) -> Result<T>,
        T: serde::Serialize,
        A: serde::Deserialize<'a>,
    {
        let a: std::result::Result<A, _> = from_value(arg);
        match a {
            Ok(a) => {
                let res = c(a);
                let res = Self::map_result(res);
                reply.send(res);
            }
            Err(err) => {
                reply.send(Self::map_result::<()>(Err(err.into())));
            }
        }
    }

    pub(super) fn on_message(&self, method: &str, arg: Value, reply: WindowMethodCallReply) {
        match method {
            method::window::SHOW => {
                return Self::reply(reply, &arg, |()| self.show());
            }
            method::window::SHOW_MODAL => {
                return self.platform_window().show_modal(move |result| {
                    reply.send(Self::map_result(result.map_err(|e| e.into())))
                });
            }
            method::window::READY_TO_SHOW => {
                return Self::reply(reply, &arg, |()| self.ready_to_show());
            }
            method::window::CLOSE => {
                return Self::reply(reply, &arg, |()| self.close());
            }
            method::window::CLOSE_WITH_RESULT => {
                return Self::reply(reply, &arg, |arg| self.close_with_result(arg));
            }
            method::window::HIDE => {
                return Self::reply(reply, &arg, |()| self.hide());
            }
            method::window::SET_GEOMETRY => {
                return Self::reply(reply, &arg, |geometry| self.set_geometry(geometry));
            }
            method::window::GET_GEOMETRY => {
                return Self::reply(reply, &arg, |()| self.get_geometry());
            }
            method::window::SUPPORTED_GEOMETRY => {
                return Self::reply(reply, &arg, |()| self.supported_geometry());
            }
            method::window::SET_STYLE => {
                return Self::reply(reply, &arg, |style| self.set_style(style));
            }
            method::window::PERFORM_WINDOW_DRAG => {
                return Self::reply(reply, &arg, |()| self.perform_window_drag());
            }
            method::window::SHOW_POPUP_MENU => {
                let request: std::result::Result<PopupMenuRequest, _> = from_value(&arg);
                match request {
                    Ok(request) => {
                        return self
                            .show_popup_menu(request, move |res| reply.send(Self::map_result(res)))
                    }
                    Err(err) => return reply.send(Self::map_result::<()>(Err(err.into()))),
                }
            }
            method::drag_source::BEGIN_DRAG_SESSION => {
                return Self::reply(reply, &arg, |request| self.begin_drag_session(request));
            }
            _ => {}
        }

        reply.send(Ok(Value::Null));
    }
}

pub trait PlatformWindowDelegate {
    fn visibility_changed(&self, visible: bool);
    fn did_request_close(&self);
    fn will_close(&self);

    fn dragging_exited(&self);
    fn dragging_updated(&self, info: &DraggingInfo);
    fn perform_drop(&self, info: &DraggingInfo);

    fn drag_ended(&self, effect: DragEffect);
}

impl PlatformWindowDelegate for Window {
    fn visibility_changed(&self, visible: bool) {
        self.broadcast_message(event::window::VISIBILITY_CHANGED, Value::Bool(visible));
    }

    fn did_request_close(&self) {
        self.broadcast_message(event::window::CLOSE_REQUEST, Value::Null);
    }

    fn will_close(&self) {
        self.broadcast_message(event::window::CLOSE, Value::Null);
        self.context.window_manager.borrow_mut().remove_window(self);
    }

    fn dragging_exited(&self) {
        self.drop_target_invoker()
            .call_method(method::drop_target::DRAGGING_EXITED, Value::Null, |_| {})
            .ok_log();
    }

    fn dragging_updated(&self, info: &DraggingInfo) {
        let weak = self.weak_self.clone_value();
        self.drop_target_invoker()
            .call_method(
                method::drop_target::DRAGGING_UPDATED,
                to_value(info).unwrap(),
                move |r| {
                    let s = weak.upgrade();
                    if let (Ok(result), Some(s)) = (r, s) {
                        let result: DragResult =
                            from_value(&result).ok_log().unwrap_or(DragResult {
                                effect: DragEffect::None,
                            });
                        s.platform_window().set_pending_effect(result.effect);
                    }
                },
            )
            .ok_log();
    }

    fn perform_drop(&self, info: &DraggingInfo) {
        self.drop_target_invoker()
            .call_method(
                method::drop_target::PERFORM_DROP,
                to_value(info).unwrap(),
                |_| {},
            )
            .ok_log();
    }

    fn drag_ended(&self, effect: DragEffect) {
        self.drag_source_invoker()
            .call_method(
                method::drag_source::DRAG_SESSION_ENDED,
                to_value(effect).unwrap(),
                |_| {},
            )
            .ok_log();
    }
}
