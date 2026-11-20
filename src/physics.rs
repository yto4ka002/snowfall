use crate::accumulation::AccumulationSystem;
use crate::config::Settings;
use crate::cursor::CursorTracker;
use crate::particle::{DebrisKind, FlakeType, ParticleSystem};
use crate::weather::WeatherSystem;
use crate::window_tracker::WindowTracker;

pub struct PhysicsEngine;

impl PhysicsEngine {
    pub fn update(
        particles: &mut ParticleSystem,
        accumulation: &mut AccumulationSystem,
        weather: &WeatherSystem,
        cursor: &CursorTracker,
        window_tracker: &mut WindowTracker,
        settings: &Settings,
        screen_w: f32,
        screen_h: f32,
        dt: f32,
    ) {
        // 1. Process window shake events and generate realistic debris bursts
        let mut shake_debris = Vec::new();
        for record in window_tracker.tracked_windows.values_mut() {
            if record.shake_triggered {
                if let Some(shelf) = accumulation.shelves.get_mut(&record.hwnd) {
                    shelf.shed_snow(
                        record.shake_intensity,
                        record.shake_velocity_x,
                        record.shake_velocity_y,
                        &mut shake_debris,
                    );
                }
            }
        }

        // Spawn shed debris particles into the dedicated debris pool
        for (x, y, vx, vy, kind, size_override, source_id) in shake_debris {
            particles.spawn_debris_particle(x, y, vx, vy, kind, size_override, source_id);
        }

        // 2. Diffuse and settle snow piles periodically
        accumulation.update_diffusion();

        // 3. Update debris particles (high-performance physical gravity and tumbling)
        for d in particles.debris.iter_mut() {
            if !d.active {
                continue;
            }

            d.life += dt;
            if d.life >= d.max_life || d.y > screen_h + 30.0 || d.x < -400.0 || d.x > screen_w + 400.0 {
                d.active = false;
                continue;
            }

            match d.kind {
                DebrisKind::Chunk => {
                    // Heavy snow chunk: strong gravity and tumbling rotation
                    d.vy += 760.0 * dt;
                    d.vx += (weather.current_wind_x * 0.4 - d.vx) * 0.8 * dt;
                    d.rotation += d.rotation_speed * dt;
                }
                DebrisKind::MediumPellet => {
                    // Medium cluster
                    d.vy += 620.0 * dt;
                    d.vx += (weather.current_wind_x * 0.6 - d.vx) * 1.1 * dt;
                    d.rotation += d.rotation_speed * dt;
                }
                DebrisKind::PowderDust => {
                    // Fine snow powder: floaty air resistance, soft billowing, fades out
                    d.vy += 380.0 * dt;
                    d.vx *= (1.0 - 1.4 * dt).max(0.1);
                    d.rotation += d.rotation_speed * dt;
                    d.size += 1.6 * dt;
                    d.opacity = d.base_opacity * (1.0 - d.life / d.max_life).max(0.0);
                }
            }

            // Integrate position
            d.x += d.vx * dt;
            d.y += d.vy * dt;

            // Collision with OTHER shelves only after clearing the source window
            if d.life > 0.85 && settings.snow_accumulation {
                // If it lands on another window shelf
                let fall_step = (d.vy * dt).abs();
                for shelf in accumulation.shelves.values_mut() {
                    if shelf.id != d.immune_window_id && shelf.try_accumulate(d.x, d.y, fall_step, d.size * 0.45) {
                        d.active = false;
                        break;
                    }
                }
            }
        }

        // 4. Update ambient snowflakes
        let wind_x = weather.current_wind_x;
        let wind_y = weather.current_wind_y;
        let fall_speed_mult = settings.fall_speed;

        for p in particles.particles.iter_mut() {
            if !p.active {
                continue;
            }

            p.lifetime += dt;

            // Wobble oscillation
            p.wobble_phase += p.wobble_speed * dt;
            let wobble_drift_x = (p.wobble_phase).sin() * p.wobble_amplitude * dt;

            // Rotation
            p.rotation += p.rotation_speed * dt;

            // Glitter twinkling
            if p.flake_type == FlakeType::Glitter {
                p.opacity = p.base_opacity * (0.6 + (p.wobble_phase * 2.5).sin().abs() * 0.4);
            }

            // Wind forces
            let wind_drag = match p.flake_type {
                FlakeType::SmallSoft => 1.15,
                FlakeType::MediumCrystal => 0.95,
                FlakeType::LargeFluff => 0.75,
                FlakeType::Glitter => 1.0,
            };

            p.vx += (wind_x * wind_drag - p.vx) * 1.8 * dt;

            // Downward velocity
            let target_vy = (p.base_fall_speed + wind_y * 0.4) * fall_speed_mult;
            p.vy += (target_vy - p.vy) * 2.2 * dt;

            // Cursor deflection
            if settings.cursor_interaction {
                cursor.apply_to_particle(p.x, p.y, &mut p.vx, &mut p.vy, dt);
            }

            // Integrate position
            p.x += (p.vx + wobble_drift_x) * dt;
            p.y += p.vy * dt;

            // Check collision with snow shelves (accumulation on application windows)
            let fall_delta_y = p.vy * dt;
            let collided = accumulation.check_collision_and_accumulate(
                p.x,
                p.y,
                fall_delta_y,
                p.flake_type,
                p.size,
                settings.snow_accumulation,
            );

            if collided {
                p.respawn_at_top(screen_w, -25.0, -5.0);
                continue;
            }

            // Screen boundary checks (flakes fall smoothly offscreen)
            if p.y > screen_h + 15.0 {
                p.respawn_at_top(screen_w, -30.0, -5.0);
            } else if p.x < -350.0 {
                p.x = screen_w + 150.0;
            } else if p.x > screen_w + 350.0 {
                p.x = -150.0;
            }
        }
    }
}
