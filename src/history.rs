//! On-device history, stored as a JSON log in
//! `~/.local/share/refineai/history.json`. Pure Rust, no SQLite — keeps the
//! binary dependency-free and the file easy to inspect.
//!
//! Deletion is soft (`deleted_at` is set, the row stays) so a recycle bin and
//! restore stay possible. `purge_expired` drops entries 30 days after deletion
//! and runs once at launch.

use serde::{Deserialize, Serialize};
use std::fs;
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

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("refineai")
        .join("history.json")
}

fn load() -> Vec<Entry> {
    match fs::read_to_string(path()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save(entries: &[Entry]) -> std::io::Result<()> {
    let p = path();
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&p, serde_json::to_string_pretty(entries).unwrap_or_default())
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
        created_at: now(),
        deleted_at: None,
    });
    let _ = save(&entries);
}

/// Active (not soft-deleted) entries, newest first.
pub fn list_active() -> Vec<Entry> {
    let mut v: Vec<Entry> = load().into_iter().filter(|e| e.deleted_at.is_none()).collect();
    v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    v
}

/// Soft-deleted entries (the recycle bin), most-recently-deleted first.
pub fn list_deleted() -> Vec<Entry> {
    let mut v: Vec<Entry> = load().into_iter().filter(|e| e.deleted_at.is_some()).collect();
    v.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));
    v
}

/// Move one entry to the recycle bin (soft delete — stays restorable for 30 days).
pub fn soft_delete(id: &str) {
    let mut entries = load();
    for e in entries.iter_mut() {
        if e.id == id && e.deleted_at.is_none() {
            e.deleted_at = Some(now());
        }
    }
    let _ = save(&entries);
}

/// Bring one entry back out of the recycle bin.
pub fn restore(id: &str) {
    let mut entries = load();
    for e in entries.iter_mut() {
        if e.id == id {
            e.deleted_at = None;
        }
    }
    let _ = save(&entries);
}

/// Permanently remove one entry, bypassing the 30-day window.
pub fn purge(id: &str) {
    let mut entries = load();
    entries.retain(|e| e.id != id);
    let _ = save(&entries);
}

/// Drop entries whose 30-day window since deletion has elapsed.
pub fn purge_expired() {
    let cutoff = now().saturating_sub(RETENTION_SECS);
    let entries = load();
    let kept: Vec<Entry> = entries
        .into_iter()
        .filter(|e| e.deleted_at.map(|d| d > cutoff).unwrap_or(true))
        .collect();
    let _ = save(&kept);
}
