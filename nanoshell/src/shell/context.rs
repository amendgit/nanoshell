use std::rc::Rc;

use crate::{util::LateRefCell, Result};

use super::{
    platform::{drag_data::DragDataAdapter, init::init_platform},
    EngineManager, MenuManager, MessageManager, RunLoop, WindowManager, WindowMethodChannel,
};

pub struct ContextOptions {
    pub app_namespace: String,

    pub on_last_engine_removed: Box<dyn Fn(Rc<Context>) -> ()>,
    pub custom_drag_data_adapters: Vec<Box<dyn DragDataAdapter>>,
}

impl Default for ContextOptions {
    fn default() -> Self {
        Self {
            app_namespace: Default::default(),
            on_last_engine_removed: Box::new(|context| context.run_loop.borrow().stop()),
            custom_drag_data_adapters: Vec::new(),
        }
    }
}

pub struct Context {
    pub options: ContextOptions,
    pub run_loop: LateRefCell<RunLoop>,
    pub engine_manager: LateRefCell<EngineManager>,
    pub message_manager: LateRefCell<MessageManager>,
    pub window_method_channel: LateRefCell<WindowMethodChannel>,
    pub window_manager: LateRefCell<WindowManager>,
    pub menu_manager: LateRefCell<MenuManager>,
}

impl Context {
    pub fn new(options: ContextOptions) -> Result<Rc<Self>> {
        let res = Rc::new(Self {
            options,
            run_loop: LateRefCell::new(),
            engine_manager: LateRefCell::new(),
            message_manager: LateRefCell::new(),
            window_method_channel: LateRefCell::new(),
            window_manager: LateRefCell::new(),
            menu_manager: LateRefCell::new(),
        });
        res.initialize(res.clone())?;
        Ok(res)
    }

    fn initialize(&self, context: Rc<Context>) -> Result<()> {
        self.run_loop.set(RunLoop::new());
        self.engine_manager.set(EngineManager::new(context.clone()));
        self.message_manager
            .set(MessageManager::new(context.clone()));
        self.window_method_channel
            .set(WindowMethodChannel::new(context.clone()));
        self.window_manager.set(WindowManager::new(context.clone()));
        self.menu_manager.set(MenuManager::new(context.clone()));

        init_platform(context.clone()).map_err(|e| e.into())
    }
}
