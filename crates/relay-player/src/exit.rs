//! Exit codes, so a script or scheduled task can tell how a run ended.

use relay_core::session::FinishReason;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Completed = 0,
    /// The macro couldn't be read, or playback failed.
    Error = 1,
    BadArgs = 2,
    /// The Stop button, Esc, or any key with "Stop on key press".
    Stopped = 3,
    Killed = 4,
    /// A pixel check or Find image step timed out.
    TimedOut = 5,
    /// The screen was locked (or a UAC prompt was up): nothing could be played.
    Locked = 6,
}

impl Exit {
    pub fn of(reason: FinishReason) -> Exit {
        match reason {
            FinishReason::Completed => Exit::Completed,
            FinishReason::Stopped | FinishReason::KeyPressed => Exit::Stopped,
            FinishReason::Killed => Exit::Killed,
            FinishReason::PixelTimeout => Exit::TimedOut,
            FinishReason::Error => Exit::Error,
        }
    }

    pub fn code(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_ending_has_its_code() {
        let codes = [
            (FinishReason::Completed, 0),
            (FinishReason::Error, 1),
            (FinishReason::Stopped, 3),
            (FinishReason::KeyPressed, 3),
            (FinishReason::Killed, 4),
            (FinishReason::PixelTimeout, 5),
        ];
        for (reason, code) in codes {
            assert_eq!(Exit::of(reason).code(), code, "{reason:?}");
        }
        assert_eq!((Exit::BadArgs.code(), Exit::Locked.code()), (2, 6));
    }
}
