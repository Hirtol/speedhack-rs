use once_cell::sync::Lazy;
use retour::static_detour;
use std::sync::RwLock;
use windows::Win32::System::Performance::QueryPerformanceCounter;
use windows::Win32::System::SystemInformation;
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::Foundation::{BOOL, TRUE};

pub static MANAGER: Lazy<RwLock<SpeedHackManager>> = Lazy::new(|| unsafe { SpeedHackManager::new().unwrap().into() });

pub struct SpeedHackManager {
    speed: f64,

    gtc_basetime: u32,
    gtc_offset_time: u32,
    gtc_64_basetime: u64,
    gtc_64_offset_time: u64,

    qpc_basetime: i64,
    qpc_offset_time: i64,
}

static_detour! {
    pub static _GET_TICK_COUNT: unsafe extern "system" fn() -> u32;
    pub static _GET_TICK_COUNT_64: unsafe extern "system" fn() -> u64;
    pub static _QUERY_PERFORMANCE_COUNTER: unsafe extern "system" fn(*mut i64) -> BOOL;
}

impl SpeedHackManager {
    pub unsafe fn new() -> anyhow::Result<Self> {
        let gtc_base = SystemInformation::GetTickCount();
        let gtc_64_base = GetTickCount64();

        let mut qpc_basetime = 0i64;
        QueryPerformanceCounter(&mut qpc_basetime)?;

        _GET_TICK_COUNT.initialize(
            windows_sys::Win32::System::SystemInformation::GetTickCount,
            real_get_tick_count,
        )?;

        _GET_TICK_COUNT_64.initialize(
            windows_sys::Win32::System::SystemInformation::GetTickCount64,
            real_get_tick_count_64,
        )?;

        _QUERY_PERFORMANCE_COUNTER.initialize(
            windows_sys::Win32::System::Performance::QueryPerformanceCounter,
            real_query_performance_counter,
        )?;

        _GET_TICK_COUNT.enable()?;
        _GET_TICK_COUNT_64.enable()?;
        _QUERY_PERFORMANCE_COUNTER.enable()?;

        Ok(SpeedHackManager {
            speed: 1.0,
            gtc_basetime: gtc_base,
            gtc_offset_time: gtc_base,
            gtc_64_basetime: gtc_64_base,
            gtc_64_offset_time: gtc_64_base,
            qpc_basetime,
            qpc_offset_time: qpc_basetime,
        })
    }

    /// Disable the static detours
    pub fn detach(&mut self) -> anyhow::Result<()> {
        unsafe {
            _GET_TICK_COUNT.disable()?;
            _GET_TICK_COUNT_64.disable()?;
            _QUERY_PERFORMANCE_COUNTER.disable()?;
        }

        Ok(())
    }

    pub fn set_speed(&mut self, speed: f64) {
        // Update the offsets to ensure we don't cause negative time warps.
        unsafe {
            self.gtc_offset_time = self.get_tick_count();
            self.gtc_basetime = _GET_TICK_COUNT.call();

            self.gtc_64_offset_time = self.get_tick_count_64();
            self.gtc_64_basetime = _GET_TICK_COUNT_64.call();

            self.qpc_offset_time = self.get_performance_counter();
            let _ = _QUERY_PERFORMANCE_COUNTER.call(&mut self.qpc_basetime);
        }

        self.speed = speed;
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    pub fn get_tick_count(&self) -> u32 {
        unsafe { self.gtc_offset_time + ((_GET_TICK_COUNT.call() - self.gtc_basetime) as f64 * self.speed) as u32 }
    }

    pub fn get_tick_count_64(&self) -> u64 {
        unsafe {
            self.gtc_64_offset_time + ((_GET_TICK_COUNT_64.call() - self.gtc_64_basetime) as f64 * self.speed) as u64
        }
    }

    pub fn get_performance_counter(&self) -> i64 {
        let mut temp = 0i64;

        unsafe {
            _QUERY_PERFORMANCE_COUNTER.call(&mut temp);
            self.qpc_offset_time + ((temp - self.qpc_basetime) as f64 * self.speed) as i64
        }
    }
}

fn real_get_tick_count() -> u32 {
    MANAGER.read().unwrap().get_tick_count()
}

fn real_get_tick_count_64() -> u64 {
    MANAGER.read().unwrap().get_tick_count_64()
}

fn real_query_performance_counter(lp_performance_counter: *mut i64) -> BOOL {
    unsafe {
        *lp_performance_counter = MANAGER.read().unwrap().get_performance_counter();
    }

    TRUE
}

impl Drop for SpeedHackManager {
    fn drop(&mut self) {
        if let Err(e) = self.detach() {
            log::error!("Failed to detach SpeedHackManager due to {:?}", e);
        }
    }
}
