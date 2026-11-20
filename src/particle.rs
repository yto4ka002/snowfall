use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlakeType {
    SmallSoft,     // Distant gentle floating speck/disc
    MediumCrystal, // Classic crisp 6-point snowflake
    LargeFluff,    // Close fluffy complex cluster flake
    Glitter,       // Twinkling crystal
}

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub base_fall_speed: f32,
    pub size: f32,
    pub opacity: f32,
    pub base_opacity: f32,
    pub rotation: f32,
    pub rotation_speed: f32,
    pub wobble_phase: f32,
    pub wobble_speed: f32,
    pub wobble_amplitude: f32,
    pub depth: f32,
    pub flake_type: FlakeType,
    pub active: bool,
    pub lifetime: f32,
}

impl Particle {
    pub fn new_inactive() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            base_fall_speed: 60.0,
            size: 3.0,
            opacity: 0.8,
            base_opacity: 0.8,
            rotation: 0.0,
            rotation_speed: 0.0,
            wobble_phase: 0.0,
            wobble_speed: 1.5,
            wobble_amplitude: 15.0,
            depth: 1.0,
            flake_type: FlakeType::SmallSoft,
            active: false,
            lifetime: 0.0,
        }
    }

    pub fn respawn_at_top(&mut self, screen_w: f32, spawn_y_min: f32, spawn_y_max: f32) {
        let mut rng = rand::thread_rng();

        let type_choice = rng.gen_range(0..100);
        let (flake_type, depth_range, size_range, base_speed_range) = if type_choice < 45 {
            (FlakeType::SmallSoft, 0.35..0.65, 2.0..4.0, 35.0..65.0)
        } else if type_choice < 80 {
            (FlakeType::MediumCrystal, 0.7..0.95, 4.5..7.5, 55.0..90.0)
        } else if type_choice < 92 {
            (FlakeType::LargeFluff, 1.0..1.25, 8.5..13.5, 45.0..80.0)
        } else {
            (FlakeType::Glitter, 0.6..1.0, 3.5..6.0, 50.0..85.0)
        };

        self.depth = rng.gen_range(depth_range);
        self.size = rng.gen_range(size_range) * (0.8 + self.depth * 0.25);
        self.base_fall_speed = rng.gen_range(base_speed_range) * self.depth;
        self.flake_type = flake_type;

        self.x = rng.gen_range(-200.0..(screen_w + 200.0));
        self.y = rng.gen_range(spawn_y_min..spawn_y_max);

        self.vx = 0.0;
        self.vy = self.base_fall_speed;

        self.base_opacity = match self.flake_type {
            FlakeType::SmallSoft => rng.gen_range(0.35..0.65),
            FlakeType::MediumCrystal => rng.gen_range(0.65..0.9),
            FlakeType::LargeFluff => rng.gen_range(0.75..0.95),
            FlakeType::Glitter => rng.gen_range(0.5..0.85),
        };
        self.opacity = self.base_opacity;

        self.rotation = rng.gen_range(0.0..std::f32::consts::TAU);
        self.rotation_speed = rng.gen_range(-1.8..1.8) * (1.3 - self.depth * 0.5);

        self.wobble_phase = rng.gen_range(0.0..std::f32::consts::TAU);
        self.wobble_speed = rng.gen_range(1.2..3.2);
        self.wobble_amplitude = rng.gen_range(12.0..28.0) * self.depth;

        self.active = true;
        self.lifetime = 0.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DebrisKind {
    Chunk,        // Large heavy clump tumbling with fast rotation
    MediumPellet, // Medium snowflake cluster
    PowderDust,   // Billowing fine snow spray / dust
}

#[derive(Clone, Copy, Debug)]
pub struct DebrisParticle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub size: f32,
    pub opacity: f32,
    pub base_opacity: f32,
    pub rotation: f32,
    pub rotation_speed: f32,
    pub kind: DebrisKind,
    pub life: f32,
    pub max_life: f32,
    pub immune_window_id: isize,
    pub active: bool,
}

impl DebrisParticle {
    pub fn new_inactive() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            size: 5.0,
            opacity: 0.9,
            base_opacity: 0.9,
            rotation: 0.0,
            rotation_speed: 0.0,
            kind: DebrisKind::Chunk,
            life: 0.0,
            max_life: 3.0,
            immune_window_id: 0,
            active: false,
        }
    }

    pub fn spawn(
        &mut self,
        x: f32,
        y: f32,
        init_vx: f32,
        init_vy: f32,
        kind: DebrisKind,
        size_override: Option<f32>,
        source_window_id: isize,
    ) {
        let mut rng = rand::thread_rng();
        self.x = x;
        self.y = y;
        self.kind = kind;
        self.immune_window_id = source_window_id;
        self.life = 0.0;
        self.active = true;

        match kind {
            DebrisKind::Chunk => {
                let base = size_override.unwrap_or(12.0);
                self.size = (base + rng.gen_range(-1.5..1.5)).clamp(4.0, 26.0);
                self.vx = init_vx + rng.gen_range(-30.0..30.0);
                self.vy = init_vy + rng.gen_range(-20.0..20.0);
                self.base_opacity = rng.gen_range(0.88..1.0);
                self.opacity = self.base_opacity;
                self.rotation = rng.gen_range(0.0..std::f32::consts::TAU);
                self.rotation_speed = rng.gen_range(-9.0..9.0);
                self.max_life = rng.gen_range(2.5..4.5);
            }
            DebrisKind::MediumPellet => {
                let base = size_override.unwrap_or(6.0);
                self.size = (base + rng.gen_range(-1.0..1.0)).clamp(3.0, 11.0);
                self.vx = init_vx + rng.gen_range(-25.0..25.0);
                self.vy = init_vy + rng.gen_range(-20.0..20.0);
                self.base_opacity = rng.gen_range(0.80..0.95);
                self.opacity = self.base_opacity;
                self.rotation = rng.gen_range(0.0..std::f32::consts::TAU);
                self.rotation_speed = rng.gen_range(-7.0..7.0);
                self.max_life = rng.gen_range(2.0..3.6);
            }
            DebrisKind::PowderDust => {
                let base = size_override.unwrap_or(3.5);
                self.size = (base + rng.gen_range(-0.8..0.8)).clamp(2.0, 6.5);
                self.vx = init_vx + rng.gen_range(-20.0..20.0);
                self.vy = init_vy + rng.gen_range(-20.0..10.0);
                self.base_opacity = rng.gen_range(0.40..0.70);
                self.opacity = self.base_opacity;
                self.rotation = rng.gen_range(0.0..std::f32::consts::TAU);
                self.rotation_speed = rng.gen_range(-4.0..4.0);
                self.max_life = rng.gen_range(1.2..2.2);
            }
        }
    }
}

pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub max_particles: usize,
    pub debris: Vec<DebrisParticle>,
}

const DEBRIS_POOL_SIZE: usize = 1600;

impl ParticleSystem {
    pub fn new(capacity: usize) -> Self {
        let mut particles = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            particles.push(Particle::new_inactive());
        }

        let mut debris = Vec::with_capacity(DEBRIS_POOL_SIZE);
        for _ in 0..DEBRIS_POOL_SIZE {
            debris.push(DebrisParticle::new_inactive());
        }

        Self {
            particles,
            max_particles: capacity,
            debris,
        }
    }

    pub fn resize(&mut self, target_count: usize, screen_w: f32, screen_h: f32) {
        let count = target_count.clamp(30, 2000);
        self.max_particles = count;
        if self.particles.len() < count {
            while self.particles.len() < count {
                let mut p = Particle::new_inactive();
                p.respawn_at_top(screen_w, -20.0, screen_h + 10.0);
                self.particles.push(p);
            }
        } else if self.particles.len() > count {
            self.particles.truncate(count);
        }
    }

    pub fn init_particles(&mut self, screen_w: f32, screen_h: f32, count: usize) {
        let count = count.clamp(30, 2000);
        self.max_particles = count;
        self.particles.clear();
        for _ in 0..count {
            let mut p = Particle::new_inactive();
            p.respawn_at_top(screen_w, -20.0, screen_h + 10.0);
            self.particles.push(p);
        }
    }

    pub fn spawn_debris_particle(
        &mut self,
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
        kind: DebrisKind,
        size_override: Option<f32>,
        source_window_id: isize,
    ) {
        for d in self.debris.iter_mut() {
            if !d.active {
                d.spawn(x, y, vx, vy, kind, size_override, source_window_id);
                return;
            }
        }
        // If all are active, recycle the oldest one safely
        if let Some(oldest) = self.debris.iter_mut().max_by(|a, b| a.life.partial_cmp(&b.life).unwrap_or(std::cmp::Ordering::Equal)) {
            oldest.spawn(x, y, vx, vy, kind, size_override, source_window_id);
        }
    }
}
