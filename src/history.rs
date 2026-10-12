//! On-device history, stored as a JSON log in
//! `~/.local/share/refineai/history.json`. Pure Rust, no SQLite — keeps the
//! binary dependency-free and the file easy to inspect.
//!
//! Deletion is soft (`deleted_at` is set, the row stays) so a recycle bin and
//! restore stay possible. `purge_expired` drops entries 30 days after deletion
//! and runs once at launch.

use crate::store;
use crate::util::unix_secs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const RETENTION_SECS: u64 = 30 * 24 * 60 * 60;

#[derive(Serialize, Deserialize, Clone)]
pub struct Entry {
    pub id: String,
    pub original: String,
    pub result: String,
    pub mode: String,
    pub model: String,
    pub created_at: u64,
    #[serde(default)]
    pub deleted_at: Option<u64>,
}

fn path() -> PathBuf {
    store::app_path(dirs::data_dir(), "history.json")
}

fn load() -> Vec<Entry> {
    store::read(&path())
}

fn save(entries: &[Entry]) {
    // History is non-critical; a failed write just drops the update rather than
    // interrupting the refine the user is waiting on.
    let _ = store::write(&path(), entries, false);
}

/// A rough client-side UUIDv4-ish id (no extra crate). Good enough to key an
/// append-only log and keep future server sync possible.
fn gen_id() -> String {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:032x}", t ^ (t.rotate_left(17)))
}

/// Append one rewrite. Written when the request returns (like Android), so it's
/// a complete log of what was actually sent — retries included.
pub fn record(original: &str, result: &str, mode: &str, model: &str) {
    let mut entries = load();
    entries.push(Entry {
        id: gen_id(),
        original: original.to_string(),
        result: result.to_string(),
        mode: mode.to_string(),
        model: model.to_string(),
        created_at: unix_secs(),
        deleted_at: None,
    });
    save(&entries);
}

/// Active (not soft-deleted) entries, newest first.
pub fn list_active() -> Vec<Entry> {
    let mut v: Vec<Entry> = load().into_iter().filter(|e| e.deleted_at.is_none()).collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.created_at));
    v
}

/// Soft-deleted entries (the recycle bin), most-recently-deleted first.
pub fn list_deleted() -> Vec<Entry> {
    let mut v: Vec<Entry> = load().into_iter().filter(|e| e.deleted_at.is_some()).collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.deleted_at));
    v
}

/// Move one entry to the recycle bin (soft delete — stays restorable for 30 days).
pub fn soft_delete(id: &str) {
    let mut entries = load();
    for e in entries.iter_mut() {
        if e.id == id && e.deleted_at.is_none() {
            e.deleted_at = Some(unix_secs());
        }
    }
    save(&entries);
}

/// Bring one entry back out of the recycle bin.
pub fn restore(id: &str) {
    let mut entries = load();
    for e in entries.iter_mut() {
        if e.id == id {
            e.deleted_at = None;
        }
    }
    save(&entries);
}

/// Permanently remove one entry, bypassing the 30-day window.
pub fn purge(id: &str) {
    let mut entries = load();
    entries.retain(|e| e.id != id);
    save(&entries);
}

/// Drop entries whose 30-day window since deletion has elapsed.
pub fn purge_expired() {
    let cutoff = unix_secs().saturating_sub(RETENTION_SECS);
    let entries = load();
    let kept: Vec<Entry> = entries
        .into_iter()
        .filter(|e| e.deleted_at.map(|d| d > cutoff).unwrap_or(true))
        .collect();
    save(&kept);
}
