use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result};
use log::LevelFilter;
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

use crate::config::SpeedhackConfig;
use crate::keyboard::KeyboardManager;

mod config;
mod keyboard;
mod speedhack;

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);
/// Some games seem to double call DllMain somehow and need an additional guard here to prevent 2 threads running at the same time.
static LOAD_GUARD: Mutex<()> = Mutex::new(());

pub fn dll_attach(hinst_dll: windows::Win32::Foundation::HMODULE) -> Result<()> {
    let dll_path = rust_hooking_utils::get_current_dll_path(hinst_dll)?;
    let config_directory = dll_path.parent().context("DLL is in root")?;
    let cfg = simplelog::ConfigBuilder::new().build();

    // Ignore result in case we have double initialisation of the DLL.
    let _ = simplelog::SimpleLogger::init(LevelFilter::Trace, cfg)?;

    config::create_initial_config(config_directory)?;

    let mut conf = config::load_config(config_directory)?;

    if conf.console {
        unsafe {
            windows::Win32::System::Console::AllocConsole();
        }
    }

    log::info!("Loaded config: {:#?}", conf);

    if let Some(wait) = conf.wait_with_hook {
        std::thread::sleep(wait);
    }

    let Ok(_lock) = LOAD_GUARD.try_lock() else {
        log::trace!("Failed to acquire lock, not the only thread running, stopping");
        return Ok(())
    };

    let speed_manager = &*speedhack::MANAGER;
    let mut key_manager = KeyboardManager::new();

    while !SHUTDOWN_FLAG.load(Ordering::Acquire) {
        {
            if let Some(reload) = &conf.reload_config_keys {
                if key_manager.all_pressed(reload.iter().copied().map(VIRTUAL_KEY)) {
                    conf = reload_config(config_directory, &conf)?;
                }
            }

            let mut manager = speed_manager.write().unwrap();

            for state in &conf.speed_states {
                let mapped = state.keys.iter().copied().map(VIRTUAL_KEY).collect::<Vec<_>>();

                if key_manager.all_pressed(mapped.iter().copied()) {
                    if manager.speed() == state.speed && state.is_toggle {
                        log::trace!("Toggle off, reset speed to 1.0");
                        manager.set_speed(1.0);
                    } else {
                        if state.is_toggle {
                            log::trace!("Toggle, set speed to: {}", state.speed)
                        } else {
                            log::trace!("Set speed to: {}", state.speed);
                        }
                        manager.set_speed(state.speed);
                    }
                } else if key_manager.any_released(mapped.into_iter()) && !state.is_toggle {
                    log::trace!("Keys released, reset speed to 1.0");
                    manager.set_speed(1.0);
                }
            }
        }

        std::thread::sleep(Duration::from_millis(16));
        key_manager.end_frame();
    }

    Ok(())
}

pub fn dll_detach(_hinst_dll: windows::Win32::Foundation::HMODULE) -> Result<()> {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
    log::info!("Detached! {:?}", std::thread::current().id());

    Ok(())
}

fn reload_config(config_dir: impl AsRef<Path>, old: &SpeedhackConfig) -> anyhow::Result<SpeedhackConfig> {
    log::debug!("Reloading config");
    let conf = config::load_config(config_dir)?;

    // Open/close console
    if old.console && !conf.console {
        unsafe {
            windows::Win32::System::Console::FreeConsole();
        }
    } else if !old.console && conf.console {
        unsafe {
            windows::Win32::System::Console::AllocConsole();
        }
    }

    log::debug!("New config loaded: {:#?}", conf);

    Ok(conf)
}
