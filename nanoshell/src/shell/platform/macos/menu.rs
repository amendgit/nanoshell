use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::c_void,
    rc::{Rc, Weak},
};

use cocoa::{
    appkit::{NSMenu, NSMenuItem},
    base::{id, nil, NO},
    foundation::NSInteger,
};
use objc::{
    declare::ClassDecl,
    rc::StrongPtr,
    runtime::{Class, Object, Sel},
};

use crate::{
    shell::{Context, Menu, MenuHandle, MenuItem, MenuManager},
    util::{update_diff, DiffResult, LateRefCell},
};

use super::{
    error::PlatformResult,
    utils::{superclass, to_nsstring},
};

pub struct PlatformMenu {
    context: Rc<Context>,
    handle: MenuHandle,
    pub(super) menu: StrongPtr,
    previous_menu: RefCell<Menu>,
    id_to_menu_item: RefCell<HashMap<i64, StrongPtr>>,
    target: StrongPtr,
    weak_self: LateRefCell<Weak<PlatformMenu>>,
}

impl PlatformMenu {
    pub fn new(context: Rc<Context>, handle: MenuHandle) -> Self {
        unsafe {
            let menu: id = NSMenu::alloc(nil).initWithTitle_(*to_nsstring("Menu title"));
            let _: () = msg_send![menu, setAutoenablesItems: NO];
            let target: id = msg_send![MENU_ITEM_TARGET_CLASS.0, new];
            let target = StrongPtr::new(target);
            Self {
                context,
                handle,
                menu: StrongPtr::new(menu),
                previous_menu: RefCell::new(Default::default()),
                id_to_menu_item: RefCell::new(HashMap::new()),
                target: target,
                weak_self: LateRefCell::new(),
            }
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformMenu>) {
        self.weak_self.set(weak.clone());
        unsafe {
            let state_ptr = Box::into_raw(Box::new(weak.clone())) as *mut c_void;
            (**self.target).set_ivar("imState", state_ptr);
        }
    }

    pub fn update_from_menu(&self, menu: Menu, manager: &MenuManager) -> PlatformResult<()> {
        let mut previous_menu = self.previous_menu.borrow_mut();

        let diff = update_diff(&previous_menu.items, &menu.items, |a, b| {
            Self::can_update(a, b)
        });

        // First remove items for menu; This is necessary in case we're reordering a
        // item with submenu - we have to remove it first otherwise we get exception
        // if adding same submenu while it already exists
        let diff: Vec<_> = diff
            .iter()
            .filter_map(|res| match res {
                DiffResult::Remove(res) => {
                    let item = self.id_to_menu_item.borrow_mut().remove(&res.id);
                    if let Some(item) = item {
                        unsafe {
                            // remove submenu, just in case
                            let _: () = msg_send![*item, setMenu: nil];
                            let _: () = msg_send![*self.menu, removeItem:*item];
                        }
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
                    let item = self
                        .id_to_menu_item
                        .borrow_mut()
                        .remove(&old.id)
                        .unwrap()
                        .clone();
                    self.id_to_menu_item
                        .borrow_mut()
                        .insert(new.id, item.clone());
                    self.update_menu_item(*item, new, manager);
                }
                DiffResult::Insert(item) => {
                    let menu_item = self.create_menu_item(item, manager);
                    self.id_to_menu_item
                        .borrow_mut()
                        .insert(item.id, menu_item.clone());
                    unsafe { msg_send![*self.menu, insertItem:*menu_item atIndex:i as NSInteger] }
                }
            }
        }

        *previous_menu = menu;

        assert!(
            previous_menu.items.len() == self.id_to_menu_item.borrow().len(),
            "Array length mismatch"
        );

        Ok(())
    }

    fn can_update(old_item: &MenuItem, new_item: &MenuItem) -> bool {
        // can't change separator item to non separator
        return old_item.separator == new_item.separator;
    }

    fn update_menu_item(&self, item: id, menu_item: &MenuItem, menu_manager: &MenuManager) {
        if menu_item.separator {
            return;
        }
        unsafe {
            if let Some(submenu) = menu_item
                .submenu
                .and_then(|s| menu_manager.get_platform_menu(s))
            {
                let _: () = msg_send![item, setSubmenu:*submenu.menu];
                let _: () = msg_send![item, setTarget: nil];
            } else {
                let _: () = msg_send![item, setSubmenu: nil];
                let _: () = msg_send![item, setTarget: *self.target];
                let _: () = msg_send![item, setAction: sel!(onAction:)];
            }
            let _: () = msg_send![item, setTitle:*to_nsstring(&menu_item.title)];
            let _: () = msg_send![item, setEnabled:menu_item.enabled];
            let state: NSInteger = {
                match menu_item.checked {
                    true => 1,
                    false => 0,
                }
            };
            let _: () = msg_send![item, setState: state];
            let number: id = msg_send![class!(NSNumber), numberWithLongLong:menu_item.id];
            let _: () = msg_send![item, setRepresentedObject: number];
        }
    }

    fn menu_item_action(&self, item: id) {
        let item_id = unsafe {
            let object: id = msg_send![item, representedObject];
            msg_send![object, longLongValue]
        };
        self.context
            .menu_manager
            .borrow()
            .on_menu_action(self.handle, item_id);
    }

    fn create_menu_item(&self, menu_item: &MenuItem, menu_manager: &MenuManager) -> StrongPtr {
        unsafe {
            if menu_item.separator {
                let res = NSMenuItem::separatorItem(nil);
                StrongPtr::retain(res)
            } else {
                let res = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                    *to_nsstring(""),
                    Sel::from_ptr(0 as *const _),
                    *to_nsstring(""),
                );
                self.update_menu_item(res, menu_item, menu_manager);
                StrongPtr::new(res)
            }
        }
    }
}

struct MenuItemTargetClass(*const Class);
unsafe impl Sync for MenuItemTargetClass {}

lazy_static! {
    static ref MENU_ITEM_TARGET_CLASS: MenuItemTargetClass = unsafe {
        let target_superclass = class!(NSObject);
        let mut decl = ClassDecl::new("IMMenuItemTarget", target_superclass).unwrap();

        decl.add_ivar::<*mut c_void>("imState");

        decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
        decl.add_method(
            sel!(onAction:),
            on_action as extern "C" fn(&Object, Sel, id),
        );

        MenuItemTargetClass(decl.register())
    };
}

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    let state_ptr = unsafe {
        let state_ptr: *mut c_void = *this.get_ivar("imState");
        &mut *(state_ptr as *mut Weak<PlatformMenu>)
    };
    unsafe {
        Box::from_raw(state_ptr);

        let superclass = superclass(this);
        let _: () = msg_send![super(this, superclass), dealloc];
    }
}

extern "C" fn on_action(this: &Object, _sel: Sel, sender: id) -> () {
    let state_ptr = unsafe {
        let state_ptr: *mut c_void = *this.get_ivar("imState");
        &mut *(state_ptr as *mut Weak<PlatformMenu>)
    };
    let upgraded = state_ptr.upgrade();
    if let Some(upgraded) = upgraded {
        upgraded.menu_item_action(sender);
    }
}
