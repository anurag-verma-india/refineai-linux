# RefineAI for Linux

Rewrite selected text with Gemini from a small floating popup — anywhere on the Linux
desktop.

Highlight text in any app → press your hotkey → a popup appears with the rewrite. Edit it,
switch modes, retry, then **Replace** (copies it back, pastes into the app) or **Copy**.

A single self-contained Rust binary (~7.7 MB), built on [egui](https://github.com/emilk/egui).
Talks straight to the Gemini API with your own key — no backend.

> **How this differs from the Android app.** Android has `ACTION_PROCESS_TEXT`, which lets
> an app add an entry to the system text-selection toolbar and replace the selection in
> place. **Linux has no equivalent.** So RefineAI here is triggered by a **global keyboard
> shortcut** you bind yourself, and the result is placed on the **clipboard** (then
> auto-pasted on a best-effort basis — see [Auto-paste](#auto-paste)). See
> [`PLAN.md`](./PLAN.md) for the full reasoning and the options that were weighed.

## Requirements

- A **Gemini API key** — free from [Google AI Studio](https://aistudio.google.com/apikey).
- A clipboard helper for your display server:
  - **Wayland** (Ubuntu's default): `wl-clipboard`
  - **X11**: `xclip`
- *(Optional)* an input tool for auto-paste: `ydotool` (Wayland) or `xdotool` (X11).

The binary itself only links glibc and your system's graphics libraries (loaded at
runtime), so it runs across distros without bundling a toolkit.

## Install on Ubuntu

Tested against Ubuntu 22.04 / 24.04 (GNOME on Wayland).

**1. Install the runtime tools.**

```bash
sudo apt update
sudo apt install wl-clipboard          # Wayland (Ubuntu default)
# sudo apt install xclip               # ...or this if you're on an X11 session
# sudo apt install ydotool             # optional: enables auto-paste on Wayland
```

Not sure which session you're on? Run `echo $XDG_SESSION_TYPE` (`wayland` or `x11`).

**2. Build from source.** You need the Rust toolchain:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # if you don't have Rust
sudo apt install build-essential libgl1-mesa-dev                 # build deps
```

Then build and install. The easiest path is the bundled script, which builds the release
binary, drops it on your `PATH`, and adds an app icon + menu entry:

```bash
cd refineai-linux
./install.sh
```

Uninstall with `./install.sh --uninstall` (your config and history are left in place). Or do
it by hand:

```bash
cargo build --release
install -Dm755 target/release/refineai ~/.local/bin/refineai
```

Make sure `~/.local/bin` is on your `PATH` (it is by default on Ubuntu). Check with
`refineai --help`.

**3. Add your API key.**

```bash
refineai --settings
```

Paste your Gemini key, pick a model if you like, and hit **Save**.

**4. Bind a global shortcut.** (GNOME / Ubuntu)

*Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts → `+`*

| Field | Value |
| --- | --- |
| Name | RefineAI |
| Command | `refineai` |
| Shortcut | e.g. `Ctrl+Alt+R` |

On KDE: *System Settings → Shortcuts → Custom Shortcuts*. On Sway/Hyprland: add a
`bindsym`/`bind` line that runs `refineai`.

## Usage

1. Select text in any app.
2. Press your shortcut.
3. In the popup: edit the result, switch mode (Coherent / Formal / Concise / Fix grammar),
   **Retry**, open **🕘 History**, or open **⚙ Settings**.
4. **Replace** puts the text on the clipboard and tries to paste it into your app;
   **Copy** just copies it. `Esc` or **Cancel** closes without touching anything.

### Auto-paste

"Replace" always copies the result to your clipboard, so you can paste it yourself with
`Ctrl+V`. It *also* attempts a real paste for you — but that needs an input tool:

- **X11**: install `xdotool` and it works.
- **Wayland**: needs `ydotool`, which requires a background daemon running as root
  (`ydotoold`). Without it, Wayland's security model blocks synthetic input, so you'll just
  press `Ctrl+V` yourself. This is a Wayland limitation, not a RefineAI bug.

## Settings & data

| What | Where |
| --- | --- |
| Config (key, model, modes, toggles) | `~/.config/refineai/config.json` (`0600`) |
| History log | `~/.local/share/refineai/history.json` |

- **Modes** are editable in the config file — add, remove, or change the prompt for any of
  them.
- **Low latency** sends `thinkingLevel: "minimal"` (Gemini 3 models only).
- **Direct replace** skips the preview and copies as soon as the rewrite arrives.
- **History** logs every rewrite that returns; soft-deleted entries are purged 30 days
  later (swept at launch). Turn it off with "Save history". Browse it from the **🕘 History**
  button: reuse (**Use**), **Copy**, or **Delete** a past rewrite. Deletes go to a **Recycle
  bin** tab you can **Restore** from (or **Delete forever**).
- **Debug mode** shows a live, timestamped activity log inside the popup — handy for
  diagnosing a request that errors or hangs. Your API key is never written to the log.

## Models

Free-text in Settings, since Google rotates names. Defaults to `gemini-3.5-flash-lite`
(fastest, free tier). `gemini-3.6-flash` / `gemini-3.5-flash` also work on the free tier;
Pro models need billing. Gemini 2.x is retired on new keys.

## Privacy

Text you refine is sent to Google's Gemini API with your key, and nowhere else. The key
lives in `~/.config/refineai/` with `0600` permissions. Note that Google may use free-tier
API input to improve their models — don't refine anything sensitive on a free key.

## Project layout

```
refineai-linux/
├── Cargo.toml
├── PLAN.md              # design doc: why this shape, options weighed
├── install.sh          # per-user install/uninstall + .desktop entry
├── assets/refineai.svg # app icon
└── src/
    ├── main.rs          # entry: read selection → popup
    ├── gemini.rs        # PURE request build + parse + sanitize (unit-tested)
    ├── refine.rs        # settings + API call + history, shared
    ├── platform.rs      # X11/Wayland: read selection, clipboard, auto-paste
    ├── history.rs       # JSON log, soft-delete, restore, 30-day purge
    ├── config.rs        # config load/save
    ├── debuglog.rs      # thread-safe activity log for debug mode
    ├── theme.rs         # design tokens + reusable gradient widgets
    └── ui.rs            # the egui popup
```

Run the tests with `cargo test`.
```
