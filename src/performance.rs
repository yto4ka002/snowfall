use std::time::Instant;
use windows::Win32::Foundation::*;
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct PerformanceManager {
    freq: i64,
    last_frame_counter: i64,
    target_frame_time_sec: f64,
    pub delta_time: f32,
    pub is_fullscreen_app_active: bool,
    pub is_screen_locked: bool,
    last_fullscreen_check: Instant,
}

impl PerformanceManager {
    pub fn new(target_fps: u32) -> Self {
        let mut freq = 0i64;
        let mut counter = 0i64;
        unsafe {
            let _ = QueryPerformanceFrequency(&mut freq);
            let _ = QueryPerformanceCounter(&mut counter);
        }
        let fps = if target_fps == 30 { 30.0 } else { 60.0 };
        Self {
            freq,
            last_frame_counter: counter,
            target_frame_time_sec: 1.0 / fps,
            delta_time: (1.0 / fps) as f32,
            is_fullscreen_app_active: false,
            is_screen_locked: false,
            last_fullscreen_check: Instant::now() - std::time::Duration::from_secs(10),
        }
    }

    pub fn set_target_fps(&mut self, fps: u32) {
        let fps_f = if fps == 30 { 30.0 } else { 60.0 };
        self.target_frame_time_sec = 1.0 / fps_f;
    }

    pub fn check_system_state(&mut self, our_hwnd: HWND, pause_on_fullscreen: bool) {
        let now = Instant::now();
        if now.duration_since(self.last_fullscreen_check).as_millis() < 300 {
            return;
        }
        self.last_fullscreen_check = now;

        unsafe {
            // Check if desktop is locked / switched
            // If GetForegroundWindow returns HWND(0) or Desktop window when locked
            let fg_hwnd = GetForegroundWindow();
            if fg_hwnd.0.is_null() {
                self.is_screen_locked = false;
                self.is_fullscreen_app_active = false;
                return;
            }

            if fg_hwnd == our_hwnd {
                self.is_fullscreen_app_active = false;
                return;
            }

            // Check if foreground window is fullscreen
            if pause_on_fullscreen {
                let mut rect = RECT::default();
                if GetWindowRect(fg_hwnd, &mut rect).is_ok() {
                    let screen_w = GetSystemMetrics(SM_CXSCREEN);
                    let screen_h = GetSystemMetrics(SM_CYSCREEN);

                    let class_name_buf = &mut [0u16; 64];
                    let len = GetClassNameW(fg_hwnd, class_name_buf);
                    let class_name = String::from_utf16_lossy(&class_name_buf[..len as usize]);

                    // Ignore desktop and shell windows
                    if class_name != "Progman" && class_name != "WorkerW" && class_name != "Shell_TrayWnd" {
                        let win_w = rect.right - rect.left;
                        let win_h = rect.bottom - rect.top;
                        if rect.left <= 0 && rect.top <= 0 && win_w >= screen_w && win_h >= screen_h {
                            self.is_fullscreen_app_active = true;
                            return;
                        }
                    }
                }
            }
            self.is_fullscreen_app_active = false;
        }
    }

    pub fn begin_frame(&mut self) -> f32 {
        let mut current_counter = 0i64;
        unsafe {
            let _ = QueryPerformanceCounter(&mut current_counter);
        }

        let elapsed = (current_counter - self.last_frame_counter) as f64 / self.freq as f64;
        self.delta_time = elapsed.clamp(0.002, 0.05) as f32;
        self.last_frame_counter = current_counter;
        self.delta_time
    }

    pub fn end_frame_and_throttle(&mut self) {
        // DirectComposition Present(sync_interval, 0) manages hardware VSync pacing directly with DWM
    }
}
