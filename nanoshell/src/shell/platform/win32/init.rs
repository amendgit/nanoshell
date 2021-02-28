use std::{ptr::null_mut, rc::Rc};

use crate::shell::Context;

use super::{
    all_bindings::*, dpi::become_dpi_aware, dxgi_hook::init_dxgi_hook, error::PlatformResult,
    util::ErrorCodeExt,
};

pub fn init_platform(_context: Rc<Context>) -> PlatformResult<()> {
    unsafe {
        // Angle will try opening these with GetModuleHandleEx, which means they need to be
        // loaded first; Otherwise it falls back to d3dcompiler_47, which is not present on
        // some Windows 7 installations.
        if LoadLibraryW(utf16_lit::utf16_null!("d3dcompiler_47.dll").as_ptr()) == 0 {
            if LoadLibraryW(utf16_lit::utf16_null!("d3dcompiler_46.dll").as_ptr()) == 0 {
                LoadLibraryW(utf16_lit::utf16_null!("d3dcompiler_43.dll").as_ptr());
            }
        }

        OleInitialize(null_mut()).as_platform_result()?;
    }
    init_dxgi_hook();
    become_dpi_aware();
    Ok(())
}
