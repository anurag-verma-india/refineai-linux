//! A tiny thread-safe activity log. The background refine thread pushes lines
//! into it; the UI reads them each frame and renders them when debug mode is on.
//! Lines are timestamped with seconds elapsed since the log was created, so you
//! can see exactly where a slow or failing request spends its time.

use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub struct DebugLog {
    start: Instant,
    lines: Arc<Mutex<Vec<String>>>,
}

impl Default for DebugLog {
    fn default() -> Self {
        DebugLog {
            start: Instant::now(),
            lines: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl DebugLog {
    /// Append a line, stamped with elapsed seconds since the last `clear`.
    pub fn log(&self, msg: impl Into<String>) {
        let t = self.start.elapsed().as_secs_f32();
        if let Ok(mut v) = self.lines.lock() {
            v.push(format!("[{t:6.3}s] {}", msg.into()));
        }
    }

    /// Reset the timer and drop all lines (called at the start of each run).
    pub fn clear(&mut self) {
        self.start = Instant::now();
        if let Ok(mut v) = self.lines.lock() {
            v.clear();
        }
    }

    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().map(|v| v.clone()).unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.lock().map(|v| v.is_empty()).unwrap_or(true)
    }
}
