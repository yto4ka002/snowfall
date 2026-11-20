use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

pub struct CursorTracker {
    pub pos_x: f32,
    pub pos_y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub speed: f32,
    first_poll: bool,
}

impl CursorTracker {
    pub fn new() -> Self {
        Self {
            pos_x: 0.0,
            pos_y: 0.0,
            vel_x: 0.0,
            vel_y: 0.0,
            speed: 0.0,
            first_poll: true,
        }
    }

    pub fn update(&mut self, dt: f32) {
        let mut pt = POINT::default();
        unsafe {
            if GetCursorPos(&mut pt).is_ok() {
                let cur_x = pt.x as f32;
                let cur_y = pt.y as f32;

                if self.first_poll {
                    self.pos_x = cur_x;
                    self.pos_y = cur_y;
                    self.first_poll = false;
                    return;
                }

                let dx = cur_x - self.pos_x;
                let dy = cur_y - self.pos_y;

                if dt > 0.0001 {
                    let raw_vx = dx / dt;
                    let raw_vy = dy / dt;
                    // Exponential smoothing for cursor velocity
                    let smooth = 0.35;
                    self.vel_x = self.vel_x * (1.0 - smooth) + raw_vx * smooth;
                    self.vel_y = self.vel_y * (1.0 - smooth) + raw_vy * smooth;
                }

                self.pos_x = cur_x;
                self.pos_y = cur_y;
                self.speed = (self.vel_x * self.vel_x + self.vel_y * self.vel_y).sqrt();
            }
        }
    }

    pub fn apply_to_particle(
        &self,
        px: f32,
        py: f32,
        p_vx: &mut f32,
        p_vy: &mut f32,
        dt: f32,
    ) {
        if self.speed < 40.0 {
            return;
        }

        let dx = px - self.pos_x;
        let dy = py - self.pos_y;
        let dist_sq = dx * dx + dy * dy;
        let radius = 110.0;
        let radius_sq = radius * radius;

        if dist_sq < radius_sq && dist_sq > 4.0 {
            let dist = dist_sq.sqrt();
            let norm_x = dx / dist;
            let norm_y = dy / dist;

            // Factor decays smoothly with distance (1 at center, 0 at radius)
            let factor = (1.0 - dist / radius).powi(2);
            let force_magnitude = (self.speed * 0.18).min(220.0) * factor;

            // Radial outward push
            let radial_x = norm_x * force_magnitude;
            let radial_y = norm_y * force_magnitude;

            // Tangential swirl (perpendicular to cursor movement)
            let speed_safe = self.speed.max(1.0);
            let dir_x = self.vel_x / speed_safe;
            let dir_y = self.vel_y / speed_safe;
            let perp_x = -dir_y;
            let perp_y = dir_x;

            let swirl_x = perp_x * force_magnitude * 0.45;
            let swirl_y = perp_y * force_magnitude * 0.45;

            *p_vx += (radial_x + swirl_x) * dt * 4.0;
            *p_vy += (radial_y + swirl_y) * dt * 4.0;
        }
    }
}
