use std::{collections::HashMap, rc::Rc};

use crate::{
    codec::{
        value::{from_value, to_value},
        MethodCall, MethodCallReply, MethodInvoker, Value,
    },
    util::OkLog,
    Error, Result,
};

use super::{
    constants::*, platform::menu::PlatformMenu, Context, EngineHandle, WindowMethodCallResult,
};

struct MenuEntry {
    engine: EngineHandle,
    platform_menu: Rc<PlatformMenu>,
}

pub struct MenuManager {
    context: Rc<Context>,
    platform_menu_map: HashMap<MenuHandle, MenuEntry>,
    next_handle: MenuHandle,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MenuHandle(pub(crate) i64);

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct MenuItem {
    pub id: i64,
    pub title: String,
    pub enabled: bool,
    pub separator: bool,
    pub checked: bool,
    pub submenu: Option<MenuHandle>,
}

impl PartialEq for MenuItem {
    fn eq(&self, other: &Self) -> bool {
        return self.id == other.id;
    }
}

#[derive(serde::Deserialize, Default, Debug)]
pub struct Menu {
    pub items: Vec<MenuItem>,
}
#[derive(serde::Deserialize)]
struct CreateRequest {
    handle: Option<MenuHandle>,
    menu: Menu,
}
#[derive(serde::Deserialize)]
struct DestroyRequest {
    handle: MenuHandle,
}

#[derive(serde::Serialize)]
struct OnMenuAction {
    handle: MenuHandle,
    id: i64,
}

impl MenuManager {
    pub(super) fn new(context: Rc<Context>) -> Self {
        let context_copy = context.clone();
        context
            .message_manager
            .borrow_mut()
            .register_method_handler(channel::MENU_MANAGER, move |value, reply, engine| {
                context_copy
                    .menu_manager
                    .borrow_mut()
                    .on_method_call(value, reply, engine);
            });

        Self {
            context,
            platform_menu_map: HashMap::new(),
            next_handle: MenuHandle(1),
        }
    }

    fn on_create_or_update(
        &mut self,
        request: CreateRequest,
        engine: EngineHandle,
    ) -> Result<MenuHandle> {
        let handle = request.handle.unwrap_or_else(|| {
            let res = self.next_handle.clone();
            self.next_handle.0 += 1;
            res
        });
        let entry = self.platform_menu_map.entry(handle.clone());
        let context = self.context.clone();
        let platform_menu = entry
            .or_insert_with(|| {
                let platform_menu = Rc::new(PlatformMenu::new(context, handle));
                platform_menu.assign_weak_self(Rc::downgrade(&platform_menu));
                MenuEntry {
                    engine: engine,
                    platform_menu: platform_menu,
                }
            })
            .platform_menu
            .clone();
        platform_menu
            .update_from_menu(request.menu, self)
            .map_err(|e| Error::from(e))?;

        Ok(handle)
    }

    pub fn get_platform_menu(&self, menu: MenuHandle) -> Option<Rc<PlatformMenu>> {
        self.platform_menu_map
            .get(&menu)
            .map(|c| c.platform_menu.clone())
    }

    fn invoker_for_menu(&self, menu_handle: MenuHandle) -> Option<MethodInvoker<Value>> {
        self.platform_menu_map.get(&menu_handle).and_then(|e| {
            self.context
                .message_manager
                .borrow()
                .get_method_invoker(e.engine, channel::MENU_MANAGER)
        })
    }

    pub(crate) fn on_menu_action(&self, menu_handle: MenuHandle, id: i64) {
        if let Some(invoker) = self.invoker_for_menu(menu_handle) {
            invoker
                .call_method(
                    method::menu::ON_ACTION.into(),
                    to_value(&OnMenuAction {
                        handle: menu_handle,
                        id: id,
                    })
                    .unwrap(),
                    |_| {},
                )
                .ok_log();
        }
    }

    #[allow(dead_code)] // only used on windows
    pub(crate) fn move_to_previous_menu(&self, menu_handle: MenuHandle) {
        if let Some(invoker) = self.invoker_for_menu(menu_handle) {
            invoker
                .call_method(
                    method::menu_bar::MOVE_TO_PREVIOUS_MENU.into(),
                    Value::Null,
                    |_| {},
                )
                .ok_log();
        }
    }

    #[allow(dead_code)] // only used on windows
    pub(crate) fn move_to_next_menu(&self, menu_handle: MenuHandle) {
        if let Some(invoker) = self.invoker_for_menu(menu_handle) {
            invoker
                .call_method(
                    method::menu_bar::MOVE_TO_NEXT_MENU.into(),
                    Value::Null,
                    |_| {},
                )
                .ok_log();
        }
    }

    fn map_result<T>(result: Result<T>) -> WindowMethodCallResult
    where
        T: serde::Serialize,
    {
        result.map(|v| to_value(v).unwrap()).map_err(|e| e.into())
    }

    fn on_method_call(
        &mut self,
        call: MethodCall<Value>,
        reply: MethodCallReply<Value>,
        engine: EngineHandle,
    ) {
        match call.method.as_str() {
            method::menu::CREATE_OR_UPDATE => {
                let request: CreateRequest = from_value(&call.args).unwrap();
                let res = self.on_create_or_update(request, engine);
                reply.send(Self::map_result(res));
            }
            method::menu::DESTROY => {
                let request: DestroyRequest = from_value(&call.args).unwrap();
                self.platform_menu_map.remove(&request.handle);
                reply.send_ok(Value::Null);
            }
            _ => {}
        };
    }
}
