use std::collections::{HashMap, VecDeque};
use std::time::Instant;
use windows::core::BOOL;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Clone, Copy, Debug)]
pub struct SurfaceSegment {
    pub hwnd: isize,
    pub x1: f32,
    pub x2: f32,
    pub y: f32,
}

#[derive(Clone, Debug)]
pub struct WindowRecord {
    pub hwnd: isize,
    pub rect: RECT,
    pub delta_x: f32,
    pub delta_y: f32,
    pub is_moving: bool,
    pub shake_triggered: bool,
    pub shake_intensity: f32,
    pub shake_velocity_x: f32,
    pub shake_velocity_y: f32,
    pub visible_segments: Vec<SurfaceSegment>,

    recent_samples: VecDeque<(Instant, f32, f32)>,
    last_shake_time: Instant,
    pub updated_this_tick: bool,
}

impl WindowRecord {
    pub fn new(hwnd: isize, rect: RECT) -> Self {
        let mut samples = VecDeque::with_capacity(16);
        samples.push_back((Instant::now(), rect.left as f32, rect.top as f32));
        Self {
            hwnd,
            rect,
            delta_x: 0.0,
            delta_y: 0.0,
            is_moving: false,
            shake_triggered: false,
            shake_intensity: 0.0,
            shake_velocity_x: 0.0,
            shake_velocity_y: 0.0,
            visible_segments: Vec::new(),
            recent_samples: samples,
            last_shake_time: Instant::now() - std::time::Duration::from_secs(5),
            updated_this_tick: true,
        }
    }

    pub fn update_position(&mut self, new_rect: RECT, sensitivity: f32) {
        let now = Instant::now();
        let cur_x = new_rect.left as f32;
        let cur_y = new_rect.top as f32;

        self.delta_x = cur_x - self.rect.left as f32;
        self.delta_y = cur_y - self.rect.top as f32;
        self.rect = new_rect;
        self.is_moving = self.delta_x.abs() > 0.5 || self.delta_y.abs() > 0.5;

        self.recent_samples.push_back((now, cur_x, cur_y));

        while let Some(&(t, _, _)) = self.recent_samples.front() {
            if now.duration_since(t).as_millis() > 320 {
                self.recent_samples.pop_front();
            } else {
                break;
            }
        }

        self.shake_triggered = false;
        self.shake_intensity = 0.0;

        if self.recent_samples.len() >= 3 {
            let oldest = self.recent_samples.front().unwrap().0;
            let time_span_sec = now.duration_since(oldest).as_secs_f32();

            if time_span_sec >= 0.04 {
                let mut reversals_x = 0;
                let mut reversals_y = 0;
                let mut current_dir_x = 0i32;
                let mut accum_dist_x = 0.0f32;
                let mut current_dir_y = 0i32;
                let mut accum_dist_y = 0.0f32;
                let mut total_traveled_x = 0.0f32;
                let mut total_traveled_y = 0.0f32;
                let mut max_speed = 0.0f32;
                let mut peak_vx = 0.0f32;
                let mut peak_vy = 0.0f32;

                for i in 1..self.recent_samples.len() {
                    let dt = self.recent_samples[i]
                        .0
                        .duration_since(self.recent_samples[i - 1].0)
                        .as_secs_f32();
                    if dt > 0.0005 {
                        let dx = self.recent_samples[i].1 - self.recent_samples[i - 1].1;
                        let dy = self.recent_samples[i].2 - self.recent_samples[i - 1].2;
                        let vx = dx / dt;
                        let vy = dy / dt;
                        let speed = (vx * vx + vy * vy).sqrt();
                        if speed > max_speed {
                            max_speed = speed;
                            peak_vx = vx;
                            peak_vy = vy;
                        }
                        total_traveled_x += dx.abs();
                        total_traveled_y += dy.abs();

                        // Direction tracking in X: resilient to deceleration frames
                        if dx.abs() > 1.0 {
                            let dir = if dx > 0.0 { 1 } else { -1 };
                            if current_dir_x == 0 {
                                current_dir_x = dir;
                                accum_dist_x = dx.abs();
                            } else if current_dir_x == dir {
                                accum_dist_x += dx.abs();
                            } else {
                                if accum_dist_x >= 4.0 {
                                    reversals_x += 1;
                                }
                                current_dir_x = dir;
                                accum_dist_x = dx.abs();
                            }
                        }

                        // Direction tracking in Y: resilient to deceleration frames
                        if dy.abs() > 1.0 {
                            let dir = if dy > 0.0 { 1 } else { -1 };
                            if current_dir_y == 0 {
                                current_dir_y = dir;
                                accum_dist_y = dy.abs();
                            } else if current_dir_y == dir {
                                accum_dist_y += dy.abs();
                            } else {
                                if accum_dist_y >= 4.0 {
                                    reversals_y += 1;
                                }
                                current_dir_y = dir;
                                accum_dist_y = dy.abs();
                            }
                        }
                    }
                }

                // Shake detection threshold: responsive, intuitive, and natural
                let sens = sensitivity.clamp(0.2, 3.0);
                let speed_threshold = (220.0 / sens).clamp(110.0, 380.0);
                let min_travel = 16.0 / sens;
                let cooldown_ok = now.duration_since(self.last_shake_time).as_millis() > 120;

                let has_reversal = reversals_x >= 1 || reversals_y >= 1;
                let total_move = total_traveled_x + total_traveled_y;

                if (has_reversal || max_speed > speed_threshold * 1.25)
                    && max_speed > speed_threshold
                    && total_move > min_travel
                    && cooldown_ok
                {
                    self.shake_triggered = true;
                    self.last_shake_time = now;
                    let intensity = ((max_speed - speed_threshold) / 240.0).clamp(0.7, 1.4);
                    self.shake_intensity = intensity;

                    // Physical fling velocity based on peak shake velocity
                    self.shake_velocity_x = (peak_vx * 0.40).clamp(-450.0, 450.0);
                    self.shake_velocity_y = (peak_vy * 0.40).clamp(-300.0, 80.0) - 50.0;
                }
            }
        }
    }
}

pub struct WindowTracker {
    pub tracked_windows: HashMap<isize, WindowRecord>,
    pub all_visible_surfaces: Vec<SurfaceSegment>,
    our_hwnd: HWND,
}

impl WindowTracker {
    pub fn new(our_hwnd: HWND) -> Self {
        Self {
            tracked_windows: HashMap::new(),
            all_visible_surfaces: Vec::new(),
            our_hwnd,
        }
    }

    pub fn update(&mut self, shake_sensitivity: f32) {
        for record in self.tracked_windows.values_mut() {
            record.updated_this_tick = false;
            record.visible_segments.clear();
        }

        let mut enumerated: Vec<(HWND, RECT)> = Vec::with_capacity(32);

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let list = &mut *(lparam.0 as *mut Vec<(HWND, RECT)>);

            if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
                return TRUE;
            }

            // Exclude cloaked windows (hidden virtual desktop / UWP background)
            let mut cloaked = 0u32;
            let hr = DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                &mut cloaked as *mut _ as *mut _,
                std::mem::size_of::<u32>() as u32,
            );
            if hr.is_ok() && cloaked != 0 {
                return TRUE;
            }

            // Extended styles check: exclude tool windows, transparent windows, non-activatable overlays
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            if (ex_style & (WS_EX_TOOLWINDOW.0 as i32 | WS_EX_TRANSPARENT.0 as i32)) != 0 {
                return TRUE;
            }

            let mut rect = RECT::default();
            let hr_bounds = DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect as *mut _ as *mut _,
                std::mem::size_of::<RECT>() as u32,
            );

            if hr_bounds.is_err() {
                let _ = GetWindowRect(hwnd, &mut rect);
            }

            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;

            // Application windows where snow accumulates must be large enough:
            // height >= 90 rejects taskbars (40-55px) and thin toolbars completely
            if width >= 180 && height >= 90 && rect.bottom > 0 && rect.right > 0 {
                let class_name_buf = &mut [0u16; 64];
                let len = GetClassNameW(hwnd, class_name_buf);
                let class_name = String::from_utf16_lossy(&class_name_buf[..len as usize]);
                let class_lower = class_name.to_lowercase();

                // Explicitly filter out Windows Shell, Taskbars, Start Menu, System trays, overlays
                if class_lower.contains("shell_traywnd")
                    || class_lower.contains("shell_secondarytraywnd")
                    || class_lower.contains("progman")
                    || class_lower.contains("workerw")
                    || class_lower.contains("corewindow")
                    || class_lower.contains("xamlexplorer")
                    || class_lower.contains("overflow")
                    || class_lower.contains("snowfall")
                    || class_lower.contains("taskview")
                    || class_lower.contains("cortana")
                    || class_lower.contains("searchhost")
                {
                    return TRUE;
                }

                list.push((hwnd, rect));
            }

            TRUE
        }

        unsafe {
            let ptr = &mut enumerated as *mut Vec<(HWND, RECT)> as isize;
            let _ = EnumWindows(Some(enum_proc), LPARAM(ptr));
        }

        self.all_visible_surfaces.clear();
        let mut occluders: Vec<RECT> = Vec::with_capacity(enumerated.len());

        for (hwnd, rect) in enumerated {
            if hwnd == self.our_hwnd {
                continue;
            }

            let hwnd_key = hwnd.0 as isize;

            let record = self
                .tracked_windows
                .entry(hwnd_key)
                .or_insert_with(|| WindowRecord::new(hwnd_key, rect));
            record.update_position(rect, shake_sensitivity);
            record.updated_this_tick = true;

            let edge_y = rect.top as f32 + 1.5;
            let mut segments = vec![(rect.left as f32, rect.right as f32)];

            for occ in &occluders {
                if edge_y >= occ.top as f32 && edge_y <= occ.bottom as f32 {
                    let occ_x1 = occ.left as f32;
                    let occ_x2 = occ.right as f32;

                    let mut next_segs = Vec::new();
                    for (s_x1, s_x2) in segments {
                        if occ_x1 <= s_x1 && occ_x2 >= s_x2 {
                            continue;
                        } else if occ_x1 > s_x1 && occ_x2 < s_x2 {
                            next_segs.push((s_x1, occ_x1));
                            next_segs.push((occ_x2, s_x2));
                        } else if occ_x1 <= s_x1 && occ_x2 > s_x1 {
                            next_segs.push((occ_x2, s_x2));
                        } else if occ_x1 < s_x2 && occ_x2 >= s_x2 {
                            next_segs.push((s_x1, occ_x1));
                        } else {
                            next_segs.push((s_x1, s_x2));
                        }
                    }
                    segments = next_segs;
                }
            }

            // In Windows 11, non-maximized windows have rounded top corners (radius ~10px).
            // Insetting outer edges prevents snow from hanging in thin air over curved corners!
            let is_maximized = unsafe { IsZoomed(hwnd).as_bool() };
            let corner_inset = if is_maximized { 0.0f32 } else { 10.0f32 };

            for (mut x1, mut x2) in segments {
                if (x1 - rect.left as f32).abs() < 3.0 {
                    x1 += corner_inset;
                }
                if (x2 - rect.right as f32).abs() < 3.0 {
                    x2 -= corner_inset;
                }
                if x2 - x1 >= 24.0 {
                    let seg = SurfaceSegment {
                        hwnd: hwnd_key,
                        x1,
                        x2,
                        y: edge_y,
                    };
                    record.visible_segments.push(seg);
                    self.all_visible_surfaces.push(seg);
                }
            }

            occluders.push(rect);
        }

        self.tracked_windows.retain(|_, record| record.updated_this_tick);
    }
}
