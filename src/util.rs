//! Small shared helpers with no more specific home.

use std::time::{SystemTime, UNIX_EPOCH};

/// Current Unix time in whole seconds (0 if the clock is somehow before the
/// epoch). Used for history timestamps and the "x ago" labels in the UI.
pub fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
