use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::config::SpeedhackConfig;
use anyhow::Result;
use log::LevelFilter;
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

use crate::keyboard::KeyboardManager;

mod config;
mod keyboard;
mod speedhack;

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

rust_hooking_utils::dll_main!(dll_attach, dll_detach);

fn dll_attach() -> Result<()> {
    let cfg = simplelog::ConfigBuilder::new().build();

    // Ignore result in case we have double initialisation of the DLL.
    let _ = simplelog::SimpleLogger::init(LevelFilter::Trace, cfg);

    config::create_initial_config()?;

    let mut conf = config::load_config()?;

    if conf.console {
        unsafe {
            windows::Win32::System::Console::AllocConsole();
        }
    }

    log::info!("Loaded config: {:#?}", conf);

    let speed_manager = &*speedhack::MANAGER;
    let mut key_manager = KeyboardManager::new();

    while !SHUTDOWN_FLAG.load(Ordering::Acquire) {
        {
            if let Some(reload) = &conf.reload_config_keys {
                if key_manager.all_pressed(reload.iter().copied().map(VIRTUAL_KEY)) {
                    conf = reload_config(&conf)?;
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

fn dll_detach() -> Result<()> {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
    log::info!("Detached!");

    Ok(())
}

fn reload_config(old: &SpeedhackConfig) -> anyhow::Result<SpeedhackConfig> {
    log::debug!("Reloading config");
    let conf = config::load_config()?;

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
