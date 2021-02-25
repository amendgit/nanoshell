use std::{ptr::null_mut, rc::Rc};

use crate::shell::Context;

use super::{
    all_bindings::*, dpi::become_dpi_aware, dxgi_hook::init_dxgi_hook, error::PlatformResult,
    util::ErrorCodeExt,
};

pub fn init_platform(_context: Rc<Context>) -> PlatformResult<()> {
    unsafe {
        OleInitialize(null_mut()).as_platform_result()?;
    }
    init_dxgi_hook();
    become_dpi_aware();
    Ok(())
}
