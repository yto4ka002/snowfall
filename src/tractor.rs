use crate::accumulation::{AccumulationSystem, COLUMN_WIDTH};
use crate::particle::DebrisKind;
use rand::Rng;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug)]
pub struct ExhaustPuff {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub size: f32,
    pub life: f32,
    pub max_life: f32,
    pub active: bool,
}

impl ExhaustPuff {
    fn inactive() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            size: 3.0,
            life: 0.0,
            max_life: 0.7,
            active: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TractorJob {
    window_id: isize,
    direction: f32,
}

pub struct TractorSystem {
    pub active: bool,
    pub current_window_id: isize,
    pub x: f32,
    pub y: f32,
    pub direction: f32,
    pub wheel_phase: f32,
    pub bounce_phase: f32,
    pub beacon_phase: f32,
    pub alpha: f32,
    pub snow_wave: f32,
    pub exhaust: [ExhaustPuff; 16],

    start_x: f32,
    end_x: f32,
    last_shelf_x1: f32,
    prev_blade_x: f32,
    exhaust_timer: f32,
    speed: f32,
    queue: VecDeque<TractorJob>,

    auto_timer: f32,
    next_auto_interval: f32,
}

impl TractorSystem {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            active: false,
            current_window_id: 0,
            x: 0.0,
            y: 0.0,
            direction: 1.0,
            wheel_phase: 0.0,
            bounce_phase: 0.0,
            beacon_phase: 0.0,
            alpha: 0.0,
            snow_wave: 0.0,
            exhaust: [ExhaustPuff::inactive(); 16],
            start_x: 0.0,
            end_x: 0.0,
            last_shelf_x1: 0.0,
            prev_blade_x: 0.0,
            exhaust_timer: 0.0,
            speed: 195.0,
            queue: VecDeque::with_capacity(16),
            auto_timer: 0.0,
            next_auto_interval: rng.gen_range(120.0..=180.0),
        }
    }

    pub fn trigger_manual(&mut self, accumulation: &AccumulationSystem) {
        let mut rng = rand::thread_rng();
        self.auto_timer = 0.0;
        self.next_auto_interval = rng.gen_range(120.0..=180.0);
        self.enqueue_shelves(accumulation, true);
    }

    pub fn trigger_auto(&mut self, accumulation: &AccumulationSystem) {
        self.enqueue_shelves(accumulation, false);
    }

    fn enqueue_shelves(&mut self, accumulation: &AccumulationSystem, include_clean_fallback: bool) {
        let mut candidates: Vec<(isize, f32, f32, bool)> = accumulation
            .shelves
            .values()
            .filter(|s| s.x2 - s.x1 >= 30.0)
            .map(|s| {
                let has_snow = s.heights.iter().any(|&h| h >= 0.45);
                (s.id, s.base_y, s.x1, has_snow)
            })
            .collect();

        if candidates.is_empty() {
            return;
        }

        // Sort top-to-bottom on screen, then left-to-right
        candidates.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        });

        let snowy: Vec<_> = candidates.iter().filter(|c| c.3).copied().collect();
        let chosen = if !snowy.is_empty() {
            snowy
        } else if include_clean_fallback && !self.active {
            vec![candidates[0]]
        } else {
            return;
        };

        let mut dir = 1.0f32;
        for &(win_id, _, _, _) in &chosen {
            if self.active && (win_id == self.current_window_id || self.queue.iter().any(|j| j.window_id == win_id)) {
                continue;
            }
            self.queue.push_back(TractorJob {
                window_id: win_id,
                direction: dir,
            });
            dir = -dir;
        }

        if !self.active {
            self.pop_next_job(accumulation);
        }
    }

    fn pop_next_job(&mut self, accumulation: &AccumulationSystem) -> bool {
        while let Some(job) = self.queue.pop_front() {
            if let Some(shelf) = accumulation.shelves.get(&job.window_id) {
                self.active = true;
                self.current_window_id = job.window_id;
                self.direction = job.direction;
                self.y = shelf.base_y;
                self.last_shelf_x1 = shelf.x1;
                self.snow_wave = 0.0;
                self.alpha = 0.0;
                self.exhaust_timer = 0.0;

                if job.direction > 0.0 {
                    self.start_x = shelf.x1 - 24.0;
                    self.end_x = shelf.x2 + 26.0;
                } else {
                    self.start_x = shelf.x2 + 24.0;
                    self.end_x = shelf.x1 - 26.0;
                }
                self.x = self.start_x;
                self.prev_blade_x = self.x + self.direction * 22.0;

                // Speed scales slightly for ultra-wide windows so passes feel crisp (4–8s)
                let width = (shelf.x2 - shelf.x1).max(100.0);
                self.speed = (width * 0.24).clamp(175.0, 340.0);
                return true;
            }
        }
        self.active = false;
        self.current_window_id = 0;
        false
    }

    pub fn update(
        &mut self,
        dt: f32,
        auto_enabled: bool,
        accumulation: &mut AccumulationSystem,
        debris_buf: &mut Vec<(f32, f32, f32, f32, DebrisKind, Option<f32>, isize)>,
    ) {
        let mut rng = rand::thread_rng();

        // Update existing exhaust puffs regardless of tractor active state
        for p in self.exhaust.iter_mut() {
            if !p.active {
                continue;
            }
            p.life += dt;
            if p.life >= p.max_life {
                p.active = false;
                continue;
            }
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vx *= (1.0 - 1.5 * dt).max(0.2);
            p.size += 5.5 * dt;
        }

        // Check periodic 2-3 minute auto-cleaning timer
        if auto_enabled {
            self.auto_timer += dt;
            if self.auto_timer >= self.next_auto_interval {
                self.auto_timer = 0.0;
                self.next_auto_interval = rng.gen_range(120.0..=180.0);
                self.trigger_auto(accumulation);
            }
        }

        if !self.active {
            return;
        }

        self.wheel_phase += dt * self.speed * 0.14;
        self.bounce_phase += dt * 28.0;
        self.beacon_phase += dt * 8.0;

        // Track live window position; advance to next job if window disappeared
        let Some(shelf) = accumulation.shelves.get_mut(&self.current_window_id) else {
            self.pop_next_job(accumulation);
            return;
        };

        // Follow window if dragged horizontally or vertically
        let shift_x = shelf.x1 - self.last_shelf_x1;
        if shift_x.abs() > 0.01 {
            self.x += shift_x;
            self.start_x += shift_x;
            self.prev_blade_x += shift_x;
            self.last_shelf_x1 = shelf.x1;
        }
        self.y = shelf.base_y;
        self.end_x = if self.direction > 0.0 {
            shelf.x2 + 26.0
        } else {
            shelf.x1 - 26.0
        };

        // Advance tractor along the window ledge
        self.x += self.direction * self.speed * dt;

        // Smooth fade-in and fade-out at window edges
        let dist_start = (self.x - self.start_x).abs();
        let dist_end = (self.end_x - self.x).abs();
        self.alpha = (dist_start / 18.0).min(dist_end / 18.0).clamp(0.0, 1.0);

        // Emit exhaust smoke puff from chimney stack
        self.exhaust_timer += dt;
        if self.exhaust_timer >= 0.14 && self.alpha > 0.35 {
            self.exhaust_timer = 0.0;
            for p in self.exhaust.iter_mut() {
                if !p.active {
                    p.active = true;
                    p.x = self.x + self.direction * 7.5;
                    p.y = self.y - 25.0;
                    p.vx = -self.direction * rng.gen_range(18.0..38.0);
                    p.vy = -rng.gen_range(24.0..42.0);
                    p.size = rng.gen_range(2.2..3.6);
                    p.life = 0.0;
                    p.max_life = rng.gen_range(0.55..0.85);
                    break;
                }
            }
        }

        // Sweep snow under plow blade
        let blade_x = self.x + self.direction * 22.0;
        let min_sweep = if self.direction > 0.0 {
            self.prev_blade_x - 14.0
        } else {
            blade_x
        };
        let max_sweep = if self.direction > 0.0 {
            blade_x
        } else {
            self.prev_blade_x + 14.0
        };

        let num_cols = shelf.heights.len();
        let mut cleared_sum = 0.0f32;
        let mut max_cleared_h = 0.0f32;

        if num_cols > 0 {
            let col_start = (((min_sweep - shelf.x1) / COLUMN_WIDTH).floor() as isize).max(0) as usize;
            let col_end = (((max_sweep - shelf.x1) / COLUMN_WIDTH).ceil() as isize)
                .min(num_cols as isize - 1)
                .max(0) as usize;

            if col_start <= col_end {
                for i in col_start..=col_end {
                    let h = shelf.heights[i];
                    if h > 0.05 {
                        cleared_sum += h;
                        if h > max_cleared_h {
                            max_cleared_h = h;
                        }
                        shelf.heights[i] = 0.0;
                    }
                }
            }
        }

        if cleared_sum > 0.35 {
            self.snow_wave = (self.snow_wave + cleared_sum * 0.38).min(12.0);

            // Spray snow particles off the plow blade
            let spray_count = ((cleared_sum * 0.25).ceil() as usize).clamp(1, 5);
            for k in 0..spray_count {
                let kind = if max_cleared_h >= 8.0 && k == 0 && rng.gen_bool(0.35) {
                    DebrisKind::Chunk
                } else if k % 2 == 0 {
                    DebrisKind::MediumPellet
                } else {
                    DebrisKind::PowderDust
                };
                let px = blade_x + self.direction * rng.gen_range(0.0..6.0);
                let py = self.y - rng.gen_range(1.0..max_cleared_h.max(3.0));
                let pvx = self.direction * rng.gen_range(55.0..155.0);
                let pvy = -rng.gen_range(35.0..105.0);
                let sz = rng.gen_range(2.8..6.2);
                debris_buf.push((px, py, pvx, pvy, kind, Some(sz), self.current_window_id));
            }
        }

        self.snow_wave *= (1.0 - 2.0 * dt).max(0.0);
        self.prev_blade_x = blade_x;

        // Finish current window pass
        let finished = if self.direction > 0.0 {
            self.x >= self.end_x
        } else {
            self.x <= self.end_x
        };

        if finished {
            // Spill accumulated snow wave off the edge of the window
            if self.snow_wave > 0.8 {
                let spill_count = ((self.snow_wave * 0.8).ceil() as usize).clamp(2, 8);
                let edge_x = if self.direction > 0.0 {
                    shelf.x2
                } else {
                    shelf.x1
                };
                for k in 0..spill_count {
                    let kind = if k % 3 == 0 {
                        DebrisKind::Chunk
                    } else {
                        DebrisKind::MediumPellet
                    };
                    debris_buf.push((
                        edge_x,
                        self.y - rng.gen_range(1.0..self.snow_wave.max(2.0)),
                        self.direction * rng.gen_range(80.0..180.0),
                        -rng.gen_range(20.0..65.0),
                        kind,
                        Some(rng.gen_range(4.0..8.5)),
                        self.current_window_id,
                    ));
                }
            }

            // Guarantee the entire shelf is spotless when the tractor completes its pass
            for h in shelf.heights.iter_mut() {
                *h = 0.0;
            }

            self.pop_next_job(accumulation);
        }
    }
}
