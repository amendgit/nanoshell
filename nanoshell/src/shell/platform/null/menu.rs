use std::rc::{Rc, Weak};

use crate::shell::{Context, Menu, MenuHandle, MenuManager};

use super::error::{PlatformError, PlatformResult};

pub struct PlatformMenu {}

#[allow(unused_variables)]
impl PlatformMenu {
    pub fn new(context: Rc<Context>, handle: MenuHandle) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformMenu>) {}

    pub fn update_from_menu(&self, menu: Menu, manager: &MenuManager) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }
}
