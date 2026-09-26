//! A precise, wakeable sleep for the playback thread: a high-resolution
//! waitable timer gets within a millisecond, then a short spin hits the
//! deadline. Windows 11 throttles timers of background processes, so the
//! process opts out of that while a timer exists (that is, while playing).

use std::ffi::c_void;
use std::sync::Arc;

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateEventW, CreateWaitableTimerExW, GetCurrentProcess, GetCurrentThread,
    INFINITE, PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
    PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION, PROCESS_POWER_THROTTLING_STATE, ProcessPowerThrottling,
    SetEvent, SetProcessInformation, SetThreadPriority, SetWaitableTimer, THREAD_PRIORITY_HIGHEST, TIMER_ALL_ACCESS,
    WaitForMultipleObjects, WaitForSingleObject,
};
use windows::core::PCWSTR;

use super::now_ms;
use crate::Timer;

/// Switch to the spin below this many milliseconds.
const SPIN_MS: f64 = 1.0;

struct Handle(HANDLE);
// Kernel handles may be used from any thread.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub struct WinTimer {
    timer: Handle,
    wake: Arc<Handle>,
}

/// Opts the process out of (or back into the system's default for) execution
/// speed and timer resolution throttling.
fn set_throttling(allowed: bool) {
    let state = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        // With both bits off in ControlMask, Windows decides again.
        ControlMask: if allowed {
            0
        } else {
            PROCESS_POWER_THROTTLING_EXECUTION_SPEED | PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION
        },
        StateMask: 0,
    };
    unsafe {
        let _ = SetProcessInformation(
            GetCurrentProcess(),
            ProcessPowerThrottling,
            &state as *const _ as *const c_void,
            size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        );
    }
}

impl Drop for WinTimer {
    fn drop(&mut self) {
        set_throttling(true);
    }
}

pub fn new() -> Box<dyn Timer> {
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
        set_throttling(false);
        let timer = CreateWaitableTimerExW(None, PCWSTR::null(), CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS.0)
            .or_else(|_| CreateWaitableTimerExW(None, PCWSTR::null(), 0, TIMER_ALL_ACCESS.0))
            .expect("CreateWaitableTimerExW");
        let wake = CreateEventW(None, false, false, PCWSTR::null()).expect("CreateEventW");
        Box::new(WinTimer { timer: Handle(timer), wake: Arc::new(Handle(wake)) })
    }
}

impl Timer for WinTimer {
    fn wait_until(&mut self, deadline: f64) -> bool {
        loop {
            let left = deadline - now_ms();
            if left <= 0.0 {
                return false;
            }
            unsafe {
                if left > SPIN_MS * 1.5 {
                    // Negative due time = relative, in 100 ns units.
                    let due = -((left - SPIN_MS) * 10_000.0) as i64;
                    if SetWaitableTimer(self.timer.0, &due, 0, None, None, false).is_err() {
                        std::thread::sleep(std::time::Duration::from_micros(((left - SPIN_MS) * 1000.0) as u64));
                        continue;
                    }
                    let r = WaitForMultipleObjects(&[self.timer.0, self.wake.0], false, INFINITE);
                    if r.0 == WAIT_OBJECT_0.0 + 1 {
                        return true;
                    }
                    if r != WAIT_OBJECT_0 {
                        // The wait failed: sleep instead of spinning on it.
                        std::thread::sleep(std::time::Duration::from_micros(((left - SPIN_MS) * 1000.0) as u64));
                    }
                } else {
                    if WaitForSingleObject(self.wake.0, 0) == WAIT_OBJECT_0 {
                        return true;
                    }
                    std::hint::spin_loop();
                }
            }
        }
    }

    fn waker(&self) -> Arc<dyn Fn() + Send + Sync> {
        let wake = self.wake.clone();
        Arc::new(move || unsafe {
            let _ = SetEvent(wake.0);
        })
    }
}
