use core::slice;
use std::{
    ffi::{c_void, CStr},
    mem::size_of,
    ptr::null_mut,
    u32,
};

use widestring::WideCStr;
use windows::{Guid, FALSE, TRUE};

use crate::shell::structs::{DragEffect, ImageData};

use super::{all_bindings::*, util::as_u8_slice};

use byte_slice_cast::*;

// Declare our own version, windows RS one leaks
// https://github.com/microsoft/windows-rs/issues/548
#[link(name = "OLE32")]
extern "system" {
    pub fn RegisterDragDrop(hwnd: HWND, p_drop_target: usize) -> ::windows::ErrorCode;
    pub fn DoDragDrop(
        p_data_obj: usize,
        p_drop_source: usize,
        dw_ok_effects: u32,
        pdw_effect: *mut u32,
    ) -> ::windows::ErrorCode;
}

#[allow(non_upper_case_globals)]
pub const CLSID_DragDropHelper: Guid = Guid::from_values(
    0x4657278a,
    0x411b,
    0x11d2,
    [0x83, 0x9a, 0x0, 0xc0, 0x4f, 0xd9, 0x18, 0xd0],
);

#[derive(Clone, Debug)]
pub struct DROPEFFECT(pub u32);

pub const DROPEFFECT_NONE: i32 = 0;
pub const DROPEFFECT_COPY: i32 = 1;
pub const DROPEFFECT_MOVE: i32 = 2;
pub const DROPEFFECT_LINK: i32 = 4;

pub fn convert_drop_effect_mask(mask: DROPEFFECT) -> Vec<DragEffect> {
    let mut res = Vec::new();
    let mask = mask.0 as i32;
    if mask & DROPEFFECT_COPY == DROPEFFECT_COPY {
        res.push(DragEffect::Copy);
    }
    if mask & DROPEFFECT_MOVE == DROPEFFECT_MOVE {
        res.push(DragEffect::Move);
    }
    if mask & DROPEFFECT_LINK == DROPEFFECT_LINK {
        res.push(DragEffect::Link);
    }
    res
}

pub fn convert_drag_effect(effect: &DragEffect) -> DROPEFFECT {
    let res = match effect {
        DragEffect::None => DROPEFFECT_NONE,
        DragEffect::Copy => DROPEFFECT_COPY,
        DragEffect::Link => DROPEFFECT_LINK,
        DragEffect::Move => DROPEFFECT_MOVE,
    };
    DROPEFFECT(res as u32)
}

pub fn convert_drag_effects(effects: &[DragEffect]) -> DROPEFFECT {
    let mut res: u32 = 0;
    for e in effects {
        res |= convert_drag_effect(e).0;
    }
    DROPEFFECT(res)
}

pub fn create_dragimage_bitmap(image: &ImageData) -> HBITMAP {
    const DIB_RGB_COLORS: i32 = 0x00;
    const BI_RGB: i32 = 0;

    let bitmap = BITMAPINFO {
        bmi_header: BITMAPINFOHEADER {
            bi_size: size_of::<BITMAPINFOHEADER>() as u32,
            bi_width: image.width,
            bi_height: image.height,
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: BI_RGB as u32,
            bi_size_image: (image.width * image.height * 4) as u32,
            bi_xpels_per_meter: 0,
            bi_ypels_per_meter: 0,
            bi_clr_used: 0,
            bi_clr_important: 0,
        },
        bmi_colors: Default::default(),
    };

    unsafe {
        let dc = GetDC(HWND(0));

        let mut ptr = std::ptr::null_mut();

        let bitmap = CreateDIBSection(
            dc,
            &bitmap as *const _,
            DIB_RGB_COLORS as u32,
            &mut ptr as *mut *mut _ as *mut *mut c_void,
            HANDLE(0),
            0,
        );

        // Bitmap needs to be flipped and unpremultiplied

        let dst_stride = (image.width * 4) as isize;
        let ptr = ptr as *mut u8;
        for y in 0..image.height as isize {
            let src_line = image
                .data
                .as_ptr()
                .offset((image.height as isize - y - 1) * image.bytes_per_row as isize);

            let dst_line = ptr.offset(y * dst_stride);

            for x in (0..dst_stride).step_by(4) {
                let (r, g, b, a) = (
                    *src_line.offset(x) as i32,
                    *src_line.offset(x + 1) as i32,
                    *src_line.offset(x + 2) as i32,
                    *src_line.offset(x + 3) as i32,
                );

                let (r, g, b) = if a == 0 {
                    (0, 0, 0)
                } else {
                    (r * 255 / a, g * 255 / a, b * 255 / a)
                };
                *dst_line.offset(x) = b as u8;
                *dst_line.offset(x + 1) = g as u8;
                *dst_line.offset(x + 2) = r as u8;
                *dst_line.offset(x + 3) = a as u8;
            }
        }

        ReleaseDC(HWND(0), dc);

        return bitmap;
    }
}

pub struct DataUtil {}

impl DataUtil {
    pub fn get_data(object: IDataObject, format: u32) -> windows::Result<Vec<u8>> {
        let mut format = Self::get_format(format);

        let mut medium = STGMEDIUM_::default();

        unsafe {
            object
                .GetData(
                    &mut format as *mut _,
                    &mut medium as *mut STGMEDIUM_ as *mut STGMEDIUM,
                )
                .ok()?;

            let size = GlobalSize(medium.data as isize);
            let data = GlobalLock(medium.data as isize);

            let v = slice::from_raw_parts(data as *const u8, size);
            let res: Vec<u8> = v.into();

            GlobalUnlock(medium.data as isize);

            ReleaseStgMedium(&mut medium as *mut STGMEDIUM_ as *mut STGMEDIUM);

            Ok(res)
        }
    }

    pub fn has_data(object: IDataObject, format: u32) -> bool {
        let mut format = Self::get_format(format);
        unsafe { object.QueryGetData(&mut format as *mut _).is_ok() }
    }

    pub fn extract_files(buffer: Vec<u8>) -> Vec<String> {
        let files: &DROPFILES = unsafe { &*(buffer.as_ptr() as *const DROPFILES) };

        let mut res = Vec::new();
        if files.f_wide == TRUE {
            let data = buffer.as_slice()[files.p_files as usize..]
                .as_slice_of::<u16>()
                .unwrap();
            let mut offset = 0;
            loop {
                let str = WideCStr::from_slice_with_nul(&data[offset..]).unwrap();
                if str.is_empty() {
                    break;
                }
                res.push(str.to_string_lossy());
                offset += str.len() + 1;
            }
        } else {
            let data = &buffer.as_slice()[files.p_files as usize..];
            let mut offset = 0;
            loop {
                let str = CStr::from_bytes_with_nul(&data[offset..]).unwrap();
                let bytes = str.to_bytes();
                if bytes.is_empty() {
                    break;
                }
                res.push(str.to_string_lossy().into());
                offset += bytes.len();
            }
        }
        res
    }

    pub fn extract_url_w(buffer: &[u8]) -> String {
        let data = buffer.as_slice_of::<u16>().unwrap();
        let str = WideCStr::from_slice_with_nul(data).unwrap();
        str.to_string_lossy()
    }

    pub fn extract_url(buffer: &[u8]) -> String {
        let str = CStr::from_bytes_with_nul(&buffer).unwrap();
        str.to_string_lossy().into()
    }

    pub fn bundle_files(files: &Vec<String>) -> Vec<u8> {
        let mut res = Vec::new();

        let drop_files = DROPFILES {
            p_files: size_of::<DROPFILES>() as u32,
            pt: POINT { x: 0, y: 0 },
            f_nc: FALSE,
            f_wide: TRUE,
        };

        let drop_files = unsafe { as_u8_slice(&drop_files) };
        res.extend_from_slice(drop_files);

        for f in files {
            let mut wide: Vec<u16> = f.encode_utf16().collect();
            wide.push(0);
            res.extend_from_slice(wide.as_byte_slice());
        }
        res.extend_from_slice(&[0, 0]);

        res
    }

    pub fn get_format(format: u32) -> FORMATETC {
        Self::get_format_with_tymed(format, TYMED::TYMED_HGLOBAL)
    }

    pub fn get_format_with_tymed(format: u32, tymed: TYMED) -> FORMATETC {
        FORMATETC {
            cf_format: format as u16,
            ptd: null_mut(),
            dw_aspect: DVASPECT::DVASPECT_CONTENT.0 as u32,
            lindex: -1,
            tymed: tymed.0 as u32,
        }
    }
}

#[repr(C)]
pub struct STGMEDIUM_ {
    pub tymed: u32,
    pub data: isize,
    pub p_unk_for_release: usize,
}

#[repr(C)]
pub struct STGMEDIUM_STREAM {
    pub tymed: u32,
    pub stream: Option<IStream>,
    pub p_unk_for_release: usize,
}

impl Default for STGMEDIUM_ {
    fn default() -> Self {
        Self {
            tymed: 0,
            data: 0,
            p_unk_for_release: 0,
        }
    }
}
