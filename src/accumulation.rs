use crate::particle::DebrisKind;
use rand::Rng;
use std::collections::HashMap;

pub const COLUMN_WIDTH: f32 = 4.0;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct SnowShelf {
    pub id: isize,
    pub x1: f32,
    pub x2: f32,
    pub base_y: f32,
    pub heights: Vec<f32>,
    pub max_height: f32,
}

impl SnowShelf {
    pub fn new(id: isize, x1: f32, x2: f32, base_y: f32, max_height: f32) -> Self {
        let width = (x2 - x1).max(COLUMN_WIDTH);
        let num_cols = (width / COLUMN_WIDTH).ceil() as usize + 1;
        Self {
            id,
            x1,
            x2,
            base_y,
            heights: vec![0.0; num_cols],
            max_height,
        }
    }

    pub fn resize_and_shift(&mut self, new_x1: f32, new_x2: f32, new_base_y: f32) {
        let old_x1 = self.x1;
        self.x1 = new_x1;
        self.x2 = new_x2;
        self.base_y = new_base_y;

        let width = (new_x2 - new_x1).max(COLUMN_WIDTH);
        let new_num_cols = (width / COLUMN_WIDTH).ceil() as usize + 1;

        if new_num_cols != self.heights.len() {
            let mut new_heights = vec![0.0; new_num_cols];
            let shift_cols = ((new_x1 - old_x1) / COLUMN_WIDTH).round() as isize;

            for (old_idx, &h) in self.heights.iter().enumerate() {
                let target_idx = old_idx as isize - shift_cols;
                if target_idx >= 0 && (target_idx as usize) < new_num_cols {
                    new_heights[target_idx as usize] = h;
                }
            }
            self.heights = new_heights;
        }
    }

    pub fn try_accumulate(&mut self, px: f32, py: f32, fall_delta_y: f32, amount: f32) -> bool {
        if px < self.x1 || px > self.x2 {
            return false;
        }

        let col_idx = ((px - self.x1) / COLUMN_WIDTH) as usize;
        if col_idx >= self.heights.len() {
            return false;
        }

        let surface_y = self.base_y - self.heights[col_idx];
        let catch_depth = fall_delta_y.max(5.0).min(14.0);
        // Generous catch zone: catches flakes hitting or passing through the surface
        if py >= surface_y - 3.0 && py <= surface_y + catch_depth {
            let added = amount.min(self.max_height - self.heights[col_idx]);
            if added > 0.0 {
                self.heights[col_idx] = (self.heights[col_idx] + added * 0.50).min(self.max_height);

                if col_idx > 0 {
                    self.heights[col_idx - 1] =
                        (self.heights[col_idx - 1] + added * 0.25).min(self.max_height);
                }
                if col_idx + 1 < self.heights.len() {
                    self.heights[col_idx + 1] =
                        (self.heights[col_idx + 1] + added * 0.25).min(self.max_height);
                }
            }
            return true;
        }

        false
    }

    pub fn diffuse_and_settle(&mut self) {
        if self.heights.len() < 3 {
            return;
        }

        // Natural angle of repose (~30-35 deg): allows rich fluffy snow mounds to form
        let max_slope = 2.4;
        let len = self.heights.len();

        for i in 1..len - 1 {
            let diff_left = self.heights[i] - self.heights[i - 1];
            if diff_left > max_slope {
                let transfer = (diff_left - max_slope) * 0.15;
                self.heights[i] -= transfer;
                self.heights[i - 1] += transfer;
            }

            let diff_right = self.heights[i] - self.heights[i + 1];
            if diff_right > max_slope {
                let transfer = (diff_right - max_slope) * 0.15;
                self.heights[i] -= transfer;
                self.heights[i + 1] += transfer;
            }
        }

        let overhang_limit = 5.0;
        if self.heights[0] > overhang_limit {
            self.heights[0] -= 0.10;
        }
        if let Some(last) = self.heights.last_mut() {
            if *last > overhang_limit {
                *last -= 0.10;
            }
        }

        // Smoothly taper down snow at the outer edges so it conforms to the window corner
        let n = self.heights.len();
        if n >= 4 {
            if self.heights[0] > 1.2 { self.heights[0] = 1.2; }
            if self.heights[1] > 3.5 { self.heights[1] = 3.5; }
            if self.heights[n - 1] > 1.2 { self.heights[n - 1] = 1.2; }
            if self.heights[n - 2] > 3.5 { self.heights[n - 2] = 3.5; }
        }
    }

    pub fn shed_snow(
        &mut self,
        intensity: f32,
        fling_vx: f32,
        fling_vy: f32,
        debris_out: &mut Vec<(f32, f32, f32, f32, DebrisKind, Option<f32>, isize)>,
    ) {
        let mut rng = rand::thread_rng();
        let num_cols = self.heights.len();
        if num_cols < 2 {
            return;
        }

        // Process snow along the window in physical clusters of 2..4 columns (8-16 px wide)
        let mut col = 0;
        while col < num_cols {
            let chunk_cols = rng.gen_range(2..=4).min(num_cols - col);
            let mut total_h = 0.0f32;
            let mut max_h = 0.0f32;
            for i in col..(col + chunk_cols) {
                let h = self.heights[i];
                total_h += h;
                if h > max_h {
                    max_h = h;
                }
            }
            let avg_h = total_h / chunk_cols as f32;

            // Only shed if there is visible snow on this section (>= 0.65 px)
            if max_h >= 0.65 {
                // Determine shed ratio based on shake intensity:
                // Gentle shake: sheds 35-55% of the crest
                // Moderate shake: sheds 60-85%
                // Hard shake: sheds 100%
                let shed_ratio = if intensity >= 1.05 {
                    1.0f32
                } else if intensity >= 0.85 {
                    (0.65 + 0.35 * ((intensity - 0.85) / 0.20)).clamp(0.65, 1.0)
                } else {
                    (0.35 + 0.30 * ((intensity - 0.60) / 0.25).max(0.0)).clamp(0.30, 0.65)
                };

                let shed_h = avg_h * shed_ratio;

                // Update remaining height on the shelf
                for i in col..(col + chunk_cols) {
                    let old_h = self.heights[i];
                    let rem = old_h * (1.0 - shed_ratio);
                    // If remaining is negligible or shake was hard, clear completely
                    self.heights[i] = if rem < 0.85 || shed_ratio >= 0.95 {
                        0.0
                    } else {
                        rem
                    };
                }

                // If the amount shed is very tiny (e.g. shaving < 0.8 px off), only spawn 1 dust speck
                if shed_h < 0.8 {
                    let px = self.x1 + (col as f32 + chunk_cols as f32 * 0.5) * COLUMN_WIDTH;
                    let py = self.base_y - max_h * 0.5;
                    let pvx = fling_vx * 0.2 + rng.gen_range(-30.0..30.0);
                    let pvy = fling_vy * 0.2 - rng.gen_range(15.0..40.0);
                    debris_out.push((px, py, pvx, pvy, DebrisKind::PowderDust, Some(2.5), self.id));
                    col += chunk_cols;
                    continue;
                }

                // PROPORTIONAL PARTICLE GENERATION BASED ON ACTUAL SHED VOLUME:
                // A. Light snow dusting (shed_h: 0.8..2.5 px) -> 0 chunks, 1-2 pellets, 2-3 dust
                // B. Modest snow layer (shed_h: 2.5..6.0 px) -> 1 small chunk, 2-4 pellets, 4-7 dust
                // C. Medium snow cap (shed_h: 6.0..14.0 px) -> 2-4 medium chunks, 4-8 pellets, 8-15 dust
                // D. Heavy deep snow cap (shed_h: 14.0..30.0+ px) -> 3-6 large blocks, 8-18 pellets, 15-30 dust cloud

                let (chunk_count, pellet_count, powder_count, chunk_size_base) = if shed_h < 2.5 {
                    (0, rng.gen_range(1..=2), rng.gen_range(2..=3), 4.0)
                } else if shed_h < 6.0 {
                    (1, rng.gen_range(2..=4), rng.gen_range(4..=7), (shed_h * 1.3).clamp(5.0, 8.0))
                } else if shed_h < 14.0 {
                    let chunks = ((shed_h / 3.8).round() as usize).clamp(2, 4);
                    let pellets = ((shed_h / 1.6).round() as usize).clamp(4, 9);
                    let powder = ((shed_h / 0.9).round() as usize).clamp(8, 16);
                    (chunks, pellets, powder, (shed_h * 1.1).clamp(8.0, 15.0))
                } else {
                    let chunks = ((shed_h / 3.2).round() as usize).clamp(3, 7);
                    let pellets = ((shed_h / 1.3).round() as usize).clamp(8, 20);
                    let powder = ((shed_h / 0.6).round() as usize).clamp(15, 35);
                    (chunks, pellets, powder, (shed_h * 1.0).clamp(14.0, 24.0))
                };

                let col_start_x = self.x1 + col as f32 * COLUMN_WIDTH;
                let chunk_width_px = chunk_cols as f32 * COLUMN_WIDTH;

                // 1. Chunks: tumbling physical clumps whose size matches the snow cap
                for _ in 0..chunk_count {
                    let px = col_start_x + rng.gen_range(0.0..chunk_width_px);
                    let py = self.base_y - rng.gen_range(0.0..max_h);
                    let pvx = fling_vx * rng.gen_range(0.40..0.95) + rng.gen_range(-60.0..60.0);
                    // Upward pop proportional to snow height and shake intensity
                    let pop = rng.gen_range(40.0..130.0) * (shed_h / 10.0).clamp(0.6, 1.4) * intensity.clamp(0.7, 1.3);
                    let pvy = fling_vy * 0.45 - pop;
                    let sz = (chunk_size_base + rng.gen_range(-2.0..2.5)).clamp(4.0, 25.0);
                    debris_out.push((px, py, pvx, pvy, DebrisKind::Chunk, Some(sz), self.id));
                }

                // 2. Pellets: crisp crystal clusters flying outward
                for _ in 0..pellet_count {
                    let px = col_start_x + rng.gen_range(0.0..chunk_width_px);
                    let py = self.base_y - rng.gen_range(0.0..max_h);
                    let pvx = fling_vx * rng.gen_range(0.30..0.85) + rng.gen_range(-80.0..80.0);
                    let pop = rng.gen_range(35.0..110.0) * (shed_h / 10.0).clamp(0.5, 1.3) * intensity.clamp(0.7, 1.3);
                    let pvy = fling_vy * 0.40 - pop;
                    let sz = (shed_h * 0.4 + rng.gen_range(3.0..6.5)).clamp(3.5, 9.5);
                    debris_out.push((px, py, pvx, pvy, DebrisKind::MediumPellet, Some(sz), self.id));
                }

                // 3. Powder Dust: fine misty cloud dispersing in air
                for _ in 0..powder_count {
                    let px = col_start_x + rng.gen_range(-4.0..(chunk_width_px + 4.0));
                    let py = self.base_y - rng.gen_range(0.0..(max_h + 3.0));
                    let pvx = fling_vx * 0.20 + rng.gen_range(-100.0..100.0);
                    let pop = rng.gen_range(20.0..80.0) * intensity.clamp(0.6, 1.2);
                    let pvy = fling_vy * 0.25 - pop;
                    let sz = rng.gen_range(2.0..4.8);
                    debris_out.push((px, py, pvx, pvy, DebrisKind::PowderDust, Some(sz), self.id));
                }
            } else if max_h > 0.0 && intensity >= 0.85 {
                // If there's just a tiny sub-threshold dust layer (< 0.65 px) and shake is active, clear it quietly
                for i in col..(col + chunk_cols) {
                    self.heights[i] = 0.0;
                }
            }

            col += chunk_cols;
        }
    }
}

pub struct AccumulationSystem {
    pub shelves: HashMap<isize, SnowShelf>,
    max_height: f32,
    accumulation_rate: f32,
}

impl AccumulationSystem {
    pub fn new(_screen_w: f32, _screen_h: f32, max_height: f32, accumulation_rate: f32) -> Self {
        Self {
            shelves: HashMap::new(),
            max_height,
            accumulation_rate,
        }
    }

    pub fn set_params(&mut self, max_height: f32, rate: f32) {
        self.max_height = max_height;
        self.accumulation_rate = rate;
        for shelf in self.shelves.values_mut() {
            shelf.max_height = max_height;
        }
    }

    pub fn sync_with_windows(
        &mut self,
        visible_segments: &[(isize, f32, f32, f32)],
    ) {
        let mut active_ids = std::collections::HashSet::new();

        for &(hwnd, x1, x2, y) in visible_segments {
            active_ids.insert(hwnd);
            if let Some(shelf) = self.shelves.get_mut(&hwnd) {
                shelf.resize_and_shift(x1, x2, y);
            } else {
                let new_shelf = SnowShelf::new(hwnd, x1, x2, y, self.max_height);
                self.shelves.insert(hwnd, new_shelf);
            }
        }

        self.shelves.retain(|id, _| active_ids.contains(id));
    }

    #[allow(dead_code)]
    pub fn resize_screen(&mut self, _screen_w: f32, _screen_h: f32) {
        // No-op: we accumulate exclusively on application windows
    }

    pub fn check_collision_and_accumulate(
        &mut self,
        px: f32,
        py: f32,
        fall_delta_y: f32,
        flake_type: crate::particle::FlakeType,
        size: f32,
        enabled: bool,
    ) -> bool {
        if !enabled {
            return false;
        }

        // Distinct, satisfying volume for each flake type
        let base_volume = match flake_type {
            crate::particle::FlakeType::SmallSoft => 0.55,
            crate::particle::FlakeType::MediumCrystal => 1.05,
            crate::particle::FlakeType::LargeFluff => 1.95,
            crate::particle::FlakeType::Glitter => 0.75,
        };
        let amount = base_volume * (size / 5.0).clamp(0.6, 1.4) * self.accumulation_rate * 0.85;

        for shelf in self.shelves.values_mut() {
            if shelf.try_accumulate(px, py, fall_delta_y, amount) {
                return true;
            }
        }

        false
    }

    pub fn update_diffusion(&mut self) {
        for shelf in self.shelves.values_mut() {
            shelf.diffuse_and_settle();
        }
    }

    pub fn clear_all(&mut self) {
        for shelf in self.shelves.values_mut() {
            for h in shelf.heights.iter_mut() {
                *h = 0.0;
            }
        }
    }
}
