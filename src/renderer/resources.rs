use std::f32::consts::PI;
use std::mem::ManuallyDrop;
use windows::core::Result;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

#[allow(dead_code)]
pub struct RenderResources {
    pub white_brush: ID2D1SolidColorBrush,
    pub mound_fill_brush: ID2D1SolidColorBrush,
    pub mound_highlight_brush: ID2D1SolidColorBrush,
    pub mound_shadow_brush: ID2D1SolidColorBrush,

    // Pre-rendered high-quality snowflake textures
    pub sprite_small_soft: ID2D1Bitmap1,
    pub sprite_medium_crystal: ID2D1Bitmap1,
    pub sprite_large_fluff: ID2D1Bitmap1,
    pub sprite_glitter: ID2D1Bitmap1,
}

impl RenderResources {
    pub fn new(d2d_factory: &ID2D1Factory1, d2d_dc: &ID2D1DeviceContext) -> Result<Self> {
        unsafe {
            let white_brush = d2d_dc.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
                None,
            )?;

            let mound_fill_brush = d2d_dc.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.95,
                    g: 0.97,
                    b: 1.0,
                    a: 0.92,
                },
                None,
            )?;

            let mound_highlight_brush = d2d_dc.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.70,
                },
                None,
            )?;

            let mound_shadow_brush = d2d_dc.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.82,
                    g: 0.88,
                    b: 0.96,
                    a: 0.45,
                },
                None,
            )?;

            let sprite_small_soft = Self::create_soft_disc_sprite(d2d_factory, d2d_dc, 48)?;
            let sprite_medium_crystal = Self::create_crystal_sprite(d2d_factory, d2d_dc, 64)?;
            let sprite_large_fluff = Self::create_fluff_sprite(d2d_factory, d2d_dc, 64)?;
            let sprite_glitter = Self::create_glitter_sprite(d2d_factory, d2d_dc, 48)?;

            Ok(Self {
                white_brush,
                mound_fill_brush,
                mound_highlight_brush,
                mound_shadow_brush,
                sprite_small_soft,
                sprite_medium_crystal,
                sprite_large_fluff,
                sprite_glitter,
            })
        }
    }

    unsafe fn create_soft_disc_sprite(
        _factory: &ID2D1Factory1,
        dc: &ID2D1DeviceContext,
        size: u32,
    ) -> Result<ID2D1Bitmap1> {
        let mut pixels = vec![0u8; (size * size * 4) as usize];
        let center = (size as f32 - 1.0) * 0.5;
        let radius = center * 0.92;

        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= radius {
                    let factor = (1.0 - dist / radius).clamp(0.0, 1.0);
                    let alpha = (factor.powf(1.8) * 255.0) as u8;

                    let idx = ((y * size + x) * 4) as usize;
                    pixels[idx] = alpha;     // B
                    pixels[idx + 1] = alpha; // G
                    pixels[idx + 2] = alpha; // R
                    pixels[idx + 3] = alpha; // A
                }
            }
        }

        let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
            colorContext: ManuallyDrop::new(None),
        };

        dc.CreateBitmap(
            D2D_SIZE_U {
                width: size,
                height: size,
            },
            Some(pixels.as_ptr() as *const _),
            size * 4,
            &bitmap_props,
        )
    }

    unsafe fn create_crystal_sprite(
        _factory: &ID2D1Factory1,
        dc: &ID2D1DeviceContext,
        size: u32,
    ) -> Result<ID2D1Bitmap1> {
        let mut pixels = vec![0u8; (size * size * 4) as usize];
        let center = (size as f32 - 1.0) * 0.5;
        let radius = center * 0.88;

        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= radius {
                    let angle = dy.atan2(dx);
                    let sector_angle = (angle % (PI / 3.0) + PI / 3.0) % (PI / 3.0);
                    let local_angle = (sector_angle - PI / 6.0).abs();

                    let d_spine = dist * local_angle.sin();
                    let spine_w = 1.4;

                    let mut intensity = 0.0f32;
                    if d_spine < spine_w {
                        intensity = (1.0 - d_spine / spine_w).max(intensity);
                    }

                    let r_norm = dist / radius;
                    if r_norm > 0.35 && r_norm < 0.75 {
                        let branch_dist = (r_norm - 0.55).abs();
                        if branch_dist < 0.12 && d_spine < 3.2 {
                            intensity = intensity.max(0.75 - branch_dist * 4.0);
                        }
                    }

                    if dist < 3.5 {
                        intensity = intensity.max(1.0 - dist / 3.5);
                    }

                    let alpha = (intensity.clamp(0.0, 1.0) * 255.0) as u8;
                    let idx = ((y * size + x) * 4) as usize;
                    pixels[idx] = alpha;
                    pixels[idx + 1] = alpha;
                    pixels[idx + 2] = alpha;
                    pixels[idx + 3] = alpha;
                }
            }
        }

        let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
            colorContext: ManuallyDrop::new(None),
        };

        dc.CreateBitmap(
            D2D_SIZE_U {
                width: size,
                height: size,
            },
            Some(pixels.as_ptr() as *const _),
            size * 4,
            &bitmap_props,
        )
    }

    unsafe fn create_fluff_sprite(
        _factory: &ID2D1Factory1,
        dc: &ID2D1DeviceContext,
        size: u32,
    ) -> Result<ID2D1Bitmap1> {
        let mut pixels = vec![0u8; (size * size * 4) as usize];
        let center = (size as f32 - 1.0) * 0.5;
        let radius = center * 0.9;

        let sub_centers = [
            (0.0f32, 0.0f32, 0.55f32),
            (-0.35, -0.2, 0.45),
            (0.35, -0.2, 0.45),
            (0.0, 0.38, 0.48),
            (-0.3, 0.3, 0.42),
            (0.3, 0.3, 0.42),
        ];

        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;

                let mut total_alpha = 0.0f32;
                for (scx, scy, sc_rad) in &sub_centers {
                    let sx = scx * radius;
                    let sy = scy * radius;
                    let s_rad = sc_rad * radius;
                    let dist = ((dx - sx) * (dx - sx) + (dy - sy) * (dy - sy)).sqrt();
                    if dist <= s_rad {
                        let falloff = (1.0 - dist / s_rad).clamp(0.0, 1.0).powf(1.4);
                        total_alpha += falloff * 0.45;
                    }
                }

                let alpha = (total_alpha.clamp(0.0, 1.0) * 255.0) as u8;
                let idx = ((y * size + x) * 4) as usize;
                pixels[idx] = alpha;
                pixels[idx + 1] = alpha;
                pixels[idx + 2] = alpha;
                pixels[idx + 3] = alpha;
            }
        }

        let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
            colorContext: ManuallyDrop::new(None),
        };

        dc.CreateBitmap(
            D2D_SIZE_U {
                width: size,
                height: size,
            },
            Some(pixels.as_ptr() as *const _),
            size * 4,
            &bitmap_props,
        )
    }

    unsafe fn create_glitter_sprite(
        _factory: &ID2D1Factory1,
        dc: &ID2D1DeviceContext,
        size: u32,
    ) -> Result<ID2D1Bitmap1> {
        let mut pixels = vec![0u8; (size * size * 4) as usize];
        let center = (size as f32 - 1.0) * 0.5;
        let radius = center * 0.9;

        for y in 0..size {
            for x in 0..size {
                let dx = (x as f32 - center).abs();
                let dy = (y as f32 - center).abs();

                let ray_h = if dy < 2.0 && dx <= radius {
                    (1.0 - dx / radius) * (1.0 - dy / 2.0)
                } else {
                    0.0
                };

                let ray_v = if dx < 2.0 && dy <= radius {
                    (1.0 - dy / radius) * (1.0 - dx / 2.0)
                } else {
                    0.0
                };

                let core_dist = (dx * dx + dy * dy).sqrt();
                let core = if core_dist < 4.5 {
                    (1.0 - core_dist / 4.5).powi(2)
                } else {
                    0.0
                };

                let intensity = (ray_h + ray_v + core).clamp(0.0, 1.0);
                let alpha = (intensity * 255.0) as u8;
                let idx = ((y * size + x) * 4) as usize;
                pixels[idx] = alpha;
                pixels[idx + 1] = alpha;
                pixels[idx + 2] = alpha;
                pixels[idx + 3] = alpha;
            }
        }

        let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
            colorContext: ManuallyDrop::new(None),
        };

        dc.CreateBitmap(
            D2D_SIZE_U {
                width: size,
                height: size,
            },
            Some(pixels.as_ptr() as *const _),
            size * 4,
            &bitmap_props,
        )
    }
}
