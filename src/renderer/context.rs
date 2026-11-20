use std::mem::ManuallyDrop;
use windows::core::{Interface, Result, BOOL};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Dxgi::Common::*;

#[allow(dead_code)]
pub struct RenderContext {
    pub hwnd: HWND,
    pub width: u32,
    pub height: u32,
    pub d3d_device: ID3D11Device,
    pub d3d_context: ID3D11DeviceContext,
    pub dxgi_device: IDXGIDevice,
    pub dxgi_factory: IDXGIFactory2,
    pub swap_chain: IDXGISwapChain1,
    pub dcomp_device: IDCompositionDevice,
    pub dcomp_target: IDCompositionTarget,
    pub dcomp_visual: IDCompositionVisual,
    pub d2d_factory: ID2D1Factory1,
    pub d2d_device: ID2D1Device,
    pub d2d_dc: ID2D1DeviceContext,
    pub target_bitmap: ID2D1Bitmap1,
}

impl RenderContext {
    pub fn new(hwnd: HWND, width: u32, height: u32) -> Result<Self> {
        unsafe {
            let width = width.max(100);
            let height = height.max(100);

            let mut d3d_device: Option<ID3D11Device> = None;
            let mut d3d_context: Option<ID3D11DeviceContext> = None;
            let feature_levels = [
                D3D_FEATURE_LEVEL_11_1,
                D3D_FEATURE_LEVEL_11_0,
                D3D_FEATURE_LEVEL_10_1,
            ];

            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut d3d_device),
                None,
                Some(&mut d3d_context),
            )?;

            let d3d_device = d3d_device.unwrap();
            let d3d_context = d3d_context.unwrap();
            let dxgi_device: IDXGIDevice = d3d_device.cast()?;

            let dxgi_adapter = dxgi_device.GetAdapter()?;
            let dxgi_factory: IDXGIFactory2 = dxgi_adapter.GetParent()?;

            let swapchain_desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: width,
                Height: height,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                Stereo: BOOL(0),
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                Scaling: DXGI_SCALING_STRETCH,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
                Flags: 0,
            };

            let swap_chain = dxgi_factory.CreateSwapChainForComposition(
                &dxgi_device,
                &swapchain_desc,
                None,
            )?;

            let dcomp_device: IDCompositionDevice = DCompositionCreateDevice(&dxgi_device)?;
            let dcomp_target = dcomp_device.CreateTargetForHwnd(hwnd, true)?;
            let dcomp_visual = dcomp_device.CreateVisual()?;

            dcomp_visual.SetContent(&swap_chain)?;
            dcomp_target.SetRoot(&dcomp_visual)?;
            dcomp_device.Commit()?;

            let d2d_factory: ID2D1Factory1 = D2D1CreateFactory(
                D2D1_FACTORY_TYPE_SINGLE_THREADED,
                None,
            )?;
            let d2d_device = d2d_factory.CreateDevice(&dxgi_device)?;
            let d2d_dc = d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;

            let surface: IDXGISurface = swap_chain.GetBuffer(0)?;
            let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                colorContext: ManuallyDrop::new(None),
            };

            let target_bitmap = d2d_dc.CreateBitmapFromDxgiSurface(&surface, Some(&bitmap_props))?;
            d2d_dc.SetTarget(&target_bitmap);

            Ok(Self {
                hwnd,
                width,
                height,
                d3d_device,
                d3d_context,
                dxgi_device,
                dxgi_factory,
                swap_chain,
                dcomp_device,
                dcomp_target,
                dcomp_visual,
                d2d_factory,
                d2d_device,
                d2d_dc,
                target_bitmap,
            })
        }
    }

    pub fn resize(&mut self, new_w: u32, new_h: u32) -> Result<()> {
        let new_w = new_w.max(100);
        let new_h = new_h.max(100);
        if self.width == new_w && self.height == new_h {
            return Ok(());
        }

        self.width = new_w;
        self.height = new_h;

        unsafe {
            self.d2d_dc.SetTarget(None);

            self.swap_chain.ResizeBuffers(
                2,
                new_w,
                new_h,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_SWAP_CHAIN_FLAG(0),
            )?;

            let surface: IDXGISurface = self.swap_chain.GetBuffer(0)?;
            let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                colorContext: ManuallyDrop::new(None),
            };

            self.target_bitmap = self.d2d_dc.CreateBitmapFromDxgiSurface(&surface, Some(&bitmap_props))?;
            self.d2d_dc.SetTarget(&self.target_bitmap);
        }

        Ok(())
    }

    pub fn begin_draw(&self) {
        unsafe {
            self.d2d_dc.BeginDraw();
            self.d2d_dc.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));
        }
    }

    pub fn end_draw_and_present(&mut self, sync_interval: u32) -> Result<()> {
        unsafe {
            self.d2d_dc.EndDraw(None, None)?;
            let hr = self.swap_chain.Present(sync_interval, DXGI_PRESENT(0));
            if hr.is_err() {
                let new_ctx = Self::new(self.hwnd, self.width, self.height)?;
                *self = new_ctx;
            }
        }
        Ok(())
    }
}
