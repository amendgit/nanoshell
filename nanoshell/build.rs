fn main() -> () {
    #[cfg(target_os = "windows")]
    {
        windows::build!(
            windows::win32::system_services::{
                // Methods
                CreateEventW, SetEvent, WaitForSingleObject,
                MsgWaitForMultipleObjects, LoadLibraryW,
                FreeLibrary, GetProcAddress, GetModuleHandleW, GetCurrentThreadId,
                GlobalSize, GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock, LocalFree,
                // Constants
                S_OK, S_FALSE, E_NOINTERFACE, E_NOTIMPL,
                QS_ALLINPUT, PM_REMOVE, TRUE, FALSE, CS_HREDRAW, CS_VREDRAW,
                WS_POPUP, WS_THICKFRAME, WS_OVERLAPPEDWINDOW, GWL_EXSTYLE, WS_SYSMENU, WS_DISABLED, WS_CAPTION,
                WS_POPUPWINDOW, WS_CLIPCHILDREN, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_BORDER,
                GWLP_USERDATA, SW_SHOW, SW_HIDE, GWL_STYLE,
                IDC_ARROW, SIZE_RESTORED,
                SWP_NOZORDER, SWP_NOSIZE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_FRAMECHANGED,
                WS_VISIBLE, HTNOWHERE, WS_EX_APPWINDOW, WS_DLGFRAME, WS_EX_NOACTIVATE, WS_EX_LAYOUTRTL, WS_EX_RTLREADING,
                WS_EX_NOREDIRECTIONBITMAP,
                FLASHW_ALL, SC_CLOSE, MF_BYCOMMAND, MF_BYPOSITION, MF_GRAYED, MF_DISABLED, MF_POPUP, GWL_HWNDPARENT, MF_HILITE, MF_MOUSESELECT,
                MIM_MENUDATA, MIM_STYLE, MNS_NOTIFYBYPOS, MIM_BACKGROUND,
                MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING, MIIM_SUBMENU,
                MFT_STRING, MFT_SEPARATOR, MFS_DISABLED, MFS_CHECKED, MFS_ENABLED, MFT_OWNERDRAW, MFS_HILITE,
                WH_MSGFILTER, MSGF_MENU,
                UIS_CLEAR, UIS_SET, UISF_HIDEACCEL, VK_LBUTTON, VK_RBUTTON, VK_LEFT, VK_RIGHT, VK_DOWN, VK_SHIFT,
                CF_HDROP, MK_LBUTTON,
                DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS,
                RDW_FRAME,
                HTCLIENT, HTCAPTION, HTTOPLEFT, HTTOPRIGHT, HTTOP, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTBOTTOM,
                HTLEFT, HTRIGHT, HTTRANSPARENT,
                DCX_WINDOW, DCX_INTERSECTRGN, FACILITY_WIN32,
                // Messages
                WM_DPICHANGED, WM_DESTROY, WM_SIZE, WM_ACTIVATE, WM_NCCREATE, WM_NCDESTROY, WM_ENTERMENULOOP,
                WM_QUIT, WM_DISPLAYCHANGE, WM_SHOWWINDOW, WM_CLOSE, WM_PAINT, WM_GETMINMAXINFO,
                WM_WINDOWPOSCHANGING, WM_NCCALCSIZE, WM_MOUSEMOVE, WM_NCMOUSEMOVE, WM_NCHITTEST, WM_NCMOUSEHOVER, WM_NCPAINT,
                WM_MOUSEFIRST, WM_MOUSELAST, WM_LBUTTONDOWN, WM_RBUTTONDOWN, WM_MBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONUP,
                WM_MBUTTONUP, WM_XBUTTONUP,
                WM_TIMER, WM_MENUCOMMAND, WM_COMMAND, WM_USER, WM_MOUSELEAVE, WM_CANCELMODE, WM_MENUSELECT,
                WM_CHANGEUISTATE, WM_UPDATEUISTATE, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYUP, WM_SETFOCUS, WM_DWMCOMPOSITIONCHANGED,
                WM_NCLBUTTONDOWN, WM_ERASEBKGND, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE,
                WM_QUERYUISTATE, WM_SYSCOMMAND,
                TPM_LEFTALIGN, TPM_RIGHTALIGN, TPM_TOPALIGN, TPM_BOTTOMALIGN, TPM_HORIZONTAL, TPM_VERTICAL,
                TPM_RETURNCMD,
                TME_LEAVE,
            },
            windows::win32::gdi::{
                EnumDisplayMonitors, ClientToScreen, ScreenToClient, CreateSolidBrush, GetDC, ReleaseDC,
                CreateDIBSection, DeleteObject, RedrawWindow, GetDCEx, ExcludeClipRect,
                FillRect, PAINTSTRUCT, BeginPaint, EndPaint,
            },
            windows::win32::menus_and_resources::{
                LoadCursorW, GetSystemMenu, EnableMenuItem, CreatePopupMenu, DestroyMenu, AppendMenuW,
                TrackPopupMenuEx, InsertMenuItemW, RemoveMenu, SetMenuItemInfoW, SetMenuInfo, GetMenuInfo,
                GetMenuItemInfoW, GetCursorPos, EndMenu, GetSubMenu, GetMenuItemCount, HiliteMenuItem,
            },
            windows::win32::keyboard_and_mouse_input::{
                SetFocus, EnableWindow, IsWindowEnabled, SetActiveWindow, ReleaseCapture, SetCapture,
                GetCapture, GetAsyncKeyState, GetKeyboardState, GetKeyState, TrackMouseEvent, ToUnicode,
            },
            windows::win32::debug::{
                IsDebuggerPresent, FlashWindowEx, GetLastError, FormatMessageW,
            },
            windows::win32::dwm:: {
                DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DwmFlush,
                DWMWINDOWATTRIBUTE, DWMNCRENDERINGPOLICY,
            },
            windows::win32::windows_programming::CloseHandle,
            windows::win32::windows_and_messaging::{
                // Methods
                RegisterClassW, UnregisterClassW, PostMessageW, SendMessageW,
                GetMessageW, PeekMessageW, TranslateMessage, DispatchMessageW, DestroyWindow, CreateWindowExW,
                DefWindowProcW, SetWindowLongW, GetWindowLongW, ShowWindow, SetProcessDPIAware,
                SetWindowPos, GetWindowRect, GetClientRect, SetParent, GetParent, MoveWindow, SetForegroundWindow,
                SetTimer, SetWindowsHookExW, UnhookWindowsHookEx, CallNextHookEx, FindWindowW,
                GetGUIThreadInfo, WindowFromPoint,
                // Structures
                CREATESTRUCTW, MSG, WINDOWPOS, NCCALCSIZE_PARAMS
            },
            windows::win32::display_devices::{
                POINTL
            },
            windows::win32::structured_storage::{
                IStream, STREAM_SEEK,
            },
            windows::win32::shell::{
                SetWindowSubclass, RemoveWindowSubclass, DefSubclassProc, IDropTargetHelper, IDragSourceHelper,
                DragQueryFileW, DROPFILES, SHCreateMemStream,
            },
            windows::win32::com::{
                CoInitializeEx, CoInitializeSecurity, CoUninitialize, COINIT,
                IDataObject, IDropSource, IDropTarget, RevokeDragDrop, OleInitialize, DVASPECT, TYMED,
                ReleaseStgMedium, DATADIR, EOLE_AUTHENTICATION_CAPABILITIES,
            },
            windows::win32::data_exchange::{
                RegisterClipboardFormatW, GetClipboardFormatNameW
            },
            windows::win32::dxgi::{IDXGIDevice, IDXGIFactory, IDXGIFactory2},
            windows::win32::hi_dpi::EnableNonClientDpiScaling
        );

        // cargo_emit::rustc_link_lib! {
        // "flutter_windows.dll",
        // }
    }
}
