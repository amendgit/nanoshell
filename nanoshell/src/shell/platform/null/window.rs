use std::rc::{Rc, Weak};

use crate::{
    codec::Value,
    shell::{
        Context, DragEffect, DragRequest, PlatformWindowDelegate, PopupMenuRequest, WindowGeometry,
        WindowGeometryFlags, WindowGeometryRequest, WindowStyle,
    },
};

use super::{
    engine::PlatformEngine,
    error::{PlatformError, PlatformResult},
    menu::PlatformMenu,
};

pub struct PlatformWindow {}

#[allow(unused_variables)]
impl PlatformWindow {
    pub fn new(
        context: Rc<Context>,
        delegate: Weak<dyn PlatformWindowDelegate>,
        parent: Option<Rc<PlatformWindow>>,
    ) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformWindow>, engine: &PlatformEngine) {}

    pub fn show(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn ready_to_show(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn close(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn close_with_result(&self, result: Value) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn hide(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn show_modal<F>(&self, done_callback: F)
    where
        F: FnOnce(PlatformResult<Value>) -> () + 'static,
    {
        done_callback(Err(PlatformError::NotImplemented))
    }

    pub fn set_geometry(
        &self,
        geometry: WindowGeometryRequest,
    ) -> PlatformResult<WindowGeometryFlags> {
        Err(PlatformError::NotImplemented)
    }

    pub fn get_geometry(&self) -> PlatformResult<WindowGeometry> {
        Err(PlatformError::NotImplemented)
    }

    pub fn supported_geometry(&self) -> PlatformResult<WindowGeometryFlags> {
        Err(PlatformError::NotImplemented)
    }

    pub fn set_style(&self, style: WindowStyle) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn perform_window_drag(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn begin_drag_session(&self, request: DragRequest) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn set_pending_effect(&self, effect: DragEffect) {}

    pub fn show_popup_menu<F>(&self, menu: Rc<PlatformMenu>, request: PopupMenuRequest, on_done: F)
    where
        F: FnOnce(PlatformResult<()>) -> () + 'static,
    {
        on_done(Err(PlatformError::NotImplemented))
    }
}
