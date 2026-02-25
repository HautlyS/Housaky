use std::time::Duration;

/// Convert a `Duration` to milliseconds as `u64` without panicking.
///
/// Saturates to `u64::MAX` if the duration is too large to fit.
pub fn duration_ms_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Convert an `Instant::elapsed()`-style millis u128 into u64 safely.
pub fn millis_u128_to_u64(ms: u128) -> u64 {
    u64::try_from(ms).unwrap_or(u64::MAX)
}
