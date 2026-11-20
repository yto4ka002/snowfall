use crate::accumulation::AccumulationSystem;
use crate::config::Settings;
use crate::cursor::CursorTracker;
use crate::particle::ParticleSystem;
use crate::performance::PerformanceManager;
use crate::physics::PhysicsEngine;
use crate::renderer::Renderer;
use crate::ui::settings_dlg::SettingsDialogHandle;
use crate::weather::WeatherSystem;
use crate::window_tracker::WindowTracker;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use windows::core::Result;
use windows::Win32::System::Threading::Sleep;
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct App {
    renderer: Renderer,
    particles: ParticleSystem,
    accumulation: AccumulationSystem,
    window_tracker: WindowTracker,
    weather: WeatherSystem,
    cursor: CursorTracker,
    perf: PerformanceManager,
    settings: Arc<Mutex<Settings>>,
    clear_snow_requested: Arc<AtomicBool>,
    exit_requested: Arc<AtomicBool>,
    #[allow(dead_code)]
    settings_dlg: SettingsDialogHandle,
    last_resize_check: Instant,
}

impl App {
    pub fn new() -> Result<Self> {
        let loaded_settings = Settings::load();
        let target_fps = loaded_settings.target_fps;
        let count = loaded_settings.snowflake_count;
        let max_accum = loaded_settings.max_accumulation_height;
        let accum_rate = loaded_settings.accumulation_rate;

        let settings = Arc::new(Mutex::new(loaded_settings));
        let clear_snow_requested = Arc::new(AtomicBool::new(false));
        let exit_requested = Arc::new(AtomicBool::new(false));

        let renderer = Renderer::new()?;
        let screen_w = renderer.overlay.width as f32;
        let screen_h = renderer.overlay.height as f32;

        let mut particles = ParticleSystem::new(count);
        particles.init_particles(screen_w, screen_h, count);

        let accumulation = AccumulationSystem::new(screen_w, screen_h, max_accum, accum_rate);
        let window_tracker = WindowTracker::new(renderer.overlay.hwnd);
        let weather = WeatherSystem::new();
        let cursor = CursorTracker::new();
        let perf = PerformanceManager::new(target_fps);

        let settings_dlg = SettingsDialogHandle::new(
            Arc::clone(&settings),
            Arc::clone(&clear_snow_requested),
            Arc::clone(&exit_requested),
        );

        Ok(Self {
            renderer,
            particles,
            accumulation,
            window_tracker,
            weather,
            cursor,
            perf,
            settings,
            clear_snow_requested,
            exit_requested,
            settings_dlg,
            last_resize_check: Instant::now(),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let mut msg = MSG::default();
        let mut running = true;
        let mut overlay_shown = true;

        while running {
            if self.exit_requested.load(Ordering::SeqCst) {
                break;
            }

            let current_settings = { self.settings.lock().unwrap().clone() };

            // When disabled: hide overlay and sleep at the OS level (0% CPU, 0% GPU, zero UI lag)
            if !current_settings.enabled {
                if overlay_shown {
                    unsafe {
                        let _ = ShowWindow(self.renderer.overlay.hwnd, SW_HIDE);
                    }
                    overlay_shown = false;
                }

                unsafe {
                    Sleep(50);
                }
                continue;
            }

            // Restore overlay when re-enabled
            if !overlay_shown {
                unsafe {
                    let _ = ShowWindow(self.renderer.overlay.hwnd, SW_SHOWNA);
                }
                overlay_shown = true;
                self.perf.begin_frame();
            }

            unsafe {
                while PeekMessageW(
                    &mut msg,
                    Some(self.renderer.overlay.hwnd),
                    0,
                    0,
                    PM_REMOVE,
                )
                .as_bool()
                {
                    if msg.message == WM_QUIT {
                        running = false;
                        break;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            if !running {
                break;
            }

            if self.clear_snow_requested.swap(false, Ordering::SeqCst) {
                self.accumulation.clear_all();
            }

            if self.particles.max_particles != current_settings.snowflake_count {
                let screen_w = self.renderer.overlay.width as f32;
                let screen_h = self.renderer.overlay.height as f32;
                self.particles.resize(current_settings.snowflake_count, screen_w, screen_h);
            }
            self.accumulation.set_params(
                current_settings.max_accumulation_height,
                current_settings.accumulation_rate,
            );
            self.perf.set_target_fps(current_settings.target_fps);

            self.perf.check_system_state(
                self.renderer.overlay.hwnd,
                current_settings.pause_on_fullscreen,
            );

            if self.perf.is_fullscreen_app_active || self.perf.is_screen_locked {
                self.renderer.context.begin_draw();
                let _ = self.renderer.context.end_draw_and_present(1);
                unsafe {
                    Sleep(200);
                }
                continue;
            }

            let dt = self.perf.begin_frame();

            // Check screen geometry only every 1.5 seconds to save CPU & avoid OS latency
            let now = Instant::now();
            if now.duration_since(self.last_resize_check).as_millis() >= 1500 {
                self.last_resize_check = now;
                let _ = self.renderer.check_resize();
            }

            let screen_w = self.renderer.overlay.width as f32;
            let screen_h = self.renderer.overlay.height as f32;

            self.weather.update(dt, current_settings.wind_intensity);
            self.cursor.update(dt);

            // Track window positions and bounds every frame for zero-lag motion
            self.window_tracker
                .update(current_settings.shake_sensitivity);

            let segments: Vec<(isize, f32, f32, f32)> = self
                .window_tracker
                .all_visible_surfaces
                .iter()
                .map(|s| (s.hwnd, s.x1, s.x2, s.y))
                .collect();
            self.accumulation.sync_with_windows(&segments);

            PhysicsEngine::update(
                &mut self.particles,
                &mut self.accumulation,
                &self.weather,
                &self.cursor,
                &mut self.window_tracker,
                &current_settings,
                screen_w,
                screen_h,
                dt,
            );

            let _ = self.renderer.render(
                &self.particles,
                &self.accumulation,
                &self.window_tracker,
                &current_settings,
            );

            self.perf.end_frame_and_throttle();
        }

        let s = self.settings.lock().unwrap();
        s.save();

        Ok(())
    }
}
