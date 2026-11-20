use rand::Rng;
use std::f32::consts::PI;

pub struct WeatherSystem {
    time: f32,
    base_wind_strength: f32,
    base_angle: f32,
    target_angle: f32,
    angle_transition_speed: f32,

    // Gust dynamics
    gust_active: bool,
    gust_timer: f32,
    gust_interval: f32,
    gust_progress: f32,
    gust_duration: f32,
    gust_force: f32,
    gust_dir: f32,

    pub current_wind_x: f32,
    pub current_wind_y: f32,
    pub is_strong_gust: bool,
}

impl WeatherSystem {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            time: 0.0,
            base_wind_strength: 35.0,
            base_angle: 0.1,
            target_angle: 0.2,
            angle_transition_speed: 0.05,

            gust_active: false,
            gust_timer: 0.0,
            gust_interval: rng.gen_range(10.0..22.0),
            gust_progress: 0.0,
            gust_duration: 3.5,
            gust_force: 0.0,
            gust_dir: 1.0,

            current_wind_x: 0.0,
            current_wind_y: 0.0,
            is_strong_gust: false,
        }
    }

    pub fn update(&mut self, dt: f32, intensity_setting: f32) {
        self.time += dt;
        let mut rng = rand::thread_rng();

        // Smoothly drift base wind direction
        if (self.base_angle - self.target_angle).abs() < 0.05 {
            // Pick new target angle gently (-0.6 to +0.6 radians)
            self.target_angle = rng.gen_range(-0.6..0.6);
            self.angle_transition_speed = rng.gen_range(0.04..0.12);
        }

        self.base_angle += (self.target_angle - self.base_angle) * self.angle_transition_speed * dt;

        // Subtle harmonic variation in base wind speed
        let harmonic = (self.time * 0.4).sin() * 0.5 + (self.time * 0.17).cos() * 0.3;
        let base_speed = (self.base_wind_strength * (1.0 + harmonic * 0.4)).max(5.0);

        // Gust handling
        self.gust_timer += dt;
        let mut gust_wind = 0.0;
        self.is_strong_gust = false;

        if !self.gust_active {
            if self.gust_timer >= self.gust_interval {
                self.gust_active = true;
                self.gust_progress = 0.0;
                self.gust_duration = rng.gen_range(2.5..4.5);
                self.gust_force = rng.gen_range(60.0..120.0);
                // Gust usually aligns with current general wind direction or slightly amplifies it
                self.gust_dir = if self.base_angle >= 0.0 { 1.0 } else { -1.0 };
                if rng.gen_bool(0.15) {
                    self.gust_dir = -self.gust_dir; // Occasional cross-current
                }
            }
        } else {
            self.gust_progress += dt / self.gust_duration;
            if self.gust_progress >= 1.0 {
                self.gust_active = false;
                self.gust_timer = 0.0;
                self.gust_interval = rng.gen_range(12.0..25.0);
            } else {
                // Bell curve envelope: sin(pi * progress)^2
                let envelope = (self.gust_progress * PI).sin().powi(2);
                gust_wind = self.gust_force * self.gust_dir * envelope;
                if envelope > 0.65 && self.gust_force > 80.0 {
                    self.is_strong_gust = true;
                }
            }
        }

        // Calculate combined wind components
        let total_horizontal = (self.base_angle.sin() * base_speed + gust_wind) * intensity_setting;
        let total_vertical = (self.base_angle.cos() * 8.0).abs() * intensity_setting;

        self.current_wind_x = total_horizontal;
        self.current_wind_y = total_vertical;
    }
}
