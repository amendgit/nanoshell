use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    shell::{
        structs::{
            WindowFrame, WindowGeometry, WindowGeometryFlags, WindowGeometryRequest, WindowStyle,
        },
        IPoint, IRect, ISize, Point, Rect, Size,
    },
    util::OkLog,
};

use super::{
    all_bindings::*,
    display::Displays,
    error::PlatformResult,
    flutter_api::{FlutterDesktopGetDpiForHWND, FlutterDesktopGetDpiForMonitor},
    util::{clamp, BoolResultExt, ErrorCodeExt, GET_X_LPARAM, GET_Y_LPARAM},
};

pub struct WindowBaseState {
    hwnd: HWND,
    min_frame_size: RefCell<Size>,
    max_frame_size: RefCell<Size>,
    min_content_size: RefCell<Size>,
    max_content_size: RefCell<Size>,
    delegate: Weak<dyn WindowDelegate>,
    style: RefCell<WindowStyle>,
}

const LARGE_SIZE: f64 = 64.0 * 1024.0;

impl WindowBaseState {
    pub fn new(hwnd: HWND, delegate: Weak<dyn WindowDelegate>) -> Self {
        Self {
            hwnd,
            delegate,
            min_frame_size: RefCell::new(Size::wh(0.0, 0.0)),
            max_frame_size: RefCell::new(Size::wh(LARGE_SIZE, LARGE_SIZE)),
            min_content_size: RefCell::new(Size::wh(0.0, 0.0)),
            max_content_size: RefCell::new(Size::wh(LARGE_SIZE, LARGE_SIZE)),
            style: Default::default(),
        }
    }

    pub fn hide(&self) -> PlatformResult<()> {
        unsafe { ShowWindow(self.hwnd, SW_HIDE).as_platform_result() }
    }

    pub fn show<F>(&self, callback: F) -> PlatformResult<()>
    where
        F: FnOnce() -> () + 'static,
    {
        unsafe {
            ShowWindow(self.hwnd, SW_SHOW); // false is not an error
        }
        callback();
        Ok(())
    }

    pub fn set_geometry(
        &self,
        geometry: WindowGeometryRequest,
    ) -> PlatformResult<WindowGeometryFlags> {
        let geometry = geometry.filtered_by_preference();

        let mut res = WindowGeometryFlags {
            ..Default::default()
        };

        if geometry.content_origin.is_some()
            || geometry.content_size.is_some()
            || geometry.frame_origin.is_some()
            || geometry.frame_size.is_some()
        {
            self.set_bounds_geometry(&geometry, &mut res)?;

            // There's no set_content_rect in winapi, so this is best effort implementation
            // that tries to deduce future content rect from current content rect and frame rect
            // in case it's wrong (i.e. display with different DPI or frame size change after reposition)
            // it will retry once again
            if res.content_origin || res.content_size {
                let content_rect = self.content_rect_for_frame_rect(&self.get_frame_rect()?)?;
                if (res.content_origin
                    && content_rect.origin() != *geometry.content_origin.as_ref().unwrap())
                    || (res.content_size
                        && content_rect.size() != *geometry.content_size.as_ref().unwrap())
                {
                    // retry
                    self.set_bounds_geometry(&geometry, &mut res)?;
                }
            }
        }

        if let Some(size) = geometry.min_frame_size {
            self.min_frame_size.replace(size);
            res.min_frame_size = true;
        }

        if let Some(size) = geometry.max_frame_size {
            self.max_frame_size.replace(size);
            res.max_frame_size = true;
        }

        if let Some(size) = geometry.min_content_size {
            self.min_content_size.replace(size);
            res.min_content_size = true;
        }

        if let Some(size) = geometry.max_content_size {
            self.max_content_size.replace(size);
            res.max_content_size = true;
        }

        Ok(res)
    }

    fn set_bounds_geometry(
        &self,
        geometry: &WindowGeometry,
        flags: &mut WindowGeometryFlags,
    ) -> PlatformResult<()> {
        let current_frame_rect = self.get_frame_rect()?;
        let current_content_rect = self.content_rect_for_frame_rect(&current_frame_rect)?;

        let content_offset = current_content_rect.to_local(&current_frame_rect.origin());
        let content_size_delta = current_frame_rect.size() - current_content_rect.size();

        let mut origin: Option<Point> = None;
        let mut size: Option<Size> = None;

        if let Some(frame_origin) = &geometry.frame_origin {
            origin.replace(frame_origin.clone());
            flags.frame_origin = true;
        }

        if let Some(frame_size) = &geometry.frame_size {
            size.replace(frame_size.clone());
            flags.frame_size = true;
        }

        if let Some(content_origin) = &geometry.content_origin {
            origin.replace(content_origin.translated(&content_offset));
            flags.content_origin = true;
        }

        if let Some(content_size) = &geometry.content_size {
            size.replace(content_size + &content_size_delta);
            flags.content_size = true;
        }

        let physical = IRect::origin_size(
            &self.to_physical(origin.as_ref().unwrap_or(&Point::xy(0.0, 0.0))),
            &size
                .as_ref()
                .unwrap_or(&Size::wh(0.0, 0.0))
                .scaled(self.get_scaling_factor())
                .into(),
        );

        let mut flags = SWP_NOZORDER | SWP_NOACTIVATE;
        if origin.is_none() {
            flags |= SWP_NOMOVE;
        }
        if size.is_none() {
            flags |= SWP_NOSIZE;
        }
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND(0),
                physical.x,
                physical.y,
                physical.width,
                physical.height,
                flags as u32,
            )
            .as_platform_result()
        }
    }

    pub fn get_geometry(&self) -> PlatformResult<WindowGeometry> {
        let frame_rect = self.get_frame_rect()?;
        let content_rect = self.content_rect_for_frame_rect(&frame_rect)?;

        Ok(WindowGeometry {
            frame_origin: Some(frame_rect.origin()),
            frame_size: Some(frame_rect.size()),
            content_origin: Some(content_rect.origin()),
            content_size: Some(content_rect.size()),
            min_frame_size: Some(self.min_frame_size.borrow().clone()),
            max_frame_size: Some(self.max_frame_size.borrow().clone()),
            min_content_size: Some(self.min_content_size.borrow().clone()),
            max_content_size: Some(self.max_content_size.borrow().clone()),
        })
    }

    pub fn supported_geometry(&self) -> PlatformResult<WindowGeometryFlags> {
        Ok(WindowGeometryFlags {
            frame_origin: true,
            frame_size: true,
            content_origin: true,
            content_size: true,
            min_frame_size: true,
            max_frame_size: true,
            min_content_size: true,
            max_content_size: true,
        })
    }

    fn get_frame_rect(&self) -> PlatformResult<Rect> {
        let mut rect: RECT = Default::default();
        unsafe {
            GetWindowRect(self.hwnd, &mut rect as *mut _).as_platform_result()?;
        }
        let size: Size = ISize::wh(rect.right - rect.left, rect.bottom - rect.top).into();
        Ok(Rect::origin_size(
            &self.to_logical(&IPoint::xy(rect.left, rect.top)),
            &size.scaled(1.0 / self.get_scaling_factor()),
        ))
    }

    fn content_rect_for_frame_rect(&self, frame_rect: &Rect) -> PlatformResult<Rect> {
        let content_rect = IRect::origin_size(
            &self.to_physical(&frame_rect.top_left()),
            &frame_rect.size().scaled(self.get_scaling_factor()).into(),
        );
        let rect = RECT {
            left: content_rect.x,
            top: content_rect.y,
            right: content_rect.x2(),
            bottom: content_rect.y2(),
        };
        unsafe {
            SendMessageW(
                self.hwnd,
                WM_NCCALCSIZE as u32,
                WPARAM(FALSE.0 as usize),
                LPARAM(&rect as *const _ as isize),
            );
        }
        let size: Size = ISize::wh(rect.right - rect.left, rect.bottom - rect.top).into();
        Ok(Rect::origin_size(
            &self.to_logical(&IPoint::xy(rect.left, rect.top)),
            &size.scaled(1.0 / self.get_scaling_factor()),
        ))
    }

    fn adjust_window_position(&self, position: &mut WINDOWPOS) -> PlatformResult<()> {
        let scale = self.get_scaling_factor();
        let frame_rect = self.get_frame_rect()?;
        let content_rect = self.content_rect_for_frame_rect(&frame_rect)?;

        let size_delta = frame_rect.size() - content_rect.size();

        let min_content = &*self.min_content_size.borrow() + &size_delta;
        let min_content: ISize = min_content.scaled(scale).into();

        let min_frame = self.min_frame_size.borrow();
        let min_frame: ISize = min_frame.scaled(scale).into();

        let min_size = ISize::wh(
            std::cmp::max(min_content.width, min_frame.width),
            std::cmp::max(min_content.height, min_frame.height),
        );

        let max_content = &*self.max_content_size.borrow() + &size_delta;
        let max_content: ISize = max_content.scaled(scale).into();

        let max_frame = self.max_frame_size.borrow();
        let max_frame: ISize = max_frame.scaled(scale).into();

        let max_size = ISize::wh(
            std::cmp::min(max_content.width, max_frame.width),
            std::cmp::min(max_content.height, max_frame.height),
        );

        position.cx = clamp(position.cx, min_size.width, max_size.width);
        position.cy = clamp(position.cy, min_size.height, max_size.height);

        Ok(())
    }

    pub fn close(&self) -> PlatformResult<()> {
        unsafe { DestroyWindow(self.hwnd).as_platform_result() }
    }

    pub fn local_to_global(&self, offset: &Point) -> IPoint {
        let scaled: IPoint = offset.scaled(self.get_scaling_factor()).into();
        return self.local_to_global_physical(&scaled);
    }

    pub fn local_to_global_physical(&self, offset: &IPoint) -> IPoint {
        let mut point = POINT {
            x: offset.x,
            y: offset.y,
        };
        unsafe {
            ClientToScreen(self.hwnd, &mut point as *mut _);
        }
        IPoint::xy(point.x, point.y)
    }

    pub fn global_to_local(&self, offset: &IPoint) -> Point {
        let local: Point = self.global_to_local_physical(&offset).into();
        local.scaled(1.0 / self.get_scaling_factor())
    }

    pub fn global_to_local_physical(&self, offset: &IPoint) -> IPoint {
        let mut point = POINT {
            x: offset.x,
            y: offset.y,
        };
        unsafe {
            ScreenToClient(self.hwnd, &mut point as *mut _);
        }
        IPoint::xy(point.x, point.y)
    }

    fn to_physical(&self, offset: &Point) -> IPoint {
        Displays::get_displays()
            .convert_logical_to_physical(offset)
            .unwrap_or(offset.clone().into())
    }

    fn to_logical(&self, offset: &IPoint) -> Point {
        Displays::get_displays()
            .convert_physical_to_logical(offset)
            .unwrap_or(offset.clone().into())
    }

    pub fn is_rtl(&self) -> bool {
        let style = unsafe { GetWindowLongW(self.hwnd, GWL_EXSTYLE) };
        return style & WS_EX_LAYOUTRTL == WS_EX_LAYOUTRTL;
    }

    pub fn get_scaling_factor(&self) -> f64 {
        unsafe { FlutterDesktopGetDpiForHWND(self.hwnd) as f64 / 96.0 }
    }

    fn get_scaling_factor_for_monitor(&self, monitor: isize) -> f64 {
        unsafe { FlutterDesktopGetDpiForMonitor(monitor) as f64 / 96.0 }
    }

    fn delegate(&self) -> Rc<dyn WindowDelegate> {
        // delegate owns us so unwrap is safe here
        self.delegate.upgrade().unwrap()
    }

    unsafe fn set_close_enabled(&self, enabled: bool) {
        let menu = GetSystemMenu(self.hwnd, FALSE);
        if enabled {
            EnableMenuItem(menu, SC_CLOSE as u32, (MF_BYCOMMAND | MFS_ENABLED) as u32);
        } else {
            EnableMenuItem(
                menu,
                SC_CLOSE as u32,
                (MF_BYCOMMAND | MFS_DISABLED | MF_GRAYED) as u32,
            );
        }
    }

    pub fn update_dwm_frame(&self) -> PlatformResult<()> {
        let margin = match self.style.borrow().frame {
            WindowFrame::Regular => 0, // already has shadow
            WindowFrame::NoTitle => 1, // neede for window shadow
            WindowFrame::NoFrame => 0, // neede for transparency
        };

        let margins = MARGINS {
            cx_left_width: 0,
            cx_right_width: 0,
            cy_top_height: margin,
            cy_bottom_height: 0,
        };
        unsafe {
            DwmExtendFrameIntoClientArea(self.hwnd, &margins as *const _).as_platform_result()
        }
    }

    pub fn set_style(&self, style: WindowStyle) -> PlatformResult<()> {
        *self.style.borrow_mut() = style.clone();
        unsafe {
            let mut s = GetWindowLongW(self.hwnd, GWL_STYLE) as u32;
            s &= !(WS_OVERLAPPEDWINDOW | WS_DLGFRAME);

            if style.frame == WindowFrame::Regular {
                s |= WS_CAPTION;
                if style.can_resize {
                    s |= WS_THICKFRAME;
                }
            }

            if style.frame == WindowFrame::NoTitle {
                s |= WS_CAPTION;
                if style.can_resize {
                    s |= WS_THICKFRAME;
                } else {
                    s |= WS_BORDER;
                }
            }

            if style.frame == WindowFrame::NoFrame {
                s |= WS_POPUP
            }

            s |= WS_SYSMENU;
            self.set_close_enabled(style.can_close);
            if style.can_maximize {
                s |= WS_MAXIMIZEBOX;
            }
            if style.can_minimize {
                s |= WS_MINIMIZEBOX;
            }

            SetWindowLongW(self.hwnd, GWL_STYLE, s as i32);
            SetWindowPos(
                self.hwnd,
                HWND(0),
                0,
                0,
                0,
                0,
                (SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER) as u32,
            )
            .as_platform_result()?;

            self.update_dwm_frame()?;
        }
        Ok(())
    }

    pub fn perform_window_drag(&self) -> PlatformResult<()> {
        unsafe {
            println!("Perform window drag!");
            ReleaseCapture();
            SendMessageW(
                self.hwnd,
                WM_NCLBUTTONDOWN as u32,
                WPARAM(HTCAPTION as usize),
                LPARAM(0),
            );
        }
        Ok(())
    }

    pub fn has_redirection_surface(&self) -> bool {
        let style = unsafe { GetWindowLongW(self.hwnd, GWL_EXSTYLE) };
        return style & WS_EX_NOREDIRECTIONBITMAP == 0;
    }

    pub fn remove_border(&self) -> bool {
        self.style.borrow().frame == WindowFrame::NoTitle
    }

    fn do_hit_test(&self, x: i32, y: i32) -> i32 {
        let mut win_rect = RECT::default();
        unsafe {
            GetWindowRect(self.hwnd, &mut win_rect as *mut _);
        }

        let border_width = (7.0 * self.get_scaling_factor()) as i32;

        if x < win_rect.left + border_width && y < win_rect.top + border_width {
            HTTOPLEFT
        } else if x > win_rect.right - border_width && y < win_rect.top + border_width {
            HTTOPRIGHT
        } else if y < win_rect.top + border_width {
            HTTOP
        } else if x < win_rect.left + border_width && y > win_rect.bottom - border_width {
            HTBOTTOMLEFT
        } else if x > win_rect.right - border_width && y > win_rect.bottom - border_width {
            HTBOTTOMRIGHT
        } else if y > win_rect.bottom - border_width {
            HTBOTTOM
        } else if x < win_rect.left + border_width {
            HTLEFT
        } else if x > win_rect.right - border_width {
            HTRIGHT
        } else {
            HTCLIENT
        }
    }

    pub fn handle_message(
        &self,
        _h_wnd: HWND,
        msg: u32,
        _w_param: WPARAM,
        l_param: LPARAM,
    ) -> Option<LRESULT> {
        match msg as i32 {
            WM_CLOSE => {
                self.delegate().should_close();
                Some(LRESULT(0))
            }
            WM_DESTROY => {
                self.delegate().will_close();
                None
            }
            WM_DISPLAYCHANGE => {
                Displays::displays_changed();
                self.delegate().displays_changed();
                None
            }
            WM_WINDOWPOSCHANGING => {
                let position = unsafe { &mut *(l_param.0 as *mut WINDOWPOS) };
                self.adjust_window_position(position).ok_log();
                None
            }
            WM_DWMCOMPOSITIONCHANGED => {
                self.update_dwm_frame().ok_log();
                None
            }
            WM_NCCALCSIZE => {
                if self.remove_border() {
                    Some(LRESULT(1))
                } else {
                    None
                }
            }
            WM_NCHITTEST => {
                if self.remove_border() {
                    let res = self.do_hit_test(GET_X_LPARAM(l_param), GET_Y_LPARAM(l_param));
                    Some(LRESULT(res))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn handle_child_message(
        &self,
        _h_wnd: HWND,
        msg: u32,
        _w_param: WPARAM,
        l_param: LPARAM,
    ) -> Option<LRESULT> {
        match msg as i32 {
            WM_NCHITTEST => {
                if self.remove_border() {
                    let res = self.do_hit_test(GET_X_LPARAM(l_param), GET_Y_LPARAM(l_param));
                    if res != HTCLIENT {
                        Some(LRESULT(HTTRANSPARENT))
                    } else {
                        Some(LRESULT(res))
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

pub trait WindowDelegate {
    fn should_close(&self);
    fn will_close(&self);
    fn displays_changed(&self);
}
