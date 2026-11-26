use crate::accumulation::{AccumulationSystem, COLUMN_WIDTH};
use crate::particle::{DebrisKind, DebrisParticle, FlakeType, ParticleSystem};
use crate::renderer::context::RenderContext;
use crate::renderer::resources::RenderResources;
use crate::tractor::TractorSystem;
use windows::core::Result;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows_numerics::{Matrix3x2, Vector2};

pub struct DrawEngine;

impl DrawEngine {
    pub fn draw_frame(
        ctx: &RenderContext,
        resources: &RenderResources,
        particles: &ParticleSystem,
        accumulation: &AccumulationSystem,
        tractor: &TractorSystem,
        accumulation_opacity: f32,
        occluders: Option<&[(f32, f32, f32, f32)]>,
        virtual_x: f32,
        virtual_y: f32,
    ) -> Result<()> {
        let dc = &ctx.d2d_dc;
        let factory = &ctx.d2d_factory;

        // 1. Draw Accumulated Snow Mounds on application windows
        Self::draw_shelf_mounds(
            dc,
            factory,
            resources,
            accumulation,
            accumulation_opacity,
            virtual_x,
            virtual_y,
        )?;

        // 2. Draw Snowplow Tractor clearing snow along window ledges
        Self::draw_tractor(dc, factory, resources, tractor, accumulation_opacity, virtual_x, virtual_y)?;

        // 3. Draw Dislodged Falling Debris (avalanche chunks, pellets, powder mist)
        Self::draw_debris(dc, resources, &particles.debris, virtual_x, virtual_y)?;

        // 4. Draw Falling Ambient Snowflakes (optionally occluded by open application windows)
        Self::draw_particles(dc, resources, particles, occluders, virtual_x, virtual_y)?;

        Ok(())
    }

    fn draw_particles(
        dc: &ID2D1DeviceContext,
        resources: &RenderResources,
        particles: &ParticleSystem,
        occluders: Option<&[(f32, f32, f32, f32)]>,
        virt_x: f32,
        virt_y: f32,
    ) -> Result<()> {
        unsafe {
            let mut transform_dirty = false;

            for p in &particles.particles {
                if !p.active {
                    continue;
                }

                // If snow should not fall over applications, skip snowflakes falling over open windows
                if let Some(rects) = occluders {
                    let mut occluded = false;
                    for &(x1, x2, y1, y2) in rects {
                        if p.x >= x1 && p.x <= x2 && p.y >= y1 && p.y <= y2 {
                            occluded = true;
                            break;
                        }
                    }
                    if occluded {
                        continue;
                    }
                }

                let rx = p.x - virt_x;
                let ry = p.y - virt_y;

                let half_size = p.size;
                let dest_rect = D2D_RECT_F {
                    left: rx - half_size,
                    top: ry - half_size,
                    right: rx + half_size,
                    bottom: ry + half_size,
                };

                let sprite = match p.flake_type {
                    FlakeType::SmallSoft => &resources.sprite_small_soft,
                    FlakeType::MediumCrystal => &resources.sprite_medium_crystal,
                    FlakeType::LargeFluff => &resources.sprite_large_fluff,
                    FlakeType::Glitter => &resources.sprite_glitter,
                };

                if p.rotation.abs() > 0.01 {
                    let rot_deg = p.rotation.to_degrees();
                    let rot_mat = Matrix3x2::rotation_around(rot_deg, Vector2 { X: rx, Y: ry });
                    dc.SetTransform(&rot_mat);
                    transform_dirty = true;
                } else if transform_dirty {
                    dc.SetTransform(&Matrix3x2::identity());
                    transform_dirty = false;
                }

                dc.DrawBitmap(
                    sprite,
                    Some(&dest_rect),
                    p.opacity,
                    D2D1_INTERPOLATION_MODE_LINEAR,
                    None,
                    None,
                );
            }

            if transform_dirty {
                dc.SetTransform(&Matrix3x2::identity());
            }
        }
        Ok(())
    }

    fn draw_debris(
        dc: &ID2D1DeviceContext,
        resources: &RenderResources,
        debris: &[DebrisParticle],
        virt_x: f32,
        virt_y: f32,
    ) -> Result<()> {
        unsafe {
            let mut transform_dirty = false;

            for d in debris {
                if !d.active {
                    continue;
                }

                let rx = d.x - virt_x;
                let ry = d.y - virt_y;

                let half_size = d.size;
                let dest_rect = D2D_RECT_F {
                    left: rx - half_size,
                    top: ry - half_size,
                    right: rx + half_size,
                    bottom: ry + half_size,
                };

                let sprite = match d.kind {
                    DebrisKind::Chunk => &resources.sprite_large_fluff,
                    DebrisKind::MediumPellet => &resources.sprite_medium_crystal,
                    DebrisKind::PowderDust => &resources.sprite_small_soft,
                };

                if d.rotation.abs() > 0.01 {
                    let rot_deg = d.rotation.to_degrees();
                    let rot_mat = Matrix3x2::rotation_around(rot_deg, Vector2 { X: rx, Y: ry });
                    dc.SetTransform(&rot_mat);
                    transform_dirty = true;
                } else if transform_dirty {
                    dc.SetTransform(&Matrix3x2::identity());
                    transform_dirty = false;
                }

                dc.DrawBitmap(
                    sprite,
                    Some(&dest_rect),
                    d.opacity,
                    D2D1_INTERPOLATION_MODE_LINEAR,
                    None,
                    None,
                );
            }

            if transform_dirty {
                dc.SetTransform(&Matrix3x2::identity());
            }
        }
        Ok(())
    }

    fn draw_shelf_mounds(
        dc: &ID2D1DeviceContext,
        factory: &ID2D1Factory1,
        resources: &RenderResources,
        accumulation: &AccumulationSystem,
        accumulation_opacity: f32,
        virt_x: f32,
        virt_y: f32,
    ) -> Result<()> {
        let op = accumulation_opacity.clamp(0.15, 1.0);
        unsafe {
            resources.mound_fill_brush.SetOpacity(op);
            resources.mound_outline_brush.SetOpacity(0.62 * op);
            resources.mound_highlight_brush.SetOpacity(0.85 * op);
            for shelf in accumulation.shelves.values() {
                Self::draw_single_shelf(dc, factory, resources, shelf, op, virt_x, virt_y)?;
            }
            resources.mound_fill_brush.SetOpacity(1.0);
            resources.mound_outline_brush.SetOpacity(1.0);
            resources.mound_highlight_brush.SetOpacity(1.0);
        }
        Ok(())
    }

    unsafe fn draw_single_shelf(
        dc: &ID2D1DeviceContext,
        factory: &ID2D1Factory1,
        resources: &RenderResources,
        shelf: &crate::accumulation::SnowShelf,
        accumulation_opacity: f32,
        virt_x: f32,
        virt_y: f32,
    ) -> Result<()> {
        let num_cols = shelf.heights.len();
        if num_cols < 2 {
            return Ok(());
        }

        let sx1 = shelf.x1 - virt_x;
        let sx2 = shelf.x2 - virt_x;
        let base_y = shelf.base_y - virt_y;

        // Group into contiguous snow mounds with height >= 0.45 px
        let min_threshold = 0.45f32;
        let mut c = 0;
        while c < num_cols {
            if shelf.heights[c] < min_threshold {
                c += 1;
                continue;
            }

            let start = c;
            let mut peak_h = shelf.heights[c];
            while c < num_cols && shelf.heights[c] >= min_threshold {
                if shelf.heights[c] > peak_h {
                    peak_h = shelf.heights[c];
                }
                c += 1;
            }
            let end = c - 1;

            // Render when snow has formed a visible mound (>= 0.65 px)
            if peak_h < 0.65 {
                continue;
            }

            let fill_geo = factory.CreatePathGeometry()?;
            let sink = fill_geo.Open()?;

            // Left anchor point on the ledge
            let left_anchor_x = (sx1 + start as f32 * COLUMN_WIDTH - 2.0).max(sx1);
            let right_anchor_x = (sx1 + (end + 1) as f32 * COLUMN_WIDTH + 2.0).min(sx2);

            sink.BeginFigure(
                Vector2 {
                    X: left_anchor_x,
                    Y: base_y,
                },
                D2D1_FIGURE_BEGIN_FILLED,
            );

            // Left slope: smoothly transition from base_y up to start column
            let start_pt_x = sx1 + start as f32 * COLUMN_WIDTH + COLUMN_WIDTH * 0.5;
            let start_pt_y = base_y - shelf.heights[start];
            sink.AddLine(Vector2 {
                X: (left_anchor_x + start_pt_x) * 0.5,
                Y: (base_y + start_pt_y) * 0.5,
            });
            sink.AddLine(Vector2 {
                X: start_pt_x,
                Y: start_pt_y,
            });

            // Smooth upper contour using midpoint Quadratic Beziers
            let mut prev_x = start_pt_x;
            let mut prev_y = start_pt_y;
            for i in (start + 1)..=end {
                let cur_x = sx1 + i as f32 * COLUMN_WIDTH + COLUMN_WIDTH * 0.5;
                let cur_y = base_y - shelf.heights[i];
                let mid_x = (prev_x + cur_x) * 0.5;
                let mid_y = (prev_y + cur_y) * 0.5;
                sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
                    point1: Vector2 { X: prev_x, Y: prev_y },
                    point2: Vector2 { X: mid_x, Y: mid_y },
                });
                prev_x = cur_x;
                prev_y = cur_y;
            }
            sink.AddLine(Vector2 { X: prev_x, Y: prev_y });

            // Right slope: smoothly transition down to base_y
            let end_pt_x = sx1 + end as f32 * COLUMN_WIDTH + COLUMN_WIDTH * 0.5;
            let end_pt_y = base_y - shelf.heights[end];
            sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
                point1: Vector2 {
                    X: (end_pt_x + right_anchor_x) * 0.5,
                    Y: (base_y + end_pt_y) * 0.5,
                },
                point2: Vector2 {
                    X: right_anchor_x,
                    Y: base_y,
                },
            });

            // Close figure along the bottom ledge
            sink.AddLine(Vector2 {
                X: left_anchor_x,
                Y: base_y,
            });
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            sink.Close()?;

            // Subtle cool ice-blue contour around the mound so snow is visible on white backgrounds
            dc.DrawGeometry(&fill_geo, &resources.mound_outline_brush, 1.8, None);

            // Fill with soft snow mound brush
            dc.FillGeometry(&fill_geo, &resources.mound_fill_brush, None);

            // Soft frosty highlight on top crest ONLY if snow is thick enough (>= 1.6 px)
            if peak_h >= 1.6 {
                let crust_geo = factory.CreatePathGeometry()?;
                let crust_sink = crust_geo.Open()?;

                crust_sink.BeginFigure(
                    Vector2 {
                        X: start_pt_x,
                        Y: start_pt_y,
                    },
                    D2D1_FIGURE_BEGIN_HOLLOW,
                );

                let mut c_prev_x = start_pt_x;
                let mut c_prev_y = start_pt_y;
                for i in (start + 1)..=end {
                    let cur_x = sx1 + i as f32 * COLUMN_WIDTH + COLUMN_WIDTH * 0.5;
                    let cur_y = base_y - shelf.heights[i];
                    let mid_x = (c_prev_x + cur_x) * 0.5;
                    let mid_y = (c_prev_y + cur_y) * 0.5;
                    crust_sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
                        point1: Vector2 { X: c_prev_x, Y: c_prev_y },
                        point2: Vector2 { X: mid_x, Y: mid_y },
                    });
                    c_prev_x = cur_x;
                    c_prev_y = cur_y;
                }
                crust_sink.AddLine(Vector2 { X: c_prev_x, Y: c_prev_y });

                crust_sink.EndFigure(D2D1_FIGURE_END_OPEN);
                crust_sink.Close()?;

                dc.DrawGeometry(&crust_geo, &resources.mound_highlight_brush, 0.9, None);
            }

            // Fluffy powder puffs on local peaks for thick snow caps (>= 3.0 px)
            if peak_h >= 3.0 {
                for i in start..=end {
                    let h = shelf.heights[i];
                    if h >= 2.5 && (i % 2 == 0) {
                        let px = sx1 + i as f32 * COLUMN_WIDTH + COLUMN_WIDTH * 0.5;
                        let py = base_y - h;
                        let puff_size = (h * 0.5).clamp(3.0, 7.5);
                        let dest_rect = D2D_RECT_F {
                            left: px - puff_size,
                            top: py - puff_size * 0.7,
                            right: px + puff_size,
                            bottom: py + puff_size * 0.7,
                        };
                        dc.DrawBitmap(
                            &resources.sprite_small_soft,
                            Some(&dest_rect),
                            0.45 * accumulation_opacity,
                            D2D1_INTERPOLATION_MODE_LINEAR,
                            None,
                            None,
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn draw_tractor(
        dc: &ID2D1DeviceContext,
        factory: &ID2D1Factory1,
        resources: &RenderResources,
        tractor: &TractorSystem,
        accumulation_opacity: f32,
        virt_x: f32,
        virt_y: f32,
    ) -> Result<()> {
        unsafe {
            // 1. Always render active exhaust puffs in world space
            for puff in &tractor.exhaust {
                if !puff.active {
                    continue;
                }
                let t = (puff.life / puff.max_life).clamp(0.0, 1.0);
                resources.tractor_smoke_brush.SetOpacity(0.55 * (1.0 - t));
                dc.FillEllipse(
                    &D2D1_ELLIPSE {
                        point: Vector2 {
                            X: puff.x - virt_x,
                            Y: puff.y - virt_y,
                        },
                        radiusX: puff.size,
                        radiusY: puff.size * 0.82,
                    },
                    &resources.tractor_smoke_brush,
                );
            }
            resources.tractor_smoke_brush.SetOpacity(0.55);

            if !tractor.active || tractor.alpha <= 0.01 {
                return Ok(());
            }

            let alpha = tractor.alpha;
            resources.tractor_body_brush.SetOpacity(alpha);
            resources.tractor_dark_brush.SetOpacity(alpha);
            resources.tractor_glass_brush.SetOpacity(0.9 * alpha);
            resources.tractor_blade_brush.SetOpacity(alpha);

            let rx = tractor.x - virt_x;
            let ry = tractor.y - virt_y;
            let bounce_y = (tractor.bounce_phase).sin() * 0.55;

            let mat = Matrix3x2 {
                M11: tractor.direction,
                M12: 0.0,
                M21: 0.0,
                M22: 1.0,
                M31: rx,
                M32: ry + bounce_y,
            };
            dc.SetTransform(&mat);

            // 2. Pushed snow wave right in front of the plow blade
            if tractor.snow_wave > 0.6 {
                let w_h = tractor.snow_wave.clamp(1.5, 11.5);
                let wave_ellipse = D2D1_ELLIPSE {
                    point: Vector2 {
                        X: 25.0,
                        Y: -w_h * 0.42,
                    },
                    radiusX: 5.5,
                    radiusY: w_h * 0.52,
                };
                let op = accumulation_opacity.clamp(0.15, 1.0);
                resources.mound_outline_brush.SetOpacity(0.62 * op * alpha);
                resources.mound_fill_brush.SetOpacity(op * alpha);
                dc.DrawEllipse(&wave_ellipse, &resources.mound_outline_brush, 1.4, None);
                dc.FillEllipse(&wave_ellipse, &resources.mound_fill_brush);
                resources.mound_outline_brush.SetOpacity(1.0);
                resources.mound_fill_brush.SetOpacity(1.0);
            }

            // 3. Exhaust pipe on hood
            let exhaust_rect = D2D_RECT_F {
                left: 6.5,
                top: -25.0,
                right: 9.0,
                bottom: -18.0,
            };
            dc.FillRectangle(&exhaust_rect, &resources.tractor_dark_brush);

            // 4. Engine hood
            let hood_rect = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: -1.0,
                    top: -18.5,
                    right: 14.5,
                    bottom: -9.5,
                },
                radiusX: 2.0,
                radiusY: 2.0,
            };
            dc.FillRoundedRectangle(&hood_rect, &resources.tractor_body_brush);
            dc.DrawRoundedRectangle(&hood_rect, &resources.tractor_dark_brush, 1.2, None);

            // Hood grille vents
            dc.DrawLine(
                Vector2 { X: 10.5, Y: -16.0 },
                Vector2 { X: 10.5, Y: -11.8 },
                &resources.tractor_dark_brush,
                1.0,
                None,
            );
            dc.DrawLine(
                Vector2 { X: 12.5, Y: -16.0 },
                Vector2 { X: 12.5, Y: -11.8 },
                &resources.tractor_dark_brush,
                1.0,
                None,
            );

            // Warm headlight on front
            resources.tractor_beacon_brush.SetOpacity(0.95 * alpha);
            dc.FillEllipse(
                &D2D1_ELLIPSE {
                    point: Vector2 { X: 14.8, Y: -13.8 },
                    radiusX: 1.8,
                    radiusY: 2.0,
                },
                &resources.tractor_beacon_brush,
            );

            // 5. Rear Cab + Window + Roof
            let cab_rect = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: -15.5,
                    top: -25.5,
                    right: 0.5,
                    bottom: -9.5,
                },
                radiusX: 2.0,
                radiusY: 2.0,
            };
            dc.FillRoundedRectangle(&cab_rect, &resources.tractor_body_brush);
            dc.DrawRoundedRectangle(&cab_rect, &resources.tractor_dark_brush, 1.2, None);

            let cab_roof = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: -17.0,
                    top: -27.0,
                    right: 2.0,
                    bottom: -24.2,
                },
                radiusX: 1.0,
                radiusY: 1.0,
            };
            dc.FillRoundedRectangle(&cab_roof, &resources.tractor_body_brush);
            dc.DrawRoundedRectangle(&cab_roof, &resources.tractor_dark_brush, 1.0, None);

            let window_rect = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: -12.8,
                    top: -22.6,
                    right: -2.0,
                    bottom: -14.2,
                },
                radiusX: 1.2,
                radiusY: 1.2,
            };
            dc.FillRoundedRectangle(&window_rect, &resources.tractor_glass_brush);
            dc.DrawRoundedRectangle(&window_rect, &resources.tractor_dark_brush, 1.0, None);

            // 6. Blinking amber roof beacon
            let beacon_pulse = 0.55 + 0.45 * (tractor.beacon_phase).sin();
            dc.FillRectangle(
                &D2D_RECT_F {
                    left: -9.2,
                    top: -29.0,
                    right: -5.8,
                    bottom: -27.0,
                },
                &resources.tractor_dark_brush,
            );
            resources.tractor_beacon_brush.SetOpacity((0.4 + 0.6 * beacon_pulse) * alpha);
            dc.FillEllipse(
                &D2D1_ELLIPSE {
                    point: Vector2 { X: -7.5, Y: -30.2 },
                    radiusX: 2.3,
                    radiusY: 2.2,
                },
                &resources.tractor_beacon_brush,
            );
            if beacon_pulse > 0.65 {
                resources.tractor_beacon_brush.SetOpacity(0.28 * beacon_pulse * alpha);
                dc.FillEllipse(
                    &D2D1_ELLIPSE {
                        point: Vector2 { X: -7.5, Y: -30.2 },
                        radiusX: 4.8,
                        radiusY: 4.5,
                    },
                    &resources.tractor_beacon_brush,
                );
            }

            // 7. Curved Front Snowplow Blade + Push Arm
            dc.DrawLine(
                Vector2 { X: 11.5, Y: -8.5 },
                Vector2 { X: 19.0, Y: -5.5 },
                &resources.tractor_dark_brush,
                2.2,
                None,
            );

            let blade_geo = factory.CreatePathGeometry()?;
            let blade_sink = blade_geo.Open()?;
            blade_sink.BeginFigure(Vector2 { X: 22.5, Y: -14.5 }, D2D1_FIGURE_BEGIN_FILLED);
            blade_sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
                point1: Vector2 { X: 18.2, Y: -7.0 },
                point2: Vector2 { X: 23.5, Y: 0.0 },
            });
            blade_sink.AddLine(Vector2 { X: 17.8, Y: -0.6 });
            blade_sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
                point1: Vector2 { X: 15.2, Y: -6.8 },
                point2: Vector2 { X: 18.2, Y: -13.5 },
            });
            blade_sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            blade_sink.Close()?;

            dc.FillGeometry(&blade_geo, &resources.tractor_blade_brush, None);
            dc.DrawGeometry(&blade_geo, &resources.tractor_dark_brush, 1.25, None);

            // 8. Spinning Tractor Wheels (Big Rear + Front)
            let rear_c = Vector2 { X: -9.5, Y: -6.8 };
            let rear_r = 6.8f32;
            dc.FillEllipse(
                &D2D1_ELLIPSE {
                    point: rear_c,
                    radiusX: rear_r,
                    radiusY: rear_r,
                },
                &resources.tractor_dark_brush,
            );
            dc.FillEllipse(
                &D2D1_ELLIPSE {
                    point: rear_c,
                    radiusX: 3.1,
                    radiusY: 3.1,
                },
                &resources.tractor_body_brush,
            );
            let sc = tractor.wheel_phase.cos() * 2.8;
            let ss = tractor.wheel_phase.sin() * 2.8;
            dc.DrawLine(
                Vector2 { X: rear_c.X - sc, Y: rear_c.Y - ss },
                Vector2 { X: rear_c.X + sc, Y: rear_c.Y + ss },
                &resources.tractor_dark_brush,
                1.1,
                None,
            );
            dc.DrawLine(
                Vector2 { X: rear_c.X + ss, Y: rear_c.Y - sc },
                Vector2 { X: rear_c.X - ss, Y: rear_c.Y + sc },
                &resources.tractor_dark_brush,
                1.1,
                None,
            );

            let front_c = Vector2 { X: 10.0, Y: -4.6 };
            let front_r = 4.6f32;
            dc.FillEllipse(
                &D2D1_ELLIPSE {
                    point: front_c,
                    radiusX: front_r,
                    radiusY: front_r,
                },
                &resources.tractor_dark_brush,
            );
            dc.FillEllipse(
                &D2D1_ELLIPSE {
                    point: front_c,
                    radiusX: 2.0,
                    radiusY: 2.0,
                },
                &resources.tractor_body_brush,
            );
            let fc = (tractor.wheel_phase * 1.48).cos() * 1.8;
            let fs = (tractor.wheel_phase * 1.48).sin() * 1.8;
            dc.DrawLine(
                Vector2 { X: front_c.X - fc, Y: front_c.Y - fs },
                Vector2 { X: front_c.X + fc, Y: front_c.Y + fs },
                &resources.tractor_dark_brush,
                1.0,
                None,
            );

            // Restore transform and brush opacities
            dc.SetTransform(&Matrix3x2::identity());
            resources.tractor_body_brush.SetOpacity(1.0);
            resources.tractor_dark_brush.SetOpacity(1.0);
            resources.tractor_glass_brush.SetOpacity(0.9);
            resources.tractor_blade_brush.SetOpacity(1.0);
            resources.tractor_beacon_brush.SetOpacity(1.0);
        }

        Ok(())
    }
}
