//! Persistent settings: API key, model, rewrite modes. Lives in
//! `~/.config/refineai/config.json`, written with `0600` perms so the key
//! isn't world-readable.

use crate::store;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Mode {
    pub name: String,
    pub prompt: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub api_key: String,
    pub model: String,
    /// Sends `thinkingLevel: "minimal"` on Gemini 3 models — faster first token.
    pub low_latency: bool,
    /// Skip the preview: refine and copy straight to the clipboard.
    pub direct_replace: bool,
    pub save_history: bool,
    /// Show a live activity log inside the popup, for diagnosing failures.
    pub debug: bool,
    pub modes: Vec<Mode>,
    /// Index into `modes` of the mode a fresh popup opens in.
    pub default_mode: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_key: String::new(),
            model: "gemini-3.5-flash-lite".into(),
            low_latency: true,
            direct_replace: false,
            save_history: true,
            debug: false,
            modes: default_modes(),
            default_mode: 0,
        }
    }
}

fn default_modes() -> Vec<Mode> {
    let m = |name: &str, prompt: &str| Mode {
        name: name.into(),
        prompt: prompt.into(),
    };
    vec![
        m(
            "Coherent",
            "Rewrite the text so it is clear and coherent while keeping the original meaning, tone and language.",
        ),
        m(
            "Formal",
            "Rewrite the text in a polished, professional and formal register, keeping the original meaning and language.",
        ),
        m(
            "Concise",
            "Rewrite the text to be as concise as possible without losing meaning, keeping the original language.",
        ),
        m(
            "Fix grammar",
            "Correct only spelling, grammar and punctuation. Do not change wording, tone or meaning more than necessary.",
        ),
    ]
}

impl Config {
    pub fn path() -> PathBuf {
        store::app_path(dirs::config_dir(), "config.json")
    }

    pub fn load() -> Config {
        let mut cfg: Config = store::read(&Self::path());
        if cfg.modes.is_empty() {
            cfg.modes = default_modes();
        }
        if cfg.default_mode >= cfg.modes.len() {
            cfg.default_mode = 0;
        }
        cfg
    }

    pub fn save(&self) -> std::io::Result<()> {
        // `private`: the config holds the API key, so lock it to the owner.
        store::write(&Self::path(), self, true)
    }
}
