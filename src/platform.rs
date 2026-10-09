//! The only OS-specific code. Reading the selection, writing the clipboard and
//! best-effort auto-paste are all done by shelling out to the small CLI tools
//! that already exist per display server — so the binary itself has no X11 or
//! Wayland link-time dependency and stays portable across distros.
//!
//! - Wayland: `wl-paste` / `wl-copy` (wl-clipboard), `ydotool` for auto-paste.
//! - X11:     `xclip`, `xdotool` for auto-paste.

use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Clone, Copy, PartialEq)]
pub enum Server {
    Wayland,
    X11,
}

pub fn detect() -> Server {
    let wayland = std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE").map(|s| s == "wayland").unwrap_or(false);
    if wayland {
        Server::Wayland
    } else {
        Server::X11
    }
}

fn run_capture(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        None
    }
}

fn run_stdin(cmd: &str, args: &[&str], input: &str) -> std::io::Result<()> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(input.as_bytes())?;
    }
    child.wait()?;
    Ok(())
}

fn exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success() || true) // presence is what we're testing, not exit code
        .unwrap_or(false)
}

/// Read the highlighted text: the PRIMARY selection first (no copy needed),
/// falling back to the clipboard.
pub fn read_selection() -> Option<String> {
    let text = match detect() {
        Server::Wayland => run_capture("wl-paste", &["--primary", "--no-newline"])
            .or_else(|| run_capture("wl-paste", &["--no-newline"])),
        Server::X11 => run_capture("xclip", &["-o", "-selection", "primary"])
            .or_else(|| run_capture("xclip", &["-o", "-selection", "clipboard"])),
    };
    text.map(|t| t.trim_end_matches('\n').to_string())
        .filter(|t| !t.trim().is_empty())
}

/// Put text on the system clipboard (CLIPBOARD selection).
pub fn set_clipboard(text: &str) -> std::io::Result<()> {
    match detect() {
        Server::Wayland => run_stdin("wl-copy", &[], text),
        Server::X11 => run_stdin("xclip", &["-selection", "clipboard"], text),
    }
}

/// Best-effort: simulate Ctrl+V so the refined text lands where the cursor is.
/// Returns false when no input tool is installed (Wayland `ydotool` needs a
/// root daemon), in which case the caller falls back to "copied — press Ctrl+V".
pub fn try_paste() -> bool {
    match detect() {
        Server::Wayland => {
            if exists("ydotool") {
                // keycodes: 29 = LEFTCTRL, 47 = V
                Command::new("ydotool")
                    .args(["key", "29:1", "47:1", "47:0", "29:0"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            } else {
                false
            }
        }
        Server::X11 => {
            if exists("xdotool") {
                Command::new("xdotool")
                    .args(["key", "--clearmodifiers", "ctrl+v"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            } else {
                false
            }
        }
    }
}

pub fn server_name() -> &'static str {
    match detect() {
        Server::Wayland => "Wayland",
        Server::X11 => "X11",
    }
}
