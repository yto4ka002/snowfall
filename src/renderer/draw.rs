use crate::accumulation::{AccumulationSystem, COLUMN_WIDTH};
use crate::particle::{DebrisKind, DebrisParticle, FlakeType, ParticleSystem};
use crate::renderer::context::RenderContext;
use crate::renderer::resources::RenderResources;
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
        occluders: Option<&[(f32, f32, f32, f32)]>,
        virtual_x: f32,
        virtual_y: f32,
    ) -> Result<()> {
        let dc = &ctx.d2d_dc;
        let factory = &ctx.d2d_factory;

        // 1. Draw Accumulated Snow Mounds on application windows
        Self::draw_shelf_mounds(dc, factory, resources, accumulation, virtual_x, virtual_y)?;

        // 2. Draw Falling Ambient Snowflakes (optionally occluded by open application windows)
        Self::draw_particles(dc, resources, particles, occluders, virtual_x, virtual_y)?;

        // 3. Draw Dislodged Falling Debris (avalanche chunks, pellets, powder mist)
        Self::draw_debris(dc, resources, &particles.debris, virtual_x, virtual_y)?;

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
        virt_x: f32,
        virt_y: f32,
    ) -> Result<()> {
        unsafe {
            for shelf in accumulation.shelves.values() {
                Self::draw_single_shelf(dc, factory, resources, shelf, virt_x, virt_y)?;
            }
        }
        Ok(())
    }

    unsafe fn draw_single_shelf(
        dc: &ID2D1DeviceContext,
        factory: &ID2D1Factory1,
        resources: &RenderResources,
        shelf: &crate::accumulation::SnowShelf,
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

            // Fill with soft snow mound brush
            dc.FillGeometry(&fill_geo, &resources.mound_fill_brush, None);

            // Subtle cool shadow along the bottom contact ledge for 3D volume
            if peak_h >= 3.5 {
                dc.DrawLine(
                    Vector2 {
                        X: left_anchor_x + 2.0,
                        Y: base_y - 0.5,
                    },
                    Vector2 {
                        X: right_anchor_x - 2.0,
                        Y: base_y - 0.5,
                    },
                    &resources.mound_shadow_brush,
                    1.2,
                    None,
                );
            }

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
                            0.45,
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
}
