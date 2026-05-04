use arc_swap::ArcSwap;
use retour::static_detour;
use std::sync::{Arc, LazyLock};
use windows::Win32::Foundation::TRUE;
use windows::Win32::System::Performance::QueryPerformanceCounter;
use windows::Win32::System::SystemInformation;
use windows::Win32::System::SystemInformation::GetTickCount64;

pub static MANAGER: LazyLock<SpeedHackManager> = LazyLock::new(|| unsafe { SpeedHackManager::new().unwrap() });

pub struct SpeedHackManager {
    state: ArcSwap<TimeState>,
}

static_detour! {
    pub static _GET_TICK_COUNT: unsafe extern "system" fn() -> u32;
    pub static _GET_TICK_COUNT_64: unsafe extern "system" fn() -> u64;
    pub static _QUERY_PERFORMANCE_COUNTER: unsafe extern "system" fn(*mut i64) -> windows_sys::core::BOOL;
    pub static _TIME_GET_TIME: unsafe extern "system" fn() -> u32;
}

impl SpeedHackManager {
    pub unsafe fn new() -> eyre::Result<Self> {
        let gtc_base = SystemInformation::GetTickCount();
        let gtc_64_base = GetTickCount64();
        let tgt_base = windows_sys::Win32::Media::timeGetTime();

        let mut qpc_basetime = 0i64;
        QueryPerformanceCounter(&mut qpc_basetime)?;

        let initial_state = TimeState {
            speed: 1.0,
            gtc_basetime: gtc_base,
            gtc_offset_time: gtc_base,
            gtc_64_basetime: gtc_64_base,
            gtc_64_offset_time: gtc_64_base,
            tgt_basetime: tgt_base,
            tgt_offset_time: tgt_base,
            qpc_basetime,
            qpc_offset_time: qpc_basetime,
        };

        _GET_TICK_COUNT.initialize(
            windows_sys::Win32::System::SystemInformation::GetTickCount,
            real_get_tick_count,
        )?;

        _GET_TICK_COUNT_64.initialize(
            windows_sys::Win32::System::SystemInformation::GetTickCount64,
            real_get_tick_count_64,
        )?;

        _TIME_GET_TIME.initialize(windows_sys::Win32::Media::timeGetTime, real_time_get_time)?;

        _QUERY_PERFORMANCE_COUNTER.initialize(
            windows_sys::Win32::System::Performance::QueryPerformanceCounter,
            real_query_performance_counter,
        )?;

        _GET_TICK_COUNT.enable()?;
        _GET_TICK_COUNT_64.enable()?;
        _TIME_GET_TIME.enable()?;
        _QUERY_PERFORMANCE_COUNTER.enable()?;

        Ok(SpeedHackManager {
            state: ArcSwap::from_pointee(initial_state),
        })
    }

    /// Disable the static detours
    pub fn detach(&self) -> eyre::Result<()> {
        unsafe {
            _GET_TICK_COUNT.disable()?;
            _GET_TICK_COUNT_64.disable()?;
            _TIME_GET_TIME.disable()?;
            _QUERY_PERFORMANCE_COUNTER.disable()?;
        }
        Ok(())
    }

    pub fn set_speed(&self, speed: f64) {
        // Update the offsets to ensure we don't cause negative time warps.
        let old_state = self.state.load_full();

        let new_state = unsafe {
            let mut current_qpc_base = 0i64;
            let current_gtc = _GET_TICK_COUNT.call();
            let current_gtc_64 = _GET_TICK_COUNT_64.call();
            let current_tgt = _TIME_GET_TIME.call();
            let _ = _QUERY_PERFORMANCE_COUNTER.call(&mut current_qpc_base);

            TimeState {
                speed,
                gtc_basetime: current_gtc,
                gtc_offset_time: old_state.get_tick_count(current_gtc),
                gtc_64_basetime: current_gtc_64,
                gtc_64_offset_time: old_state.get_tick_count_64(current_gtc_64),
                tgt_basetime: current_tgt,
                tgt_offset_time: old_state.time_get_time(current_tgt),
                qpc_basetime: current_qpc_base,
                qpc_offset_time: old_state.query_performance_counter(current_qpc_base),
            }
        };

        self.state.store(Arc::new(new_state));
    }

    pub fn speed(&self) -> f64 {
        self.state.load().speed
    }

    pub fn get_time_get_time(&self) -> u32 {
        unsafe { self.state.load().time_get_time(_TIME_GET_TIME.call()) }
    }

    pub fn get_tick_count(&self) -> u32 {
        unsafe { self.state.load().get_tick_count(_GET_TICK_COUNT.call()) }
    }

    pub fn get_tick_count_64(&self) -> u64 {
        unsafe { self.state.load().get_tick_count_64(_GET_TICK_COUNT_64.call()) }
    }

    pub fn get_performance_counter(&self) -> i64 {
        let mut temp = 0i64;
        unsafe {
            _QUERY_PERFORMANCE_COUNTER.call(&mut temp);
            self.state.load().query_performance_counter(temp)
        }
    }
}

#[derive(Clone)]
struct TimeState {
    speed: f64,

    gtc_basetime: u32,
    gtc_offset_time: u32,

    gtc_64_basetime: u64,
    gtc_64_offset_time: u64,

    qpc_basetime: i64,
    qpc_offset_time: i64,

    tgt_basetime: u32,
    tgt_offset_time: u32,
}

impl Drop for SpeedHackManager {
    fn drop(&mut self) {
        if let Err(e) = self.detach() {
            log::error!("Failed to detach SpeedHackManager due to {:?}", e);
        }
    }
}

impl TimeState {
    #[inline]
    fn get_tick_count(&self, current_real: u32) -> u32 {
        self.gtc_offset_time + ((current_real - self.gtc_basetime) as f64 * self.speed) as u32
    }

    #[inline]
    fn get_tick_count_64(&self, current_real: u64) -> u64 {
        self.gtc_64_offset_time + ((current_real - self.gtc_64_basetime) as f64 * self.speed) as u64
    }

    #[inline]
    fn time_get_time(&self, current_real: u32) -> u32 {
        self.tgt_offset_time + ((current_real - self.tgt_basetime) as f64 * self.speed) as u32
    }

    #[inline]
    fn query_performance_counter(&self, current_real: i64) -> i64 {
        self.qpc_offset_time + ((current_real - self.qpc_basetime) as f64 * self.speed) as i64
    }
}

fn real_time_get_time() -> u32 {
    MANAGER.get_time_get_time()
}

fn real_get_tick_count() -> u32 {
    MANAGER.get_tick_count()
}

fn real_get_tick_count_64() -> u64 {
    MANAGER.get_tick_count_64()
}

fn real_query_performance_counter(lp_performance_counter: *mut i64) -> windows_sys::core::BOOL {
    unsafe {
        *lp_performance_counter = MANAGER.get_performance_counter();
    }

    TRUE.0
}
