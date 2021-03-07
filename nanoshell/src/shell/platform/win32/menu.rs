use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use super::{
    all_bindings::*,
    error::{PlatformError, PlatformResult},
    util::to_utf16,
};

use crate::{
    shell::{
        structs::{Menu, MenuItem},
        Context, MenuHandle, MenuManager,
    },
    util::{update_diff, DiffResult},
};

pub struct PlatformMenu {
    pub(super) handle: MenuHandle,
    pub(super) menu: HMENU,
    previous_menu: RefCell<Menu>,
    weak_self: RefCell<Weak<PlatformMenu>>,
}

pub struct PlatformMenuManager {}

impl PlatformMenuManager {
    pub fn new(_context: Rc<Context>) -> Self {
        Self {}
    }

    pub fn set_app_menu(&self, _menu: Rc<PlatformMenu>) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }
}

impl PlatformMenu {
    pub fn new(_context: Rc<Context>, handle: MenuHandle) -> Self {
        let menu = unsafe {
            let menu = CreatePopupMenu();

            let mut info = MENUINFO {
                cb_size: std::mem::size_of::<MENUINFO>() as u32,
                f_mask: (MIM_MENUDATA | MIM_STYLE) as u32,
                dw_style: 0,
                cy_max: 0,
                hbr_back: HBRUSH(0),
                dw_context_help_id: 0,
                dw_menu_data: handle.0 as usize,
            };

            SetMenuInfo(menu, &mut info as *mut _);
            menu
        };
        Self {
            handle,
            menu: menu,
            previous_menu: RefCell::new(Default::default()),
            weak_self: RefCell::new(Weak::new()),
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformMenu>) {
        *self.weak_self.borrow_mut() = weak.clone();
    }

    pub fn get_menu_item_info<'a>(
        item: &MenuItem,
        title: &'a Vec<u16>,
        manager: &MenuManager,
    ) -> MENUITEMINFOW {
        let submenu = item
            .submenu
            .and_then(|h| manager.get_platform_menu(h).ok())
            .map(|m| m.menu)
            .unwrap_or_else(|| HMENU(0));

        let item_type = {
            match item.separator {
                true => MFT_SEPARATOR,
                false => MFT_STRING,
            }
        };

        let mut state = MFS_ENABLED;
        if !item.enabled {
            state |= MFS_DISABLED;
        }
        if item.checked {
            state |= MFS_CHECKED;
        }

        MENUITEMINFOW {
            cb_size: std::mem::size_of::<MENUITEMINFOW>() as u32,
            f_mask: (MIIM_FTYPE | MIIM_ID | MIIM_STATE | MIIM_STRING | MIIM_SUBMENU) as u32,
            f_type: item_type as u32,
            f_state: state as u32,
            w_id: item.id as u32,
            h_sub_menu: submenu,
            hbmp_checked: HBITMAP(0),
            hbmp_unchecked: HBITMAP(0),
            dw_item_data: 0,
            dw_type_data: title.as_ptr() as *mut _,
            cch: title.len() as u32,
            hbmp_item: HBITMAP(0),
        }
    }

    pub fn update_from_menu(&self, menu: Menu, manager: &MenuManager) -> PlatformResult<()> {
        let mut previous_menu = self.previous_menu.borrow_mut();

        let diff = update_diff(&previous_menu.items, &menu.items, |a, b| {
            Self::can_update(a, b)
        });

        // First remove items for menu;
        let diff: Vec<_> = diff
            .iter()
            .filter_map(|res| match res {
                DiffResult::Remove(res) => {
                    unsafe {
                        RemoveMenu(self.menu, res.id as u32, MF_BYCOMMAND as u32);
                    }
                    None
                }
                _ => Some(res),
            })
            .collect();

        for i in 0..diff.len() {
            let d = diff[i];
            match d {
                DiffResult::Remove(_) => {
                    panic!("Should have been already removed.")
                }
                DiffResult::Keep(_, _) => {
                    // nothing
                }
                DiffResult::Update(old, new) => {
                    let title = to_utf16(&self.title_for_item(&new));
                    let mut info = Self::get_menu_item_info(new, &title, manager);
                    unsafe {
                        SetMenuItemInfoW(self.menu, old.id as u32, FALSE, &mut info as *mut _);
                    }
                }
                DiffResult::Insert(item) => {
                    let title = to_utf16(&self.title_for_item(&item));
                    let mut info = Self::get_menu_item_info(item, &title, manager);
                    unsafe {
                        InsertMenuItemW(self.menu, i as u32, TRUE, &mut info as *mut _);
                    }
                }
            }
        }

        *previous_menu = menu;

        Ok(())
    }

    fn title_for_item(&self, item: &MenuItem) -> String {
        let mut res = item.title.clone();
        if let Some(accelerator) = &item.accelerator {
            let mut separator = '\t';

            if accelerator.control {
                res.push(separator);
                res.push_str("Ctrl");
                separator = '+';
            }
            if accelerator.alt {
                res.push(separator);
                res.push_str("Alt");
                separator = '+';
            }
            if accelerator.shift {
                res.push(separator);
                res.push_str("Shift");
                separator = '+';
            }
            if accelerator.meta {
                res.push(separator);
                res.push_str("Win");
                separator = '+';
            }
            res.push(separator);
            res.push_str(&accelerator.label);
        }
        res
    }

    fn can_update(_old_item: &MenuItem, _new_item: &MenuItem) -> bool {
        true
    }
}

impl Drop for PlatformMenu {
    fn drop(&mut self) {
        unsafe {
            DestroyMenu(self.menu);
        }
    }
}
