use std::{
    cell::{Ref, RefCell, RefMut},
    rc::{Rc, Weak},
};

use crate::shell::{Context, IPoint, IRect, PopupMenuRequest};

use super::{
    all_bindings::*,
    error::PlatformResult,
    menu::PlatformMenu,
    util::{GET_X_LPARAM, GET_Y_LPARAM, HIWORD, MAKELONG},
    window_base::WindowBaseState,
};

pub trait WindowMenuDelegate {
    fn get_state<'a>(&'a self) -> Ref<'a, WindowBaseState>;

    fn synthetize_mouse_up(&self);
}

pub struct WindowMenu {
    context: Rc<Context>,
    hwnd: HWND,
    child_hwnd: HWND,
    delegate: Option<Weak<dyn WindowMenuDelegate>>,
    current_menu: RefCell<Option<MenuState>>,
    mouse_state: RefCell<MouseState>,
}

struct MouseState {
    ignore_mouse_leave: bool,
}

struct MenuState {
    platform_menu: Rc<PlatformMenu>,
    request: PopupMenuRequest,
    mouse_in: bool,

    // after pressing left move to previous menu in menubar
    current_item_is_first: bool,

    // after pressing right move to next menu in menubar
    current_item_is_last: bool,
}

// Support mouse tracking while popup menu is visible
impl WindowMenu {
    pub fn new(
        context: Rc<Context>,
        hwnd: HWND,
        child_hwnd: HWND,
        delegate: Weak<dyn WindowMenuDelegate>,
    ) -> Self {
        Self {
            context,
            hwnd: hwnd,
            child_hwnd: child_hwnd,
            delegate: Some(delegate),
            current_menu: RefCell::new(None),
            mouse_state: RefCell::new(MouseState {
                ignore_mouse_leave: false,
            }),
        }
    }

    fn delegate(&self) -> Rc<dyn WindowMenuDelegate> {
        // delegate owns us so unwrap is safe here
        self.delegate.as_ref().and_then(|d| d.upgrade()).unwrap()
    }

    pub fn show_popup<F>(&self, menu: Rc<PlatformMenu>, request: PopupMenuRequest, on_done: F)
    where
        F: FnOnce(PlatformResult<()>) -> () + 'static,
    {
        // We need hook for the tracking rect (if set), but also to forward mouse up
        // because popup menu eats the mouse up message
        let hook = unsafe {
            SetWindowsHookExW(
                WH_MSGFILTER,
                Some(Self::hook_proc),
                HINSTANCE(0),
                GetCurrentThreadId(),
            )
        };

        self.current_menu.borrow_mut().replace(MenuState {
            platform_menu: menu.clone(),
            request: request.clone(),
            mouse_in: false,
            // starting with no item selected, moving left/right moves to next/prev item in menubar
            current_item_is_first: true,
            current_item_is_last: true,
        });

        self.delegate().synthetize_mouse_up();

        let position = self
            .delegate()
            .get_state()
            .local_to_global(&request.position);

        // with popup menu active, the TrackMouseLeaveEvent in flutter view will be fired on every
        // mouse move; we block this in subclass, only allowing our wM_MOUSELEAVE message synthetized
        // when leaving tracking rect
        self.mouse_state.borrow_mut().ignore_mouse_leave = true;

        unsafe {
            TrackPopupMenuEx(
                menu.menu,
                0,
                position.x,
                position.y,
                self.hwnd,
                std::ptr::null_mut(),
            );

            UnhookWindowsHookEx(hook);
        }

        self.current_menu.borrow_mut().take();
        self.mouse_state.borrow_mut().ignore_mouse_leave = false;
        on_done(Ok(()));
    }

    const WM_MENU_HOOK: i32 = WM_USER;

    extern "system" fn hook_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        unsafe {
            let ptr = l_param.0 as *const MSG;
            let msg: &MSG = &*ptr;

            if code == MSGF_MENU {
                // for keydown we need parent hwnd
                let mut parent = GetParent(msg.hwnd);
                if parent.0 == 0 {
                    parent = msg.hwnd;
                }
                SendMessageW(parent, Self::WM_MENU_HOOK as u32, w_param, l_param);
            }
            CallNextHookEx(0, code, w_param, l_param)
        }
    }

    pub fn on_subclass_proc(
        &self,
        _h_wnd: HWND,
        u_msg: u32,
        _w_param: WPARAM,
        _l_param: LPARAM,
    ) -> Option<LRESULT> {
        let mouse_state = self.mouse_state.borrow_mut();

        if u_msg == WM_MOUSELEAVE as u32 && mouse_state.ignore_mouse_leave {
            return Some(LRESULT(0));
        }
        None
    }

    pub fn on_menu_hook(&self, mut msg: MSG) {
        if self.current_menu.borrow().is_none() {
            return;
        }

        let message = msg.message as i32;

        // mouse global to local coordinates for mouse messages
        if message >= WM_MOUSEFIRST && message <= WM_MOUSELAST {
            let mut current_menu =
                RefMut::map(self.current_menu.borrow_mut(), |x| x.as_mut().unwrap());

            let point = IPoint::xy(GET_X_LPARAM(msg.l_param), GET_Y_LPARAM(msg.l_param));
            let point = self.delegate().get_state().global_to_local_physical(&point);
            msg.l_param = LPARAM(MAKELONG(point.x as u16, point.y as u16) as isize);

            if let Some(rect) = &current_menu.request.tracking_rect {
                let scaled: IRect = rect
                    .scaled(self.delegate().get_state().get_scaling_factor())
                    .into();
                if scaled.is_inside(&point) {
                    current_menu.mouse_in = true;
                    unsafe {
                        SendMessageW(self.child_hwnd, msg.message, msg.w_param, msg.l_param);
                    }
                } else if current_menu.mouse_in {
                    current_menu.mouse_in = false;
                    self.mouse_state.borrow_mut().ignore_mouse_leave = false;
                    unsafe {
                        SendMessageW(self.child_hwnd, WM_MOUSELEAVE as u32, WPARAM(1), LPARAM(0));
                    }
                    self.mouse_state.borrow_mut().ignore_mouse_leave = true;
                }
            }
        } else if message == WM_KEYDOWN {
            let current_menu = Ref::map(self.current_menu.borrow(), |x| x.as_ref().unwrap());
            let key = msg.w_param.0 as i32;

            let (key_prev, key_next) = match self.delegate().get_state().is_rtl() {
                true => (VK_RIGHT, VK_LEFT),
                false => (VK_LEFT, VK_RIGHT),
            };

            if key == key_prev && current_menu.current_item_is_first {
                self.context
                    .menu_manager
                    .borrow()
                    .move_to_previous_menu(current_menu.platform_menu.handle);
            } else if key == key_next && current_menu.current_item_is_last {
                self.context
                    .menu_manager
                    .borrow()
                    .move_to_next_menu(current_menu.platform_menu.handle);
            }
        }
    }

    pub fn on_menu_select(&self, _msg: u32, w_param: WPARAM, l_param: LPARAM) {
        if self.current_menu.borrow().is_none() {
            return;
        }

        let mut current_menu = RefMut::map(self.current_menu.borrow_mut(), |x| x.as_mut().unwrap());

        let menu = HMENU(l_param.0);
        let flags = HIWORD(w_param.0 as u32) as i32;

        current_menu.current_item_is_first = menu == current_menu.platform_menu.menu;
        current_menu.current_item_is_last = flags & MF_POPUP == 0;
    }

    pub fn handle_message(
        &self,
        _h_wnd: HWND,
        msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> Option<LRESULT> {
        match msg as i32 {
            WM_MENUCOMMAND => {
                PlatformMenu::on_menu_command(self.context.clone(), HMENU(l_param.0), w_param.0);
            }
            WM_MENUSELECT => {
                self.on_menu_select(msg, w_param, l_param);
            }
            Self::WM_MENU_HOOK => {
                let ptr = l_param.0 as *const MSG;
                let msg: &MSG = unsafe { &*ptr };
                self.on_menu_hook(msg.clone());
            }
            _ => {}
        }
        None
    }
}
