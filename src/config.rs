use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use windows::core::w;
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_SET_VALUE,
    REG_SZ,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub enabled: bool,
    pub snowflake_count: usize,
    pub fall_speed: f32,
    pub wind_intensity: f32,
    pub snow_accumulation: bool,
    pub max_accumulation_height: f32,
    pub accumulation_rate: f32,
    pub shake_sensitivity: f32,
    pub cursor_interaction: bool,
    pub target_fps: u32,
    pub pause_on_fullscreen: bool,
    pub auto_start: bool,
    #[serde(default = "default_true")]
    pub snow_over_windows: bool,
    #[serde(default = "default_true")]
    pub auto_tractor: bool,
    #[serde(default = "default_one")]
    pub accumulation_opacity: f32,
}

fn default_true() -> bool {
    true
}

fn default_one() -> f32 {
    1.0
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            snowflake_count: 450,
            fall_speed: 1.0,
            wind_intensity: 1.0,
            snow_accumulation: true,
            max_accumulation_height: 24.0,
            accumulation_rate: 1.0,
            shake_sensitivity: 1.0,
            cursor_interaction: true,
            target_fps: 60,
            pause_on_fullscreen: true,
            auto_start: false,
            snow_over_windows: true,
            auto_tractor: true,
            accumulation_opacity: 1.0,
        }
    }
}

impl Settings {
    pub fn config_path() -> Option<PathBuf> {
        let appdata = std::env::var_os("APPDATA")?;
        let dir = PathBuf::from(appdata).join("Snowfall");
        Some(dir.join("settings.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<Settings>(&data) {
                    return settings;
                }
            }
        }
        let default_settings = Self::default();
        default_settings.save();
        default_settings
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = fs::write(&path, json);
            }
        }
        self.apply_autostart();
    }

    pub fn apply_autostart(&self) {
        unsafe {
            let mut key = Default::default();
            let subkey = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
            if RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey,
                Some(0),
                KEY_SET_VALUE,
                &mut key,
            )
            .is_ok()
            {
                let value_name = w!("Snowfall");
                if self.auto_start {
                    let mut exe_path = [0u16; MAX_PATH as usize];
                    let len = GetModuleFileNameW(None, &mut exe_path);
                    if len > 0 {
                        let bytes = std::slice::from_raw_parts(
                            exe_path.as_ptr() as *const u8,
                            ((len + 1) * 2) as usize,
                        );
                        let _ = RegSetValueExW(
                            key,
                            value_name,
                            Some(0),
                            REG_SZ,
                            Some(bytes),
                        );
                    }
                } else {
                    let _ = RegDeleteValueW(key, value_name);
                }
                let _ = RegCloseKey(key);
            }
        }
    }
}
