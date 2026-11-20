pub mod context;
pub mod draw;
pub mod overlay_window;
pub mod resources;

use crate::accumulation::AccumulationSystem;
use crate::particle::ParticleSystem;
use context::RenderContext;
use draw::DrawEngine;
use overlay_window::OverlayWindow;
use resources::RenderResources;
use windows::core::Result;

pub struct Renderer {
    pub overlay: OverlayWindow,
    pub context: RenderContext,
    pub resources: RenderResources,
}

impl Renderer {
    pub fn new() -> Result<Self> {
        let overlay = OverlayWindow::new()?;
        let context = RenderContext::new(
            overlay.hwnd,
            overlay.width as u32,
            overlay.height as u32,
        )?;
        let resources = RenderResources::new(&context.d2d_factory, &context.d2d_dc)?;

        Ok(Self {
            overlay,
            context,
            resources,
        })
    }

    pub fn check_resize(&mut self) -> Result<()> {
        if self.overlay.update_bounds() {
            self.context
                .resize(self.overlay.width as u32, self.overlay.height as u32)?;
        }
        Ok(())
    }

    pub fn render(
        &mut self,
        particles: &ParticleSystem,
        accumulation: &AccumulationSystem,
        window_tracker: &crate::window_tracker::WindowTracker,
        settings: &crate::config::Settings,
    ) -> Result<()> {
        self.context.begin_draw();

        let occluders = if !settings.snow_over_windows {
            Some(
                window_tracker
                    .tracked_windows
                    .values()
                    .map(|r| {
                        (
                            r.rect.left as f32,
                            r.rect.right as f32,
                            r.rect.top as f32 + 4.0,
                            r.rect.bottom as f32,
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };

        DrawEngine::draw_frame(
            &self.context,
            &self.resources,
            particles,
            accumulation,
            occluders.as_deref(),
            self.overlay.x as f32,
            self.overlay.y as f32,
        )?;

        let sync_interval = if settings.target_fps <= 30 { 2 } else { 1 };
        self.context.end_draw_and_present(sync_interval)
    }
}
