# ❄️ SNOWFALL — Interactive Windows 11 Desktop Snow Simulator

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%2011%20%7C%2010-blue?logo=windows" alt="Platform" />
  <img src="https://img.shields.io/badge/Language-Rust%202021-orange?logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/Graphics-Direct2D%20%2B%20DirectComposition-cyan" alt="Graphics" />
  <img src="https://img.shields.io/badge/Binary%20Size-~490%20KB-success" alt="Size" />
  <img src="https://img.shields.io/badge/License-MIT-green" alt="License" />
</p>

<p align="center">
  <b>English</b> | <a href="README_RU.md">Русский</a>
</p>

<p align="center">
  <a href="https://github.com/yto4ka002/snowfall/releases/latest/download/snowfall.exe">
    <img src="https://img.shields.io/badge/Download-snowfall.exe%20(~490%20KB)-2ea44f?style=for-the-badge&logo=windows" alt="Download snowfall.exe" />
  </a>
  <a href="https://github.com/yto4ka002/snowfall/raw/main/snowfall.exe">
    <img src="https://img.shields.io/badge/Direct%20Repo-snowfall.exe-blue?style=for-the-badge&logo=github" alt="Direct Repo Download" />
  </a>
</p>

---

**Snowfall** is a lightweight, ultra-high-performance native desktop snow simulator crafted specifically for Windows 11 and Windows 10. Built entirely in **Rust** using **Direct2D**, **Direct3D 11**, and **DirectComposition**, it creates an interactive winter atmosphere directly on top of your desktop and application windows.

Unlike static desktop wallpapers or heavy Electron/web overlays, Snowfall features a **real-time particle system**, **procedural snow accumulation on window borders**, **Aero Shake snow disintegration physics**, and **cursor aerodynamics** — running at an effortless 60 FPS with minimal CPU/GPU usage (< 1% on modern hardware, and exactly 0% when paused or in full-screen games).

---

## 🌟 Key Features

### 🖥️ 1. Transparent Click-Through Overlay
- **DirectComposition Hardware Acceleration**: Uses `DXGI_ALPHA_MODE_PREMULTIPLIED` for seamless per-pixel alpha blending directly composed by the Windows Desktop Window Manager (DWM).
- **100% Click-Through**: Built with `WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP` — clicks, typing, and window dragging pass straight through without interference.
- **Multi-Monitor Support**: Automatically covers the entire virtual desktop area across all connected monitors, with dynamic resolution and DPI change handling (Per-Monitor V2).
- **No Clutter**: Completely hidden from the Taskbar and `Alt+Tab` switcher (`WS_EX_TOOLWINDOW`).

### ❄️ 2. Natural Particle System
- **4 Distinct Flake Types**:
  - `SmallSoft`: Distant fine flakes ($z \in [0.35, 0.65]$) providing a deep, 3D atmospheric backdrop.
  - `MediumCrystal`: Classic six-fold dendritic crystals with sharp branching rays.
  - `LargeFluff`: Big, slow-drifting foreground snowflakes with soft radial glowing halos.
  - `Glitter`: Sparkly ice crystals with periodic sinusoidal luminance shimmer.
- **Individual Aerodynamics**: Each snowflake simulates its own mass, drag coefficient, sinusoidal fluttering/wobble, and angular rotation speed.
- **Zero-Allocation Particle Pool**: Reuses pre-allocated memory buffers for rock-solid frametimes without garbage collection spikes.

### 🏔️ 3. Physical Snow Accumulation on Windows
- **Silky-Smooth Quadratic Bezier Contours**: Snow mounds are rendered using midpoint-interpolated Quadratic Beziers (`AddQuadraticBezier`) for organic, pillowy curves instead of jagged polygons.
- **3D Volume & Frost Rim**: Soft cool-toned underside contact shading on thick mounds paired with a glistening frost highlight crest.
- **Windows 11 Rounded Corners Adaptation**: Automatically queries DWM window states (`IsZoomed`). For non-maximized windows, snow shelves seamlessly inset by 10 px with smooth boundary tapering, preventing snow from unnaturally hanging in mid-air over curved window corners.
- **Angle of Repose & Diffusion**: Snow naturally flows from sharp peaks down into crevices and tapers off at window edges.
- **Window Movement Synchronization**: Smoothly translates accumulated snow together with the window as you drag it across your desktop.

### 🌪️ 4. Window Aero Shake & Snowplow Tractor
- **Autonomous Snowplow Tractor**: Every 2–3 minutes (or immediately via the **🚜 Clear Snow** button), an animated snowplow tractor with a curved blade, spinning wheels, blinking amber beacon, and exhaust puffs drives along open windows and clears all accumulated snow with spraying snow waves.
- **Real-Time Window Velocity Tracking**: Monitors every window's position, velocity, and directional oscillation frequency.
- **Aero Shake Recognition**: Rapidly shaking a window side-to-side triggers physical snow detachment:
  - Snow fractures and tears away from the window surface proportionally to shake velocity.
  - Breaks apart into flying snow chunks, fine debris particles, and fluff flakes.
  - Dislodged particles inherit the window's momentum and fall down with real gravity and air resistance.
  - Leaves behind an organic, thin residual frosting on the window ledge.
- **Anti-False-Positive Filtering**: Gentle or normal window dragging keeps the snow safely in place.

### 🪟 5. Window Occlusion & Depth Ordering
- **Real-Time Z-Order Tracking**: Accurately tracks window stacking order. Windows in the foreground occlude background surfaces so snow only accumulates on visibly exposed window borders.
- **Snow Over Windows Toggle**: Choose between letting snowfall drift over applications or clipping flakes behind active application windows.
- **Smart Filtering**: Automatically ignores Windows shell components (Taskbar, Start Menu, virtual desktop surfaces, and invisible background processes).

### 💨 6. Dynamic Wind & Cursor Aerodynamics
- **Drifting Wind & Squalls**: Continuous shifting wind with smooth bell-curve gusts. Strong gusts gently blow fine snow powder wisps off tall snow peaks.
- **Mouse Vortices & Snow Brushing**: Moving the cursor through the snowfall generates aerodynamic air vortices (radial push + tangential swirl) that naturally deflect falling snowflakes, while fast sweeps across snow-covered window ledges brush snow off in real time.

### ⚡ 7. Decoupled UI & Extreme Optimization
- **Multithreaded UI Architecture**: The Control Center window (complete with 1-click **Light Snow / Classic / Blizzard** weather presets and custom snowflake titlebar icon) and System Tray run on their own isolated Win32 thread (`SnowfallUIThread`). Moving or dragging the settings dialog **never halts** the Direct2D rendering pipeline.
- **Zero-Allocation Hot Loop**: Reusable frame buffers and batched Direct2D transforms eliminate per-frame heap allocations for rock-solid 60 FPS frametimes.
- **Auto-Pause in Fullscreen**: Automatically enters deep pause when full-screen 3D games or videos are detected, dropping CPU and GPU usage to zero.
- **Power-Saving Modes**: Switchable between buttery-smooth 60 FPS and energy-efficient 30 FPS.
- **Tiny Footprint**: Only **~490 KB** executable size with zero runtime dependencies.

---

## 📸 Architecture Overview

```
                      ┌──────────────────────────────────────────────┐
                      │                 main.rs                      │
                      │  (Single-Instance, COM, DPI-v2 Awareness)   │
                      └──────────────────────┬───────────────────────┘
                                             │
                      ┌──────────────────────▼───────────────────────┐
                      │              App Coordinator                 │
                      │       (QPC Timer, State Synchronization)     │
                      └───────┬──────────────────────────────┬───────┘
                              │                              │
         ┌────────────────────▼──────────────┐   ┌───────────▼─────────────────────┐
         │        Render / Physics Loop      │   │      Dedicated UI Thread        │
         │      (Direct2D + Direct3D 11)     │   │      (Win32 Message Pump)       │
         ├───────────────────────────────────┤   ├─────────────────────────────────┤
         │ • ParticleSystem (Pool & Flakes)  │   │ • SettingsDialog (Native GUI)   │
         │ • AccumulationSystem (Heightfield)│   │ • System Tray Icon & Menu       │
         │ • WindowTracker (DWM & Z-Order)   │   │ • Atomic State Synchronization │
         │ • WeatherSystem (Wind & Gusts)    │   └─────────────────────────────────┘
         │ • CursorTracker (Aerodynamics)    │
         │ • PerformanceManager (FPS & Pause)│
         └───────────────────────────────────┘
```

---

## 🚀 Getting Started

### Prerequisites
- **OS**: Windows 11 or Windows 10 (version 1809+).
- **Rust Toolchain**: Rust 1.75+ with Cargo.

### Building from Source

1. Clone the repository:
   ```powershell
   git clone https://github.com/yto4ka002/snowfall.git
   cd snowfall
   ```

2. Compile optimized release binary:
   ```powershell
   cargo build --release
   ```

3. Run the application:
   ```powershell
   .\target\release\snowfall.exe
   ```

---

## 🎛️ Control Center & System Tray

Snowfall includes a native, lightweight Win32 control center accessible anytime:

- **Left-Click Tray Icon**: Opens the Control Center window.
- **Right-Click Tray Icon**: Quick context menu (Start/Pause, Clear Snow, Settings, Exit).

### Available Controls
| Option | Description |
|---|---|
| **Snowfall Toggle** | Instantly start or pause simulation (zero CPU/GPU when paused) |
| **Snowflakes Count** | Adjust snowflake density (50 to 1,500 particles) |
| **Fall Speed** | Adjust gravitational descent velocity (0.2x to 3.0x) |
| **Wind Intensity** | Adjust horizontal drift and squall force |
| **Snow Depth** | Maximum snow accumulation thickness on window borders (8 to 60 px) |
| **Shake Sensitivity** | Threshold for Aero Shake snow detachment (Low, Normal, High) |
| **Snow Over Windows** | Toggle whether snowflakes fall over or behind application windows |
| **Accumulation** | Enable or disable physical snow buildup on windows |
| **Cursor Interaction** | Enable or disable aerodynamic mouse displacement |
| **Pause in Fullscreen** | Automatically save 100% resources during games or full-screen video |
| **Frame Rate** | Choose between 60 FPS (smooth) or 30 FPS (battery-saver) |
| **Start with Windows** | Toggle autostart on Windows boot |

Settings are automatically saved to `%APPDATA%\Snowfall\settings.json`.

---

## 📦 Project Structure

```
snowfall/
├── Cargo.toml                # Project dependencies and release profile optimization
├── build.rs                  # Embeds Windows application manifest
├── snowfall.manifest         # Per-Monitor V2 DPI awareness & ComCtl32 v6 visual styles
├── README.md                 # English documentation
├── README_RU.md              # Russian documentation
└── src/
    ├── main.rs               # Entry point: Single-instance mutex, COM initialization
    ├── app.rs                # Main application loop and frame coordinator
    ├── config.rs             # Configuration manager, JSON I/O, registry autostart
    ├── performance.rs        # Frame limiter, QPC timing, fullscreen game detection
    ├── weather.rs            # Atmospheric wind simulation and squall gusts
    ├── cursor.rs             # Mouse velocity tracker and aerodynamic vortices
    ├── particle.rs           # Snowflake particle pool, flake types, and render data
    ├── window_tracker.rs     # Win32/DWM window tracking, Z-order clipping, Aero Shake
    ├── tractor.rs            # Animated snowplow tractor patrol and window clearing logic
    ├── accumulation.rs       # Snow shelf height-field, diffusion, and shedding physics
    ├── physics.rs            # Physics engine: collision detection and particle movement
    ├── renderer/             # Direct2D hardware-accelerated rendering engine
    │   ├── mod.rs            # Renderer façade and presentation pipeline
    │   ├── context.rs        # D3D11 device, DXGI composition swapchain, D2D device context
    │   ├── overlay_window.rs # Fullscreen transparent click-through Win32 overlay window
    │   ├── resources.rs      # Direct2D brushes and GPU snowflake texture synthesis
    │   └── draw.rs           # Snowflake rotation drawing and snow shelf mesh rendering
    └── ui/                   # User interface subsystem
        ├── mod.rs
        ├── tray.rs           # System tray icon and popup menu
        └── settings_dlg.rs   # Dedicated UI thread, native Win32 controls and event handling
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
Feel free to use, modify, and distribute.
