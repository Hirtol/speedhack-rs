use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result};
use log::LevelFilter;
use rust_hooking_utils::patching::process::GameProcess;
use rust_hooking_utils::raw_input::virtual_keys::VirtualKey;
use windows::core::HSTRING;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxExW, MB_OK};

use crate::config::SpeedhackConfig;
use crate::speedhack::MANAGER;

mod config;
mod speedhack;

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);
/// Some games seem to double call DllMain somehow and need an additional guard here to prevent 2 threads running at the same time.
static LOAD_GUARD: Mutex<()> = Mutex::new(());

pub fn dll_attach(hinst_dll: windows::Win32::Foundation::HMODULE) -> Result<()> {
    let dll_path = rust_hooking_utils::get_current_dll_path(hinst_dll)?;
    let config_directory = dll_path.parent().context("DLL is in root")?;
    let cfg = simplelog::ConfigBuilder::new().build();

    // Ignore result in case we have double initialisation of the DLL.
    let _ = simplelog::SimpleLogger::init(LevelFilter::Trace, cfg);

    config::create_initial_config(config_directory)?;

    let mut conf = load_validated_config(config_directory, None)?;

    if conf.console {
        unsafe {
            windows::Win32::System::Console::AllocConsole()?;
        }
    }

    log::info!("Loaded config: {:#?}", conf);

    if let Some(wait) = conf.wait_with_hook {
        std::thread::sleep(wait);
    }

    let Ok(_lock) = LOAD_GUARD.try_lock() else {
        log::trace!("Failed to acquire lock, not the only thread running, stopping");
        return Ok(());
    };

    let speed_manager = &*speedhack::MANAGER;
    let mut key_manager = rust_hooking_utils::raw_input::key_manager::KeyboardManager::new();

    startup_routine(&conf)?;

    let main_window = loop {
        if let Some(wnd) = GameProcess::current_process().get_main_window() {
            break wnd;
        } else {
            std::thread::sleep(Duration::from_millis(100))
        }
    };

    log::info!("Found main window: {:?} ({:?})", main_window.title(), main_window.0);

    while !SHUTDOWN_FLAG.load(Ordering::Acquire) {
        {
            if let Some(reload) = &conf.reload_config_keys {
                if key_manager.all_pressed(reload.iter().copied().map(VirtualKey::to_virtual_key)) {
                    conf = reload_config(config_directory, &conf, main_window.0)?;
                }
            }

            if main_window.is_foreground_window() {
                let mut manager = speed_manager.write().unwrap();

                for state in &conf.speed_states {
                    let mapped = state
                        .keys
                        .iter()
                        .copied()
                        .map(VirtualKey::to_virtual_key)
                        .collect::<Vec<_>>();

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

fn reload_config(
    config_dir: impl AsRef<Path>,
    old: &SpeedhackConfig,
    parent_window: HWND,
) -> anyhow::Result<SpeedhackConfig> {
    log::debug!("Reloading config");
    let conf = load_validated_config(config_dir, Some(parent_window))?;

    // Open/close console
    if old.console && !conf.console {
        unsafe {
            windows::Win32::System::Console::FreeConsole()?;
        }
    } else if !old.console && conf.console {
        unsafe {
            windows::Win32::System::Console::AllocConsole()?;
        }
    }

    log::debug!("New config loaded: {:#?}", conf);

    Ok(conf)
}

fn load_validated_config(config_dir: impl AsRef<Path>, parent_window: Option<HWND>) -> anyhow::Result<SpeedhackConfig> {
    match config::load_config(config_dir) {
        Ok(conf) => Ok(conf),
        Err(e) => unsafe {
            let message = format!("Error: {}\nSpeedhack will now exit", e);
            let _ = MessageBoxExW(
                parent_window.unwrap_or_default(),
                &HSTRING::from(message),
                windows::core::w!("Failed to validate Speedhack config"),
                MB_OK,
                0,
            );
            Err(e)
        },
    }
}

fn startup_routine(config: &SpeedhackConfig) -> anyhow::Result<()> {
    if let Some(startup) = config.startup_state.clone() {
        std::thread::spawn(move || {
            let manager = &*MANAGER;
            log::info!(
                "Startup detected, set speed to `{}` for `{:?}`",
                startup.speed,
                startup.duration
            );
            manager.write().unwrap().set_speed(startup.speed);

            std::thread::sleep(startup.duration);
            let mut lock = manager.write().unwrap();
            // If the user hasn't touched the manager since that time we'll reset it.
            if lock.speed() == startup.speed {
                lock.set_speed(1.0);
                log::info!("Startup sequence ended, reset speed to `1.0`");
            }
        });
    }

    Ok(())
}
