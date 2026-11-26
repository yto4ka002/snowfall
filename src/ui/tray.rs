use std::mem::size_of;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_TRAYICON: u32 = WM_USER + 101;
pub const ID_TRAY_TOGGLE_ENABLED: usize = 1001;
pub const ID_TRAY_CLEAR_SNOW: usize = 1002;
pub const ID_TRAY_SETTINGS: usize = 1003;
pub const ID_TRAY_EXIT: usize = 1004;

pub struct TrayIcon {
    pub hwnd: HWND,
    nid: NOTIFYICONDATAW,
    pub hicon: HICON,
}

impl TrayIcon {
    pub fn new(hwnd: HWND) -> Self {
        unsafe {
            let hicon = Self::create_snowflake_hicon();

            let mut tip = [0u16; 128];
            let text = "Snowfall — Desktop Snow Simulator\0".encode_utf16().collect::<Vec<_>>();
            let len = text.len().min(127);
            tip[..len].copy_from_slice(&text[..len]);

            let mut nid = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: WM_TRAYICON,
                hIcon: hicon,
                szTip: tip,
                ..Default::default()
            };

            let _ = Shell_NotifyIconW(NIM_ADD, &mut nid);

            Self { hwnd, nid, hicon }
        }
    }

    pub fn show_menu(&self, enabled: bool) {
        unsafe {
            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);

            let menu = CreatePopupMenu().unwrap();

            let enabled_text = if enabled {
                w!("Остановить снегопад")
            } else {
                w!("Запустить снегопад")
            };

            let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_SETTINGS, w!("Окно управления Snowfall"));
            let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_TOGGLE_ENABLED, enabled_text);
            let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_CLEAR_SNOW, w!("Смахнуть снег с окон"));
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_EXIT, w!("Закрыть программу (Выход)"));

            let _ = SetForegroundWindow(self.hwnd);
            let _ = TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
                pt.x,
                pt.y,
                None,
                self.hwnd,
                None,
            );
            let _ = DestroyMenu(menu);
        }
    }

    unsafe fn create_snowflake_hicon() -> HICON {
        let size = 32;
        let mut color_pixels = vec![0u32; size * size];
        let mut mask_pixels = vec![0xFFu8; (size * size / 8) as usize];

        let center = (size as f32 - 1.0) * 0.5;
        let radius = center * 0.85;

        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= radius {
                    let angle = dy.atan2(dx);
                    let pi = std::f32::consts::PI;
                    let sector_angle = (angle % (pi / 3.0) + pi / 3.0) % (pi / 3.0);
                    let local_angle = (sector_angle - pi / 6.0).abs();
                    let d_spine = dist * local_angle.sin();

                    let mut lit = false;
                    if d_spine < 1.4 {
                        lit = true;
                    }
                    if dist > radius * 0.4 && dist < radius * 0.75 && d_spine < 3.0 {
                        lit = true;
                    }
                    if dist < 3.0 {
                        lit = true;
                    }

                    if lit {
                        let idx = y * size + x;
                        color_pixels[idx] = 0xFFEEF6FF;

                        let bit_idx = y * size + x;
                        let byte_idx = bit_idx / 8;
                        let bit_offset = 7 - (bit_idx % 8);
                        mask_pixels[byte_idx] &= !(1 << bit_offset);
                    }
                }
            }
        }

        let hbm_color = CreateBitmap(
            size as i32,
            size as i32,
            1,
            32,
            Some(color_pixels.as_ptr() as *const _),
        );
        let hbm_mask = CreateBitmap(
            size as i32,
            size as i32,
            1,
            1,
            Some(mask_pixels.as_ptr() as *const _),
        );

        let mut icon_info = ICONINFO {
            fIcon: TRUE,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hbm_mask,
            hbmColor: hbm_color,
        };

        let hicon = CreateIconIndirect(&mut icon_info).unwrap_or_default();

        let _ = DeleteObject(hbm_color.into());
        let _ = DeleteObject(hbm_mask.into());

        hicon
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &mut self.nid);
            if !self.hicon.0.is_null() {
                let _ = DestroyIcon(self.hicon);
            }
        }
    }
}
