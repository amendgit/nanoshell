use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    mem::{self, forget},
    rc::{Rc, Weak},
    slice,
};

use windows::{create_instance, ErrorCode, IUnknown, Interface, TRUE};

use super::{
    all_bindings::*,
    drag_util::{CLSID_DragDropHelper, DataUtil, DROPEFFECT, STGMEDIUM_, STGMEDIUM_STREAM},
    util::{get_raw_ptr, ErrorCodeExt},
};

pub trait DropTargetDelegate {
    fn drag_enter(&self, object: IDataObject, pt: &POINTL, effect_mask: DROPEFFECT) -> DROPEFFECT;
    fn drag_over(&self, pt: &POINTL, effect_mask: DROPEFFECT) -> DROPEFFECT;
    fn drag_leave(&self);
    fn perform_drop(&self, object: IDataObject, pt: &POINTL, effect_mask: DROPEFFECT)
        -> DROPEFFECT;
}

#[repr(C)]
pub(super) struct DropTarget {
    _abi: Box<IDropTarget_abi>,
    ref_cnt: u32,
    drop_target_helper: IDropTargetHelper,
    hwnd: HWND,
    delegate: Weak<dyn DropTargetDelegate>,
}

#[allow(dead_code)]
impl DropTarget {
    pub fn new(hwnd: HWND, delegate: Weak<dyn DropTargetDelegate>) -> IDropTarget {
        let helper: IDropTargetHelper = create_instance(&CLSID_DragDropHelper).unwrap();
        let target = Box::new(Self {
            _abi: Box::new(IDropTarget_abi(
                Self::_query_interface,
                Self::_add_ref,
                Self::_release,
                Self::_drag_enter,
                Self::_drag_over,
                Self::_drag_leave,
                Self::_drop,
            )),
            ref_cnt: 1,
            drop_target_helper: helper,
            hwnd,
            delegate,
        });

        unsafe {
            let ptr = Box::into_raw(target);
            mem::transmute(ptr)
        }
    }

    fn query_interface(
        &mut self,
        iid: &::windows::Guid,
        interface: *mut ::windows::RawPtr,
    ) -> ErrorCode {
        if iid == &IDropTarget::IID || iid == &IUnknown::IID {
            unsafe {
                *interface = self as *mut Self as *mut _;
            }
            self.add_ref();
            ErrorCode(S_OK as u32)
        } else {
            ErrorCode(E_NOINTERFACE as u32)
        }
    }

    fn add_ref(&mut self) -> u32 {
        self.ref_cnt += 1;
        self.ref_cnt
    }

    fn release(&mut self) -> u32 {
        self.ref_cnt -= 1;
        let res = self.ref_cnt;

        if res == 0 {
            unsafe {
                Box::from_raw(self as *mut Self);
            }
        }

        res
    }

    fn drag_enter(
        &self,
        p_data_obj: ::std::option::Option<IDataObject>,
        _grf_key_state: u32,
        mut pt: POINTL,
        pdw_effect: *mut u32,
    ) -> ::windows::ErrorCode {
        unsafe {
            if let Some(delegate) = self.delegate.upgrade() {
                *pdw_effect = delegate
                    .drag_enter(p_data_obj.clone().unwrap(), &pt, DROPEFFECT(*pdw_effect))
                    .0;
            }

            self.drop_target_helper
                .DragEnter(
                    self.hwnd,
                    p_data_obj.clone(),
                    &mut pt as *mut POINTL as *mut _,
                    *pdw_effect,
                )
                .ok_log();

            p_data_obj.map(|o| forget(o));
        }
        ErrorCode(S_OK as u32)
    }

    fn drag_over(
        &self,
        _grf_key_state: u32,
        mut pt: POINTL,
        pdw_effect: *mut u32,
    ) -> ::windows::ErrorCode {
        unsafe {
            if let Some(delegate) = self.delegate.upgrade() {
                *pdw_effect = delegate.drag_over(&pt, DROPEFFECT(*pdw_effect)).0;
            }

            self.drop_target_helper
                .DragOver(&mut pt as *mut POINTL as *mut _, *pdw_effect)
                .ok_log();
        }
        ErrorCode(S_OK as u32)
    }

    fn drag_leave(&self) -> ::windows::ErrorCode {
        unsafe {
            if let Some(delegate) = self.delegate.upgrade() {
                delegate.drag_leave();
            }

            self.drop_target_helper.DragLeave().ok_log();
        }
        ErrorCode(S_OK as u32)
    }

    fn drop(
        &self,
        p_data_obj: ::std::option::Option<IDataObject>,
        _grf_key_state: u32,
        mut pt: POINTL,
        pdw_effect: *mut u32,
    ) -> ::windows::ErrorCode {
        unsafe {
            if let Some(delegate) = self.delegate.upgrade() {
                *pdw_effect = delegate
                    .perform_drop(p_data_obj.clone().unwrap(), &pt, DROPEFFECT(*pdw_effect))
                    .0;
            }

            self.drop_target_helper
                .Drop(
                    p_data_obj.clone(),
                    &mut pt as *mut POINTL as *mut _,
                    *pdw_effect,
                )
                .ok_log();
        }
        p_data_obj.map(|o| forget(o));
        ErrorCode(S_OK as u32)
    }

    unsafe extern "system" fn _query_interface(
        this: ::windows::RawPtr,
        iid: &::windows::Guid,
        interface: *mut ::windows::RawPtr,
    ) -> windows::ErrorCode {
        (*(this as *mut Self)).query_interface(iid, interface)
    }

    unsafe extern "system" fn _add_ref(this: ::windows::RawPtr) -> u32 {
        (*(this as *mut Self)).add_ref()
    }

    unsafe extern "system" fn _release(this: ::windows::RawPtr) -> u32 {
        (*(this as *mut Self)).release()
    }

    unsafe extern "system" fn _drag_enter(
        this: ::windows::RawPtr,
        p_data_obj: ::std::option::Option<IDataObject>,
        grf_key_state: u32,
        pt: POINTL,
        pdw_effect: *mut u32,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).drag_enter(p_data_obj, grf_key_state, pt, pdw_effect)
    }

    unsafe extern "system" fn _drag_over(
        this: ::windows::RawPtr,
        grf_key_state: u32,
        pt: POINTL,
        pdw_effect: *mut u32,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).drag_over(grf_key_state, pt, pdw_effect)
    }

    unsafe extern "system" fn _drag_leave(this: ::windows::RawPtr) -> ::windows::ErrorCode {
        (*(this as *mut Self)).drag_leave()
    }

    unsafe extern "system" fn _drop(
        this: ::windows::RawPtr,
        p_data_obj: ::std::option::Option<IDataObject>,
        grf_key_state: u32,
        pt: POINTL,
        pdw_effect: *mut u32,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).drop(p_data_obj, grf_key_state, pt, pdw_effect)
    }
}

//
//
//

struct EnumFORMATETC {
    _abi: Box<IEnumFORMATETC_abi>,
    ref_cnt: u32,
    formats: Vec<FORMATETC>,
    index: usize,
}

#[allow(dead_code)]
impl EnumFORMATETC {
    fn new_(formats: Vec<FORMATETC>, index: usize) -> IEnumFORMATETC {
        let target = Box::new(Self {
            _abi: Box::new(IEnumFORMATETC_abi(
                Self::_query_interface,
                Self::_add_ref,
                Self::_release,
                Self::_next,
                Self::_skip,
                Self::_reset,
                Self::_clone,
            )),
            ref_cnt: 1,
            formats,
            index,
        });

        unsafe {
            let ptr = Box::into_raw(target);
            mem::transmute(ptr)
        }
    }

    pub fn new(formats: Vec<FORMATETC>) -> IEnumFORMATETC {
        Self::new_(formats, 0)
    }

    fn query_interface(
        &mut self,
        iid: &::windows::Guid,
        interface: *mut ::windows::RawPtr,
    ) -> ErrorCode {
        if iid == &IEnumFORMATETC::IID || iid == &IUnknown::IID {
            unsafe {
                *interface = self as *mut Self as *mut _;
            }
            self.add_ref();
            ErrorCode(S_OK as u32)
        } else {
            ErrorCode(E_NOINTERFACE as u32)
        }
    }

    fn add_ref(&mut self) -> u32 {
        self.ref_cnt += 1;
        self.ref_cnt
    }

    fn release(&mut self) -> u32 {
        self.ref_cnt -= 1;
        let res = self.ref_cnt;

        if res == 0 {
            unsafe {
                Box::from_raw(self as *mut Self);
            }
        }

        res
    }

    fn remaining(&self) -> usize {
        self.formats.len() - self.index
    }

    fn next(
        &mut self,
        mut celt: u32,
        rgelt: *mut FORMATETC,
        pcelt_fetched: *mut u32,
    ) -> ::windows::ErrorCode {
        let mut offset = 0;
        let dest: &mut [FORMATETC] = unsafe { slice::from_raw_parts_mut(rgelt, celt as usize) };
        while celt > 0 && self.remaining() > 0 {
            dest[offset] = self.formats.get(self.index).unwrap().clone();

            celt -= 1;
            self.index += 1;
            offset += 1;
        }
        if pcelt_fetched != std::ptr::null_mut() {
            unsafe {
                *pcelt_fetched = offset as u32;
            }
        }
        if celt > 0 {
            ErrorCode(S_FALSE as u32)
        } else {
            ErrorCode(S_OK as u32)
        }
    }

    fn skip(&mut self, mut celt: u32) -> ::windows::ErrorCode {
        while celt > 0 && self.remaining() > 0 {
            celt -= 1;
            self.index += 1;
        }
        if celt > 0 {
            ErrorCode(S_FALSE as u32)
        } else {
            ErrorCode(S_OK as u32)
        }
    }

    fn reset(&mut self) -> ::windows::ErrorCode {
        self.index = 0;
        ErrorCode(S_OK as u32)
    }

    fn clone(&self, ppenum: *mut ::std::option::Option<IEnumFORMATETC>) -> ::windows::ErrorCode {
        let clone = EnumFORMATETC::new_(self.formats.clone(), self.index);
        unsafe {
            *ppenum = Some(clone);
        }
        ErrorCode(S_OK as u32)
    }

    unsafe extern "system" fn _query_interface(
        this: ::windows::RawPtr,
        iid: &::windows::Guid,
        interface: *mut ::windows::RawPtr,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).query_interface(iid, interface)
    }

    unsafe extern "system" fn _add_ref(this: ::windows::RawPtr) -> u32 {
        (*(this as *mut Self)).add_ref()
    }

    unsafe extern "system" fn _release(this: ::windows::RawPtr) -> u32 {
        (*(this as *mut Self)).release()
    }

    unsafe extern "system" fn _next(
        this: ::windows::RawPtr,
        celt: u32,
        rgelt: *mut FORMATETC,
        pcelt_fetched: *mut u32,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).next(celt, rgelt, pcelt_fetched)
    }

    unsafe extern "system" fn _skip(this: ::windows::RawPtr, celt: u32) -> ::windows::ErrorCode {
        (*(this as *mut Self)).skip(celt)
    }

    unsafe extern "system" fn _reset(this: ::windows::RawPtr) -> ::windows::ErrorCode {
        (*(this as *mut Self)).reset()
    }
    unsafe extern "system" fn _clone(
        this: ::windows::RawPtr,
        ppenum: *mut ::std::option::Option<IEnumFORMATETC>,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).clone(ppenum)
    }
}

//
// DataObject
//

pub struct DataObject {
    _abi: Box<IDataObject_abi>,
    ref_cnt: u32,
    data: Weak<RefCell<HashMap<u32, Vec<u8>>>>,
}

const DATA_E_FORMATETC: i32 = -2147221404 + 1;

#[allow(dead_code)]
impl DataObject {
    // Using weak reference just in case some other software keeps DragObject alive after drag is finished
    pub fn new(data: Rc<RefCell<HashMap<u32, Vec<u8>>>>) -> IDataObject {
        let target = Box::new(Self {
            _abi: Box::new(IDataObject_abi(
                Self::_query_interface,
                Self::_add_ref,
                Self::_release,
                Self::_get_data,
                Self::_get_data_here,
                Self::_query_get_data,
                Self::_get_canonical_format_etc,
                Self::_set_data,
                Self::_enum_format_etc,
                Self::_d_advise,
                Self::_d_unadvise,
                Self::_enum_d_advise,
            )),
            ref_cnt: 1,
            data: Rc::downgrade(&data),
        });

        unsafe {
            let ptr = Box::into_raw(target);
            mem::transmute(ptr)
        }
    }

    fn query_interface(
        &mut self,
        iid: &::windows::Guid,
        interface: *mut ::windows::RawPtr,
    ) -> ErrorCode {
        if iid == &IDataObject::IID || iid == &IUnknown::IID {
            unsafe {
                *interface = self as *mut Self as *mut _;
            }
            self.add_ref();
            ErrorCode(S_OK as u32)
        } else {
            ErrorCode(E_NOINTERFACE as u32)
        }
    }

    fn add_ref(&mut self) -> u32 {
        self.ref_cnt += 1;
        self.ref_cnt
    }

    fn release(&mut self) -> u32 {
        self.ref_cnt -= 1;
        let res = self.ref_cnt;

        if res == 0 {
            unsafe {
                Box::from_raw(self as *mut Self);
            }
        }

        res
    }

    fn with_data_or<F, R>(&self, callback: F, or: R) -> R
    where
        F: FnOnce(RefMut<HashMap<u32, Vec<u8>>>) -> R,
    {
        if let Some(data) = self.data.upgrade() {
            callback(data.as_ref().borrow_mut())
        } else {
            or
        }
    }

    fn get_data(
        &self,
        pformatetc_in: *mut FORMATETC,
        pmedium: *mut STGMEDIUM,
    ) -> ::windows::ErrorCode {
        let format = unsafe { &*pformatetc_in };

        // println!(
        //     "GET: {}, {}",
        //     clipboard_format_to_string(format.cf_format as u32),
        //     format.tymed
        // );

        self.with_data_or(
            |data| {
                if format.tymed == TYMED::TYMED_HGLOBAL.0 as u32 {
                    let data = data.get(&(format.cf_format as u32));
                    if let Some(data) = data {
                        let global = unsafe {
                            let global = GlobalAlloc(0, data.len());
                            let global_data = GlobalLock(global);
                            std::ptr::copy_nonoverlapping(
                                data.as_ptr(),
                                global_data as *mut u8,
                                data.len(),
                            );
                            GlobalUnlock(global);
                            global
                        };

                        let medium = STGMEDIUM_ {
                            tymed: TYMED::TYMED_HGLOBAL.0 as u32,
                            data: global as isize,
                            p_unk_for_release: 0,
                        };
                        let pmedium = pmedium as *mut STGMEDIUM_;
                        unsafe {
                            *pmedium = medium;
                        }

                        ErrorCode(S_OK as u32)
                    } else {
                        ErrorCode(DATA_E_FORMATETC as u32)
                    }
                } else if format.tymed == TYMED::TYMED_ISTREAM.0 as u32 {
                    unsafe {
                        let medium = &mut *(pmedium as *mut STGMEDIUM_);
                        let data = data.get(&(format.cf_format as u32));

                        if let Some(data) = data {
                            let stream = SHCreateMemStream(data.as_ptr(), data.len() as u32);
                            stream
                                .clone()
                                .unwrap()
                                .Seek(
                                    0,
                                    STREAM_SEEK::STREAM_SEEK_END.0 as u32,
                                    std::ptr::null_mut(),
                                )
                                .ok_log();
                            *medium = STGMEDIUM_ {
                                tymed: TYMED::TYMED_ISTREAM.0 as u32,
                                data: get_raw_ptr(&stream) as isize,
                                p_unk_for_release: 0,
                            };
                            forget(stream); // will be released through sgtmedium

                            ErrorCode(S_OK as u32)
                        } else {
                            ErrorCode(DATA_E_FORMATETC as u32)
                        }
                    }
                } else {
                    ErrorCode(DATA_E_FORMATETC as u32)
                }
            },
            ErrorCode(DATA_E_FORMATETC as u32),
        )
    }

    fn get_data_here(
        &self,
        _pformatetc: *mut FORMATETC,
        _pmedium: *mut STGMEDIUM,
    ) -> ::windows::ErrorCode {
        ErrorCode(DATA_E_FORMATETC as u32)
    }

    fn query_get_data(&self, pformatetc: *mut FORMATETC) -> ::windows::ErrorCode {
        // println!("QUERY GET DATA");

        self.with_data_or(
            |data| {
                let format = unsafe { &*pformatetc };
                if (format.tymed == TYMED::TYMED_HGLOBAL.0 as u32
                    || format.tymed == TYMED::TYMED_ISTREAM.0 as u32)
                    && data.contains_key(&(format.cf_format as u32))
                {
                    ErrorCode(S_OK as u32)
                } else {
                    ErrorCode(S_FALSE as u32)
                }
            },
            ErrorCode(S_FALSE as u32),
        )
    }

    fn get_canonical_format_etc(
        &self,
        _pformatect_in: *mut FORMATETC,
        _pformatetc_out: *mut FORMATETC,
    ) -> ::windows::ErrorCode {
        ErrorCode(E_NOTIMPL as u32)
    }

    fn set_data(
        &mut self,
        pformatetc: *mut FORMATETC,
        pmedium: *mut STGMEDIUM,
        f_release: ::windows::BOOL,
    ) -> ::windows::ErrorCode {
        let format = unsafe { &*pformatetc };

        // println!(
        //     "SET: {}, {}",
        //     clipboard_format_to_string(format.cf_format as u32),
        //     format.tymed
        // );

        self.with_data_or(
            |mut data| {
                if format.tymed == TYMED::TYMED_HGLOBAL.0 as u32 {
                    unsafe {
                        let medium = &*(pmedium as *mut STGMEDIUM_);
                        let size = GlobalSize(medium.data);
                        let global_data = GlobalLock(medium.data);

                        let v = slice::from_raw_parts(global_data as *const u8, size);
                        let global_data: Vec<u8> = v.into();

                        GlobalUnlock(medium.data);
                        data.insert(format.cf_format as u32, global_data);

                        if f_release == TRUE {
                            ReleaseStgMedium(pmedium);
                        }
                    }

                    ErrorCode(S_OK as u32)
                } else if format.tymed == TYMED::TYMED_ISTREAM.0 as u32 {
                    unsafe {
                        let medium = &*(pmedium as *mut STGMEDIUM_STREAM);

                        let mut stream_data = Vec::<u8>::new();
                        let mut buf: [u8; 4096] = [0; 4096];
                        if let Some(stream) = medium.stream.clone() {
                            loop {
                                let mut num_read: u32 = 0;
                                if !stream
                                    .Read(
                                        buf.as_mut_ptr() as *mut _,
                                        buf.len() as u32,
                                        &mut num_read as *mut _,
                                    )
                                    .is_ok()
                                {
                                    break;
                                }

                                if num_read == 0 {
                                    break;
                                }
                                stream_data.extend_from_slice(&buf[..num_read as usize]);
                            }
                        }

                        data.insert(format.cf_format as u32, stream_data);

                        if f_release == TRUE {
                            ReleaseStgMedium(pmedium);
                        }
                    }

                    ErrorCode(S_OK as u32)
                } else {
                    ErrorCode(DATA_E_FORMATETC as u32)
                }
            },
            ErrorCode(DATA_E_FORMATETC as u32),
        )
    }

    fn enum_format_etc(
        &self,
        dw_direction: u32,
        ppenum_format_etc: *mut ::std::option::Option<IEnumFORMATETC>,
    ) -> ::windows::ErrorCode {
        let mut formats = Vec::<FORMATETC>::new();

        self.with_data_or(
            |data| {
                if dw_direction == DATADIR::DATADIR_GET.0 as u32 {
                    for f in data.keys() {
                        formats.push(DataUtil::get_format_with_tymed(*f, TYMED::TYMED_HGLOBAL));
                        formats.push(DataUtil::get_format_with_tymed(*f, TYMED::TYMED_ISTREAM));
                    }
                }
                let enum_format = EnumFORMATETC::new(formats);
                unsafe {
                    *ppenum_format_etc = Some(enum_format);
                }
                ErrorCode(S_OK as u32)
            },
            ErrorCode(S_OK as u32),
        )
    }

    fn d_advise(
        &self,
        _pformatetc: *mut FORMATETC,
        _advf: u32,
        _p_adv_sink: ::std::option::Option<IAdviseSink>,
        _pdw_connection: *mut u32,
    ) -> ::windows::ErrorCode {
        ErrorCode(E_NOTIMPL as u32)
    }

    fn d_unadvise(&self, _dw_connection: u32) -> ::windows::ErrorCode {
        ErrorCode(E_NOTIMPL as u32)
    }

    fn enum_d_advise(
        &self,
        _ppenum_advise: *mut ::std::option::Option<IEnumSTATDATA>,
    ) -> ::windows::ErrorCode {
        ErrorCode(E_NOTIMPL as u32)
    }

    unsafe extern "system" fn _query_interface(
        this: ::windows::RawPtr,
        iid: &::windows::Guid,
        interface: *mut ::windows::RawPtr,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).query_interface(iid, interface)
    }

    unsafe extern "system" fn _add_ref(this: ::windows::RawPtr) -> u32 {
        (*(this as *mut Self)).add_ref()
    }

    unsafe extern "system" fn _release(this: ::windows::RawPtr) -> u32 {
        (*(this as *mut Self)).release()
    }

    unsafe extern "system" fn _get_data(
        this: ::windows::RawPtr,
        pformatetc_in: *mut FORMATETC,
        pmedium: *mut STGMEDIUM,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).get_data(pformatetc_in, pmedium)
    }

    unsafe extern "system" fn _get_data_here(
        this: ::windows::RawPtr,
        pformatetc: *mut FORMATETC,
        pmedium: *mut STGMEDIUM,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).get_data_here(pformatetc, pmedium)
    }

    unsafe extern "system" fn _query_get_data(
        this: ::windows::RawPtr,
        pformatetc: *mut FORMATETC,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).query_get_data(pformatetc)
    }

    unsafe extern "system" fn _get_canonical_format_etc(
        this: ::windows::RawPtr,
        pformatetc_in: *mut FORMATETC,
        pformatetc_out: *mut FORMATETC,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).get_canonical_format_etc(pformatetc_in, pformatetc_out)
    }

    unsafe extern "system" fn _set_data(
        this: ::windows::RawPtr,
        pformatetc: *mut FORMATETC,
        pmedium: *mut STGMEDIUM,
        f_release: ::windows::BOOL,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).set_data(pformatetc, pmedium, f_release)
    }

    unsafe extern "system" fn _enum_format_etc(
        this: ::windows::RawPtr,
        dw_direction: u32,
        ppenum_format_etc: *mut ::std::option::Option<IEnumFORMATETC>,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).enum_format_etc(dw_direction, ppenum_format_etc)
    }

    unsafe extern "system" fn _d_advise(
        this: ::windows::RawPtr,
        pformatetc: *mut FORMATETC,
        advf: u32,
        p_adv_sink: ::std::option::Option<IAdviseSink>,
        pdw_connection: *mut u32,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).d_advise(pformatetc, advf, p_adv_sink, pdw_connection)
    }

    pub unsafe extern "system" fn _d_unadvise(
        this: ::windows::RawPtr,
        dw_connection: u32,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).d_unadvise(dw_connection)
    }

    pub unsafe extern "system" fn _enum_d_advise(
        this: ::windows::RawPtr,
        ppenum_advise: *mut ::std::option::Option<IEnumSTATDATA>,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).enum_d_advise(ppenum_advise)
    }
}

//
//
//

pub struct DropSource {
    _abi: Box<IDropSource_abi>,
    ref_cnt: u32,
}

#[allow(dead_code)]
impl DropSource {
    pub fn new() -> IDropSource {
        let target = Box::new(Self {
            _abi: Box::new(IDropSource_abi(
                Self::_query_interface,
                Self::_add_ref,
                Self::_release,
                Self::_query_continue_drag,
                Self::_give_feedback,
            )),
            ref_cnt: 1,
        });

        unsafe {
            let ptr = Box::into_raw(target);
            mem::transmute(ptr)
        }
    }

    fn query_interface(
        &mut self,
        iid: &::windows::Guid,
        interface: *mut ::windows::RawPtr,
    ) -> ErrorCode {
        if iid == &IDropSource::IID || iid == &IUnknown::IID {
            unsafe {
                *interface = self as *mut Self as *mut _;
            }
            self.add_ref();
            ErrorCode(S_OK as u32)
        } else {
            ErrorCode(E_NOINTERFACE as u32)
        }
    }

    fn add_ref(&mut self) -> u32 {
        self.ref_cnt += 1;
        self.ref_cnt
    }

    fn release(&mut self) -> u32 {
        self.ref_cnt -= 1;
        let res = self.ref_cnt;

        if res == 0 {
            unsafe {
                Box::from_raw(self as *mut Self);
            }
        }

        res
    }

    fn query_continue_drag(
        &self,
        f_escape_pressed: ::windows::BOOL,
        grf_key_state: u32,
    ) -> ::windows::ErrorCode {
        if f_escape_pressed == TRUE {
            ErrorCode(DRAGDROP_S_CANCEL as u32)
        } else if grf_key_state & MK_LBUTTON as u32 == 0 {
            ErrorCode(DRAGDROP_S_DROP as u32)
        } else {
            ErrorCode(S_OK as u32)
        }
    }

    fn give_feedback(&self, _dw_effect: u32) -> ::windows::ErrorCode {
        ErrorCode(DRAGDROP_S_USEDEFAULTCURSORS as u32)
    }

    unsafe extern "system" fn _query_interface(
        this: ::windows::RawPtr,
        iid: &::windows::Guid,
        interface: *mut ::windows::RawPtr,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).query_interface(iid, interface)
    }

    unsafe extern "system" fn _add_ref(this: ::windows::RawPtr) -> u32 {
        (*(this as *mut Self)).add_ref()
    }

    unsafe extern "system" fn _release(this: ::windows::RawPtr) -> u32 {
        (*(this as *mut Self)).release()
    }

    unsafe extern "system" fn _query_continue_drag(
        this: ::windows::RawPtr,
        f_escape_pressed: ::windows::BOOL,
        grf_key_state: u32,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).query_continue_drag(f_escape_pressed, grf_key_state)
    }

    unsafe extern "system" fn _give_feedback(
        this: ::windows::RawPtr,
        dw_effect: u32,
    ) -> ::windows::ErrorCode {
        (*(this as *mut Self)).give_feedback(dw_effect)
    }
}
