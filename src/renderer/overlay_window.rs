use std::mem::size_of;
use windows::core::{w, Result};
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct OverlayWindow {
    pub hwnd: HWND,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

unsafe extern "system" fn overlay_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

impl OverlayWindow {
    pub fn new() -> Result<Self> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class_name = w!("SnowfallDesktopOverlayClass");

            let wc = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(overlay_wndproc),
                hInstance: instance.into(),
                lpszClassName: class_name,
                hCursor: HCURSOR::default(),
                ..Default::default()
            };
            RegisterClassExW(&wc);

            let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let vcx = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let vcy = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST
                    | WS_EX_TRANSPARENT
                    | WS_EX_LAYERED
                    | WS_EX_NOACTIVATE
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOREDIRECTIONBITMAP,
                class_name,
                w!("SnowfallDesktopOverlay"),
                WS_POPUP | WS_VISIBLE,
                vx,
                vy,
                vcx,
                vcy,
                None,
                None,
                Some(instance.into()),
                None,
            )?;

            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                vx,
                vy,
                vcx,
                vcy,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );

            Ok(Self {
                hwnd,
                x: vx,
                y: vy,
                width: vcx,
                height: vcy,
            })
        }
    }

    pub fn update_bounds(&mut self) -> bool {
        unsafe {
            let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let vcx = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let vcy = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            // ONLY call SetWindowPos when virtual screen geometry actually changes!
            if vx != self.x || vy != self.y || vcx != self.width || vcy != self.height {
                self.x = vx;
                self.y = vy;
                self.width = vcx;
                self.height = vcy;

                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    vx,
                    vy,
                    vcx,
                    vcy,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
                return true;
            }

            false
        }
    }
}
