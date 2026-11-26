#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accumulation;
mod app;
mod config;
mod cursor;
mod particle;
mod performance;
mod physics;
mod renderer;
mod tractor;
mod ui;
mod weather;
mod window_tracker;

use app::App;
use windows::core::{w, Result};
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::WindowsAndMessaging::*;

fn main() -> Result<()> {
    unsafe {
        // 1. Single-instance protection
        let mutex_name = w!("Local\\Snowfall_Desktop_Simulator_SingleInstance");
        let mutex = CreateMutexW(None, true, mutex_name);
        if GetLastError() == ERROR_ALREADY_EXISTS {
            // If already running, find existing settings dialog and bring to focus
            let existing = FindWindowW(w!("SnowfallSettingsDialogClass"), None);
            if let Ok(h) = existing {
                if !h.0.is_null() {
                    let _ = ShowWindow(h, SW_SHOW);
                    let _ = SetForegroundWindow(h);
                }
            }
            return Ok(());
        }

        // 2. High-DPI Awareness (Per Monitor V2) for crisp rendering on 4K/retina displays
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        // 3. Initialize COM
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        // 4. Initialize Windows Common Controls (Trackbars, modern visual styles)
        let icce = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES | ICC_STANDARD_CLASSES,
        };
        let _ = InitCommonControlsEx(&icce);

        // 5. Run application
        let mut app = App::new()?;
        app.run()?;

        // Clean up mutex
        if let Ok(m) = mutex {
            let _ = CloseHandle(m);
        }
    }

    Ok(())
}
