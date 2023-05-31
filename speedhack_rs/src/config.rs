use anyhow::Context;
use std::path::Path;
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_R, VK_SHIFT};

pub const CONFIG_FILE_NAME: &str = "speedhack_config.json";

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct SpeedhackConfig {
    /// Whether to open a console for logging
    pub console: bool,
    /// How long to wait before trying to hook the relevant game functions. Can prevent crashes due to early loads.
    pub wait_with_hook: Option<Duration>,
    /// If set, will allow the config to be reloaded during gameplay by providing the given key codes.
    pub reload_config_keys: Option<Vec<u16>>,
    /// Different speed states
    pub speed_states: Vec<SpeedStateConfig>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct SpeedStateConfig {
    /// All keys that need to be pressed for a speed state to be selected.
    ///
    /// Expects [virtual key codes](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes).
    pub keys: Vec<u16>,
    /// The speed to run at while the selected keys are selected.
    ///
    /// Needs to be `> 0`
    pub speed: f64,
    /// Whether the keys need to be held to have the speed change take effect.
    ///
    /// If `false` then the keys act as a toggle.
    pub is_toggle: bool,
}

impl Default for SpeedhackConfig {
    fn default() -> Self {
        Self {
            console: false,
            wait_with_hook: Some(Duration::from_millis(250)),
            reload_config_keys: Some(vec![VK_CONTROL.0, VK_SHIFT.0, VK_R.0]),
            speed_states: vec![SpeedStateConfig {
                keys: vec![VK_SHIFT.0, VK_CONTROL.0],
                speed: 10.0,
                is_toggle: false,
            }],
        }
    }
}

pub fn load_config(directory: impl AsRef<Path>) -> anyhow::Result<SpeedhackConfig> {
    let file = std::fs::read(directory.as_ref().join(CONFIG_FILE_NAME))?;
    let conf = serde_json::from_slice(&file).context("Failed to read config file, is it valid?")?;

    validate_config(&conf)?;

    Ok(conf)
}

pub fn create_initial_config(directory: impl AsRef<Path>) -> anyhow::Result<()> {
    let default_conf = SpeedhackConfig::default();
    let path = directory.as_ref().join(CONFIG_FILE_NAME);

    if !path.exists() {
        let mut file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(&mut file, &default_conf)?;
    }

    Ok(())
}

fn validate_config(config: &SpeedhackConfig) -> anyhow::Result<()> {
    let mut errors = Vec::new();

    for state in &config.speed_states {
        if state.keys.iter().any(|key| *key > 256) {
            errors.push(format!(
                "Key with index of greater than 256 is not allowed, are you sure it's valid?\nState: `{:#?}`",
                state
            ))
        }
    }

    let error = errors.join("\n");

    if error.is_empty() {
        Ok(())
    } else {
        Err(anyhow::Error::msg(error))
    }
}
