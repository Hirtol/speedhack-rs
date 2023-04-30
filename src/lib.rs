use crate::keyboard::{KeyState, KeyboardManager};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyboardState, VIRTUAL_KEY, VK_SHIFT,
};

mod config;
mod keyboard;
mod speedhack;

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

fn dll_attach() -> Result<()> {
    config::create_initial_config()?;

    let config = config::load_config()?;

    log::info!("Loaded config: {:?}", config);

    if config.console {
        // Create a console if one doesn't exist
        unsafe {
            windows::Win32::System::Console::AllocConsole();
        }
    }

    println!("Starting SpeedHackManager");
    unsafe {
        let man = &*speedhack::MANAGER;

        let mut key_manager = KeyboardManager::new();

        loop {
            while !SHUTDOWN_FLAG.load(Ordering::Acquire) {
                {
                    let mut man = man.write().unwrap();

                    for state in &config.speed_states {
                        let mapped = state
                            .keys
                            .iter()
                            .copied()
                            .map(|key| key_manager.get_key_state(VIRTUAL_KEY(key)))
                            .collect::<Vec<_>>();

                        if mapped.iter().all(|key| *key == KeyState::Pressed) {
                            if man.speed() == state.speed && state.is_toggle {
                                log::trace!("Toggle off, reset speed to 1.0");
                                man.set_speed(1.0);
                            } else {
                                log::trace!("Set speed to: {}", state.speed);
                                man.set_speed(state.speed);
                            }
                        } else if mapped.iter().any(|key| *key == KeyState::Released)
                            && !state.is_toggle
                        {
                            log::trace!("Keys released, reset speed to 1.0");
                            man.set_speed(1.0);
                        }
                    }
                }

                std::thread::sleep(Duration::from_millis(16));
            }
        }
    }

    Ok(())
}

/// This is ran on the detachment of the Rust DLL. It returns a Result to indicate
/// whether were errors.
fn dll_detach() -> Result<()> {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
    println!("Detached!");

    // everything worked out fine
    Ok(())
}

rust_hooking_utils::dll_main!(dll_attach, dll_detach);
