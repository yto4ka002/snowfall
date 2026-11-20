use crate::config::Settings;
use crate::ui::tray::{
    TrayIcon, ID_TRAY_CLEAR_SNOW, ID_TRAY_EXIT, ID_TRAY_SETTINGS, ID_TRAY_TOGGLE_ENABLED,
    WM_TRAYICON,
};
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, Mutex};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

// Control IDs
pub const ID_BTN_TOGGLE_RUNNING: usize = 101;
pub const ID_BTN_EXIT_APP: usize = 102;
pub const ID_LABEL_STATUS: usize = 103;

pub const ID_SLIDER_COUNT: usize = 201;
pub const ID_LABEL_COUNT: usize = 202;
pub const ID_SLIDER_SPEED: usize = 203;
pub const ID_LABEL_SPEED: usize = 204;
pub const ID_SLIDER_WIND: usize = 205;
pub const ID_LABEL_WIND: usize = 206;
pub const ID_SLIDER_DEPTH: usize = 207;
pub const ID_LABEL_DEPTH: usize = 208;
pub const ID_SLIDER_SHAKE: usize = 209;
pub const ID_LABEL_SHAKE: usize = 210;

pub const ID_CHECK_ACCUMULATION: usize = 221;
pub const ID_CHECK_CURSOR: usize = 222;
pub const ID_CHECK_FULLSCREEN: usize = 223;
pub const ID_CHECK_AUTOSTART: usize = 224;
pub const ID_CHECK_SNOW_OVER_WINDOWS: usize = 225;

pub const ID_RADIO_60FPS: usize = 230;
pub const ID_RADIO_30FPS: usize = 231;

pub const ID_BTN_CLEAR_SNOW: usize = 240;
pub const ID_BTN_RESET_DEFAULTS: usize = 241;
pub const ID_BTN_MINIMIZE_TRAY: usize = 242;

// Trackbar messages
const TBM_SETRANGE: u32 = WM_USER + 6;
const TBM_SETPOS: u32 = WM_USER + 5;
const TBM_GETPOS: u32 = WM_USER;

const BST_UNCHECKED: usize = 0;
const BST_CHECKED: usize = 1;

pub struct SettingsDialog {
    pub hwnd: HWND,
    pub settings: Arc<Mutex<Settings>>,
    pub clear_snow_requested: Arc<AtomicBool>,
    pub exit_requested: Arc<AtomicBool>,
    pub tray: Option<TrayIcon>,
    font: HFONT,
    title_font: HFONT,
}

static mut DIALOG_PTR: *mut SettingsDialog = std::ptr::null_mut();

unsafe extern "system" fn settings_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let mut ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsDialog;
    if ptr.is_null() {
        ptr = DIALOG_PTR;
    }
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let dlg = &mut *ptr;

    match msg {
        WM_TRAYICON => {
            let event = (lparam.0 & 0xFFFF) as u32;
            if event == WM_RBUTTONUP {
                let enabled = dlg.settings.lock().unwrap().enabled;
                if let Some(tray) = &dlg.tray {
                    tray.show_menu(enabled);
                }
            } else if event == WM_LBUTTONUP || event == WM_LBUTTONDBLCLK {
                dlg.show();
            }
            LRESULT(0)
        }
        WM_HSCROLL => {
            dlg.handle_slider_change();
            LRESULT(0)
        }
        WM_COMMAND => {
            let ctrl_id = (wparam.0 & 0xFFFF) as usize;
            match ctrl_id {
                ID_BTN_TOGGLE_RUNNING | ID_TRAY_TOGGLE_ENABLED => {
                    {
                        let mut s = dlg.settings.lock().unwrap();
                        s.enabled = !s.enabled;
                    }
                    dlg.sync_controls_from_settings();
                }
                ID_BTN_EXIT_APP | ID_TRAY_EXIT => {
                    {
                        let s = dlg.settings.lock().unwrap();
                        s.save();
                    }
                    dlg.exit_requested.store(true, Ordering::SeqCst);
                    PostQuitMessage(0);
                }
                ID_TRAY_SETTINGS => {
                    dlg.show();
                }
                ID_BTN_CLEAR_SNOW | ID_TRAY_CLEAR_SNOW => {
                    dlg.clear_snow_requested.store(true, Ordering::SeqCst);
                }
                ID_CHECK_ACCUMULATION
                | ID_CHECK_SNOW_OVER_WINDOWS
                | ID_CHECK_CURSOR
                | ID_CHECK_FULLSCREEN
                | ID_CHECK_AUTOSTART
                | ID_RADIO_60FPS
                | ID_RADIO_30FPS => {
                    dlg.handle_checkbox_change();
                }
                ID_BTN_RESET_DEFAULTS => {
                    dlg.reset_to_defaults();
                }
                ID_BTN_MINIMIZE_TRAY => {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                    let s = dlg.settings.lock().unwrap();
                    s.save();
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = ShowWindow(hwnd, SW_HIDE);
            let s = dlg.settings.lock().unwrap();
            s.save();
            LRESULT(0)
        }
        WM_DESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DIALOG_PTR = std::ptr::null_mut();
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC => {
            let hdc = HDC(wparam.0 as *mut _);
            SetBkMode(hdc, TRANSPARENT);
            LRESULT(GetSysColorBrush(COLOR_BTNFACE).0 as isize)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[allow(dead_code)]
pub struct SettingsDialogHandle {
    pub exit_requested: Arc<AtomicBool>,
    hwnd_store: Arc<AtomicIsize>,
}

unsafe impl Send for SettingsDialogHandle {}
unsafe impl Sync for SettingsDialogHandle {}

#[allow(dead_code)]
impl SettingsDialogHandle {
    pub fn new(
        settings: Arc<Mutex<Settings>>,
        clear_snow: Arc<AtomicBool>,
        exit_requested: Arc<AtomicBool>,
    ) -> Self {
        let hwnd_store = Arc::new(AtomicIsize::new(0));
        let hwnd_store_clone = Arc::clone(&hwnd_store);
        let settings_clone = Arc::clone(&settings);
        let clear_snow_clone = Arc::clone(&clear_snow);
        let exit_requested_clone = Arc::clone(&exit_requested);

        std::thread::Builder::new()
            .name("SnowfallUIThread".into())
            .spawn(move || {
                unsafe {
                    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
                    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

                    let instance = GetModuleHandleW(None).unwrap();
                    let class_name = w!("SnowfallSettingsDialogClass");

                    let font = CreateFontW(
                        -13,
                        0,
                        0,
                        0,
                        FW_NORMAL.0 as i32,
                        0,
                        0,
                        0,
                        DEFAULT_CHARSET,
                        OUT_DEFAULT_PRECIS,
                        CLIP_DEFAULT_PRECIS,
                        CLEARTYPE_QUALITY,
                        DEFAULT_PITCH.0 as u32 | (FF_DONTCARE.0 as u32),
                        w!("Segoe UI"),
                    );

                    let title_font = CreateFontW(
                        -15,
                        0,
                        0,
                        0,
                        FW_SEMIBOLD.0 as i32,
                        0,
                        0,
                        0,
                        DEFAULT_CHARSET,
                        OUT_DEFAULT_PRECIS,
                        CLIP_DEFAULT_PRECIS,
                        CLEARTYPE_QUALITY,
                        DEFAULT_PITCH.0 as u32 | (FF_DONTCARE.0 as u32),
                        w!("Segoe UI"),
                    );

                    let wc = WNDCLASSEXW {
                        cbSize: size_of::<WNDCLASSEXW>() as u32,
                        lpfnWndProc: Some(settings_wndproc),
                        hInstance: instance.into(),
                        lpszClassName: class_name,
                        hbrBackground: HBRUSH((COLOR_BTNFACE.0 + 1) as isize as *mut _),
                        hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
                        ..Default::default()
                    };
                    RegisterClassExW(&wc);

                    let sw = GetSystemMetrics(SM_CXSCREEN);
                    let sh = GetSystemMetrics(SM_CYSCREEN);
                    let win_w = 475;
                    let win_h = 660;
                    let win_x = (sw - win_w) / 2;
                    let win_y = (sh - win_h) / 2;

                    let hwnd = CreateWindowExW(
                        WINDOW_EX_STYLE(0),
                        class_name,
                        w!("Snowfall — Управление снегопадом"),
                        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
                        win_x,
                        win_y,
                        win_w,
                        win_h,
                        None,
                        None,
                        Some(instance.into()),
                        None,
                    )
                    .unwrap();

                    hwnd_store_clone.store(hwnd.0 as isize, Ordering::SeqCst);

                    let tray = Some(TrayIcon::new(hwnd));

                    let boxed = Box::new(SettingsDialog {
                        hwnd,
                        settings: settings_clone,
                        clear_snow_requested: clear_snow_clone,
                        exit_requested: exit_requested_clone,
                        tray,
                        font,
                        title_font,
                    });

                    let raw_ptr = Box::into_raw(boxed);
                    DIALOG_PTR = raw_ptr;
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);

                    (*raw_ptr).create_child_controls(instance.into());
                    (*raw_ptr).sync_controls_from_settings();
                    (*raw_ptr).show();

                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            })
            .expect("Failed to spawn UI thread");

        Self {
            exit_requested,
            hwnd_store,
        }
    }

    pub fn show(&self) {
        unsafe {
            let raw = self.hwnd_store.load(Ordering::SeqCst);
            if raw != 0 {
                let h = HWND(raw as *mut _);
                let _ = ShowWindow(h, SW_SHOW);
                let _ = SetForegroundWindow(h);
            }
        }
    }
}

impl SettingsDialog {
    pub fn show(&self) {
        unsafe {
            self.sync_controls_from_settings();
            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let _ = SetForegroundWindow(self.hwnd);
        }
    }

    unsafe fn create_child_controls(&self, instance: HINSTANCE) {
        let h = self.hwnd;

        // 1. Status Indicator
        let h_status = self.create_label(h, instance, w!("Статус: ● Снегопад активен"), 20, 15, 435, 22, ID_LABEL_STATUS);
        let _ = SendMessageW(h_status, WM_SETFONT, Some(WPARAM(self.title_font.0 as usize)), Some(LPARAM(1)));

        // 2. Primary Action Buttons: Start/Stop & Exit
        self.create_button(h, instance, w!("⏸ Остановить снегопад"), 20, 42, 230, 40, ID_BTN_TOGGLE_RUNNING);
        self.create_button(h, instance, w!("⏻ Закрыть программу"), 258, 42, 192, 40, ID_BTN_EXIT_APP);

        // 3. Quick Actions
        self.create_button(h, instance, w!("🧹 Смахнуть снег с окон"), 20, 90, 220, 28, ID_BTN_CLEAR_SNOW);
        self.create_button(h, instance, w!("↺ Сбросить настройки"), 248, 90, 202, 28, ID_BTN_RESET_DEFAULTS);

        // 4. Sliders (start at y=130)
        self.create_label(h, instance, w!("Количество снежинок:"), 20, 130, 220, 18, 0);
        self.create_label(h, instance, w!("450"), 365, 130, 85, 18, ID_LABEL_COUNT);
        self.create_slider(h, instance, 20, 149, 430, 26, ID_SLIDER_COUNT);

        self.create_label(h, instance, w!("Скорость падения:"), 20, 180, 220, 18, 0);
        self.create_label(h, instance, w!("1.0x"), 365, 180, 85, 18, ID_LABEL_SPEED);
        self.create_slider(h, instance, 20, 199, 430, 26, ID_SLIDER_SPEED);

        self.create_label(h, instance, w!("Сила ветра:"), 20, 230, 220, 18, 0);
        self.create_label(h, instance, w!("1.0x"), 365, 230, 85, 18, ID_LABEL_WIND);
        self.create_slider(h, instance, 20, 249, 430, 26, ID_SLIDER_WIND);

        self.create_label(h, instance, w!("Толщина снежной шапки на окнах:"), 20, 280, 260, 18, 0);
        self.create_label(h, instance, w!("24 px"), 365, 280, 85, 18, ID_LABEL_DEPTH);
        self.create_slider(h, instance, 20, 299, 430, 26, ID_SLIDER_DEPTH);

        self.create_label(h, instance, w!("Чувствительность стряхивания окон:"), 20, 330, 260, 18, 0);
        self.create_label(h, instance, w!("Обычная"), 365, 330, 85, 18, ID_LABEL_SHAKE);
        self.create_slider(h, instance, 20, 349, 430, 26, ID_SLIDER_SHAKE);

        // 5. Checkboxes (start at y=388)
        self.create_checkbox(h, instance, w!("Снегопад поверх окон приложений"), 25, 388, 380, 22, ID_CHECK_SNOW_OVER_WINDOWS);
        self.create_checkbox(h, instance, w!("Накапливать снежные шапки на окнах"), 25, 412, 380, 22, ID_CHECK_ACCUMULATION);
        self.create_checkbox(h, instance, w!("Реакция снега на курсор мыши (завихрения)"), 25, 436, 380, 22, ID_CHECK_CURSOR);
        self.create_checkbox(h, instance, w!("Пауза в полноэкранных играх и видео"), 25, 460, 380, 22, ID_CHECK_FULLSCREEN);
        self.create_checkbox(h, instance, w!("Запускать автоматически с Windows"), 25, 484, 380, 22, ID_CHECK_AUTOSTART);

        // 6. Frame Rate (y=516)
        self.create_label(h, instance, w!("Режим плавности:"), 25, 518, 140, 20, 0);
        self.create_radio(h, instance, w!("60 FPS (Плавно)"), 170, 516, 125, 22, ID_RADIO_60FPS);
        self.create_radio(h, instance, w!("30 FPS (Экономия)"), 300, 516, 140, 22, ID_RADIO_30FPS);

        // 7. Bottom Tray Button (y=556)
        self.create_button(h, instance, w!("Свернуть окно в системный трей (фоновый режим)"), 20, 556, 430, 34, ID_BTN_MINIMIZE_TRAY);
    }

    unsafe fn create_label(
        &self,
        parent: HWND,
        instance: HINSTANCE,
        text: PCWSTR,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        id: usize,
    ) -> HWND {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            text,
            WS_CHILD | WS_VISIBLE,
            x,
            y,
            w,
            h,
            Some(parent),
            Some(HMENU(id as *mut _)),
            Some(instance),
            None,
        )
        .unwrap();
        let _ = SendMessageW(
            hwnd,
            WM_SETFONT,
            Some(WPARAM(self.font.0 as usize)),
            Some(LPARAM(1)),
        );
        hwnd
    }

    unsafe fn create_slider(
        &self,
        parent: HWND,
        instance: HINSTANCE,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        id: usize,
    ) -> HWND {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("msctls_trackbar32"),
            PCWSTR::null(),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(0x0001),
            x,
            y,
            w,
            h,
            Some(parent),
            Some(HMENU(id as *mut _)),
            Some(instance),
            None,
        )
        .unwrap();
        hwnd
    }

    unsafe fn create_checkbox(
        &self,
        parent: HWND,
        instance: HINSTANCE,
        text: PCWSTR,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        id: usize,
    ) -> HWND {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            text,
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            x,
            y,
            w,
            h,
            Some(parent),
            Some(HMENU(id as *mut _)),
            Some(instance),
            None,
        )
        .unwrap();
        let _ = SendMessageW(
            hwnd,
            WM_SETFONT,
            Some(WPARAM(self.font.0 as usize)),
            Some(LPARAM(1)),
        );
        hwnd
    }

    unsafe fn create_radio(
        &self,
        parent: HWND,
        instance: HINSTANCE,
        text: PCWSTR,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        id: usize,
    ) -> HWND {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            text,
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTORADIOBUTTON as u32),
            x,
            y,
            w,
            h,
            Some(parent),
            Some(HMENU(id as *mut _)),
            Some(instance),
            None,
        )
        .unwrap();
        let _ = SendMessageW(
            hwnd,
            WM_SETFONT,
            Some(WPARAM(self.font.0 as usize)),
            Some(LPARAM(1)),
        );
        hwnd
    }

    unsafe fn create_button(
        &self,
        parent: HWND,
        instance: HINSTANCE,
        text: PCWSTR,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        id: usize,
    ) -> HWND {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            text,
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
            x,
            y,
            w,
            h,
            Some(parent),
            Some(HMENU(id as *mut _)),
            Some(instance),
            None,
        )
        .unwrap();
        let _ = SendMessageW(
            hwnd,
            WM_SETFONT,
            Some(WPARAM(self.font.0 as usize)),
            Some(LPARAM(1)),
        );
        hwnd
    }

    pub fn sync_controls_from_settings(&self) {
        unsafe {
            let s = self.settings.lock().unwrap();

            let status_text = if s.enabled {
                "Статус: ● Снегопад активен (работает поверх окон)"
            } else {
                "Статус: ○ Снегопад остановлен"
            };
            self.set_label_text(ID_LABEL_STATUS, status_text);

            let btn_text = if s.enabled {
                "⏸ Остановить снегопад"
            } else {
                "▶ Запустить снегопад"
            };
            self.set_button_text(ID_BTN_TOGGLE_RUNNING, btn_text);

            if let Ok(h_count) = GetDlgItem(Some(self.hwnd), ID_SLIDER_COUNT as i32) {
                let _ = SendMessageW(
                    h_count,
                    TBM_SETRANGE,
                    Some(WPARAM(1)),
                    Some(LPARAM((50 | (1500 << 16)) as isize)),
                );
                let _ = SendMessageW(
                    h_count,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(s.snowflake_count as isize)),
                );
            }
            self.set_label_text(ID_LABEL_COUNT, &format!("{}", s.snowflake_count));

            if let Ok(h_speed) = GetDlgItem(Some(self.hwnd), ID_SLIDER_SPEED as i32) {
                let _ = SendMessageW(
                    h_speed,
                    TBM_SETRANGE,
                    Some(WPARAM(1)),
                    Some(LPARAM((2 | (30 << 16)) as isize)),
                );
                let _ = SendMessageW(
                    h_speed,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM((s.fall_speed * 10.0) as isize)),
                );
            }
            self.set_label_text(ID_LABEL_SPEED, &format!("{:.1}x", s.fall_speed));

            if let Ok(h_wind) = GetDlgItem(Some(self.hwnd), ID_SLIDER_WIND as i32) {
                let _ = SendMessageW(
                    h_wind,
                    TBM_SETRANGE,
                    Some(WPARAM(1)),
                    Some(LPARAM((0 | (30 << 16)) as isize)),
                );
                let _ = SendMessageW(
                    h_wind,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM((s.wind_intensity * 10.0) as isize)),
                );
            }
            self.set_label_text(ID_LABEL_WIND, &format!("{:.1}x", s.wind_intensity));

            if let Ok(h_depth) = GetDlgItem(Some(self.hwnd), ID_SLIDER_DEPTH as i32) {
                let _ = SendMessageW(
                    h_depth,
                    TBM_SETRANGE,
                    Some(WPARAM(1)),
                    Some(LPARAM((8 | (60 << 16)) as isize)),
                );
                let _ = SendMessageW(
                    h_depth,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(s.max_accumulation_height as isize)),
                );
            }
            self.set_label_text(ID_LABEL_DEPTH, &format!("{:.0} px", s.max_accumulation_height));

            if let Ok(h_shake) = GetDlgItem(Some(self.hwnd), ID_SLIDER_SHAKE as i32) {
                let _ = SendMessageW(
                    h_shake,
                    TBM_SETRANGE,
                    Some(WPARAM(1)),
                    Some(LPARAM((2 | (30 << 16)) as isize)),
                );
                let _ = SendMessageW(
                    h_shake,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM((s.shake_sensitivity * 10.0) as isize)),
                );
            }
            let shake_str = if s.shake_sensitivity < 0.6 {
                "Слабая"
            } else if s.shake_sensitivity > 1.4 {
                "Высокая"
            } else {
                "Обычная"
            };
            self.set_label_text(ID_LABEL_SHAKE, shake_str);

            self.set_check(ID_CHECK_SNOW_OVER_WINDOWS, s.snow_over_windows);
            self.set_check(ID_CHECK_ACCUMULATION, s.snow_accumulation);
            self.set_check(ID_CHECK_CURSOR, s.cursor_interaction);
            self.set_check(ID_CHECK_FULLSCREEN, s.pause_on_fullscreen);
            self.set_check(ID_CHECK_AUTOSTART, s.auto_start);

            self.set_check(ID_RADIO_60FPS, s.target_fps == 60);
            self.set_check(ID_RADIO_30FPS, s.target_fps == 30);
        }
    }

    unsafe fn set_check(&self, id: usize, checked: bool) {
        if let Ok(h) = GetDlgItem(Some(self.hwnd), id as i32) {
            let val = if checked { BST_CHECKED } else { BST_UNCHECKED };
            let _ = SendMessageW(h, BM_SETCHECK, Some(WPARAM(val)), None);
        }
    }

    unsafe fn get_check(&self, id: usize) -> bool {
        if let Ok(h) = GetDlgItem(Some(self.hwnd), id as i32) {
            SendMessageW(h, BM_GETCHECK, None, None).0 == BST_CHECKED as isize
        } else {
            false
        }
    }

    unsafe fn set_label_text(&self, id: usize, text: &str) {
        if let Ok(h) = GetDlgItem(Some(self.hwnd), id as i32) {
            let wide = text.encode_utf16().chain(std::iter::once(0)).collect::<Vec<_>>();
            let _ = SetWindowTextW(h, PCWSTR(wide.as_ptr()));
        }
    }

    unsafe fn set_button_text(&self, id: usize, text: &str) {
        if let Ok(h) = GetDlgItem(Some(self.hwnd), id as i32) {
            let wide = text.encode_utf16().chain(std::iter::once(0)).collect::<Vec<_>>();
            let _ = SetWindowTextW(h, PCWSTR(wide.as_ptr()));
        }
    }

    fn handle_slider_change(&self) {
        unsafe {
            let mut s = self.settings.lock().unwrap();

            if let Ok(h) = GetDlgItem(Some(self.hwnd), ID_SLIDER_COUNT as i32) {
                let val = SendMessageW(h, TBM_GETPOS, None, None).0 as usize;
                s.snowflake_count = val;
                self.set_label_text(ID_LABEL_COUNT, &format!("{}", val));
            }

            if let Ok(h) = GetDlgItem(Some(self.hwnd), ID_SLIDER_SPEED as i32) {
                let val = SendMessageW(h, TBM_GETPOS, None, None).0 as f32 / 10.0;
                s.fall_speed = val;
                self.set_label_text(ID_LABEL_SPEED, &format!("{:.1}x", val));
            }

            if let Ok(h) = GetDlgItem(Some(self.hwnd), ID_SLIDER_WIND as i32) {
                let val = SendMessageW(h, TBM_GETPOS, None, None).0 as f32 / 10.0;
                s.wind_intensity = val;
                self.set_label_text(ID_LABEL_WIND, &format!("{:.1}x", val));
            }

            if let Ok(h) = GetDlgItem(Some(self.hwnd), ID_SLIDER_DEPTH as i32) {
                let val = SendMessageW(h, TBM_GETPOS, None, None).0 as f32;
                s.max_accumulation_height = val;
                self.set_label_text(ID_LABEL_DEPTH, &format!("{:.0} px", val));
            }

            if let Ok(h) = GetDlgItem(Some(self.hwnd), ID_SLIDER_SHAKE as i32) {
                let val = SendMessageW(h, TBM_GETPOS, None, None).0 as f32 / 10.0;
                s.shake_sensitivity = val;
                let shake_str = if val < 0.6 {
                    "Слабая"
                } else if val > 1.4 {
                    "Высокая"
                } else {
                    "Обычная"
                };
                self.set_label_text(ID_LABEL_SHAKE, shake_str);
            }
        }
    }

    fn handle_checkbox_change(&self) {
        unsafe {
            let mut s = self.settings.lock().unwrap();
            s.snow_over_windows = self.get_check(ID_CHECK_SNOW_OVER_WINDOWS);
            s.snow_accumulation = self.get_check(ID_CHECK_ACCUMULATION);
            s.cursor_interaction = self.get_check(ID_CHECK_CURSOR);
            s.pause_on_fullscreen = self.get_check(ID_CHECK_FULLSCREEN);
            s.auto_start = self.get_check(ID_CHECK_AUTOSTART);

            if self.get_check(ID_RADIO_30FPS) {
                s.target_fps = 30;
            } else {
                s.target_fps = 60;
            }
        }
    }

    fn reset_to_defaults(&self) {
        {
            let mut s = self.settings.lock().unwrap();
            *s = Settings::default();
        }
        self.sync_controls_from_settings();
    }
}
