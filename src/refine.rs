//! The shared core: take some text and a mode, call Gemini, log the result.
//! One blocking function; the UI runs it on a background thread.

use crate::config::Config;
use crate::debuglog::DebugLog;
use crate::gemini;
use crate::history;
use std::time::Duration;

pub fn refine(cfg: &Config, text: &str, mode_index: usize, log: &DebugLog) -> Result<String, String> {
    if cfg.api_key.trim().is_empty() {
        log.log("no API key set — aborting");
        return Err("No API key set. Open Settings and paste your Gemini key.".into());
    }
    let mode = cfg.modes.get(mode_index).ok_or_else(|| {
        log.log(format!("mode index {mode_index} out of range"));
        "Selected mode no longer exists.".to_string()
    })?;

    log.log(format!("mode: {}", mode.name));
    log.log(format!(
        "model: {} · low_latency: {} · input: {} chars",
        cfg.model,
        cfg.low_latency,
        text.chars().count()
    ));

    let url = gemini::endpoint(&cfg.model, &cfg.api_key);
    let body = gemini::build_body(text, &mode.prompt, &cfg.model, cfg.low_latency);

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| {
            log.log(format!("client build failed: {e}"));
            format!("Client error: {e}")
        })?;

    // Log the endpoint without the `?key=…` so the API key never lands in the UI.
    log.log(format!(
        "POST .../models/{}:generateContent (connect 10s, timeout 60s)",
        cfg.model
    ));
    let resp = client.post(&url).json(&body).send().map_err(|e| {
        if e.is_timeout() {
            log.log("request timed out");
            "Request timed out.".to_string()
        } else {
            log.log(format!("network error: {e}"));
            format!("Network error: {e}")
        }
    })?;

    let status = resp.status();
    log.log(format!("HTTP {status}"));

    let raw = resp.text().map_err(|e| {
        log.log(format!("failed reading body: {e}"));
        format!("Read error: {e}")
    })?;
    log.log(format!("received {} bytes", raw.len()));

    let result = gemini::parse(&raw).map_err(|e| {
        log.log(format!("parse/API error: {e}"));
        e
    })?;
    log.log(format!("parsed {} chars", result.chars().count()));

    if cfg.save_history {
        history::record(text, &result, &mode.name, &cfg.model);
        log.log("saved to history");
    }
    log.log("done");
    Ok(result)
}
