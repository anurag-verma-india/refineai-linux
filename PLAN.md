# RefineAI for Linux — Design & Options

Porting the one-tap "select → Refine → replace" flow from Android to the Linux desktop,
keeping it **as light as possible** while running across **multiple Linux environments**
(different distros, X11 **and** Wayland, GNOME/KDE/others).

This doc is a decision document: it lays out the one hard constraint, the realistic UX
model, and then **five implementation options weighed against each other**. Pick one and
I'll scaffold it.

---

## 1. The core problem (read this first — it shapes everything)

The Android app rests entirely on **`ACTION_PROCESS_TEXT`**: any app can add an entry to
the system text-selection toolbar, and returning text *replaces the selection in place*.

**Linux has no equivalent.** There is no system-wide "add an action to the selection
toolbar" mechanism that works across GTK, Qt, Electron, terminals, and browsers. So the
port cannot reproduce the exact Android interaction. The closest workable model is:

```
select text  →  press a global hotkey  →  popup reads the selection  →  refine  →  result
                                                                               ↓
                                        copy to clipboard (always works) · or auto-paste/type
```

Two sub-problems, and **both are worse on Wayland than X11**:

| Step | X11 | Wayland |
| --- | --- | --- |
| **Global hotkey** | We can grab keys directly | We can't grab globally; rely on the **compositor's own keybinding** (user binds a key to run our command) or the newer `GlobalShortcuts` xdg portal (partial support) |
| **Read the selection** | PRIMARY selection via `xclip`/`xsel` — zero extra clicks | `wl-paste --primary` (works on most compositors) |
| **Put text back "in place"** | `xdotool` can type or synth-paste | `ydotool` (needs a **root daemon**), or `wtype` (broken on KDE Plasma, barely maintained) |

**Honest consequence:** reliable "replace in place" like Android is **not achievable
universally**. The dependable default is **"refined text → clipboard, you press Ctrl+V"**,
with optional auto-type/auto-paste as a best-effort extra where the tools exist. I'd rather
state that up front than promise the Android magic and have it fail on half of setups.

**Recommended UX that works everywhere:** ship the app as a command the user binds to a
hotkey *in their own desktop's keyboard settings*. This sidesteps grabbing keys ourselves
and works identically on X11, Wayland, GNOME, KDE, Sway, etc.

---

## 2. What exists on Linux already (research)

No tool reproduces `PROCESS_TEXT`, but several occupy the same space and prove the
hotkey-driven model is the norm:

- **[LinguistAI](https://mynixos.com/nixpkgs/package/gnomeExtensions.linguistai)** — GNOME
  Shell extension. Select → hotkey → AI (Claude/Gemini) → result. Closest in spirit, but
  **GNOME-only** (a Shell extension can't run on KDE/Sway), so it fails our
  "multiple environments" goal. Good proof of the interaction, wrong distribution model.
- **[Clipbud](https://rust-digger.code-maven.com/crates/clipbud)** — cross-platform Rust
  tool. Registers custom prompt actions (Fix / Summarize / Friendly) and runs them on the
  **clipboard** via a global hotkey. This is essentially our Option 2, already proven.
- **[Keygeist](https://github.com/mudler/keygeist)** — Linux, hotkey-driven, **types** the
  AI response back via a keyboard emulator and uses a `zenity` dialog for input. Proof that
  the "glue existing CLI tools" approach (Option 1) is viable.
- **[ReplaceSelectionAI](https://smartvless.duckdns.org/en-US/firefox/addon/replaceselectionai/)**
  (Firefox) and **[CapyBro](https://www.producthunt.com/products/capybro)** (Windows) —
  same workflow, other platforms. Confirms the pattern is well-trodden.

Supporting tools we'd build on: **[wl-clipboard](https://packages.guix.gnu.org/packages/wl-clipboard/)**
(Wayland copy/paste), `xclip`/`xsel` (X11), `xdotool` (X11 input),
**ydotool** (Wayland input, [reported awkward — needs a root daemon and is keyboard-layout
naive](https://forum.endeavouros.com/t/shortcut-to-paste-pre-defined-text-with-built-in-tools/61192)),
and **espanso** (a mature Wayland-capable Rust app to study for packaging).

Takeaway: the field is **either DE-locked (GNOME extension) or clipboard-based glue**.
A single, portable, selection-aware binary with a nice popup is an open niche.

---

## 3. The five implementation options

Scored 1–5 (5 = best). "UX fidelity" = how close to the Android popup experience.

| # | Stack | Binary / footprint | Portability (distros + X11/Wayland) | Build effort | UX fidelity | Runtime deps |
| --- | --- | --- | --- | --- | --- | --- |
| **1** | **Shell/Python glue** — `rofi`/`yad`/`zenity` + `xclip`/`wl-clipboard` + `curl` | **~0 MB** (scripts) | ★★★☆☆ varies by what's installed | ★★★★★ fastest | ★★☆☆☆ basic popup | many external CLIs |
| **2** | **Rust + egui**, one static binary ⭐ | ~10–18 MB single file | ★★★★★ | ★★★☆☆ | ★★★★☆ custom popup | clipboard/input helpers only |
| **3** | **Rust + GTK4**, Flatpak | ~2 MB app + GTK runtime | ★★★★☆ (Flathub) | ★★☆☆☆ steeper | ★★★★★ native GNOME feel | GTK runtime (shared) |
| **4** | **Tauri** (Rust + system WebView) | ~5 MB binary | ★★★☆☆ needs WebKitGTK | ★★★☆☆ | ★★★★★ web UI | system WebKitGTK |
| **5** | **Python + GTK/Qt** | no single binary | ★★★☆☆ | ★★★★☆ | ★★★★☆ | Python + GUI runtime |

Notes grounded in the research:

- **egui** is "the lightest single-file option" for a popup that opens, shows, exits; pin the
  version (it has breaking changes) and keep API work off the UI thread.
  ([comparison](https://lukaskalbertodt.github.io/2023/02/03/tauri-iced-egui-performance-comparison.html))
- **Tauri** is small *on disk* but **not a true self-contained binary on Linux** — it leans on
  the system WebKitGTK, which varies across distros. Weakest on our portability goal despite
  looking "light."
- **GTK-rs** gives the most native result but is "mostly good for Linux" and has a steep
  curve; Flatpak's shared runtime is the clean distribution path.
- **Option 1** is genuinely the lightest to *write and ship initially*, but pushes the weight
  onto the user's machine (must have `rofi`, `wl-clipboard`, etc.) and the popup can't match
  the Android dialog (mode switcher, editable preview, retry).

### Distribution (orthogonal to stack)

- **AppImage** — single file, runs on any distro, no install. Best match for "multiple
  environments." Pairs naturally with Option 2.
- **Flatpak/Flathub** — shared runtimes, sandbox, a store. Best for Option 3; sandbox makes
  clipboard/input access fiddlier.
- **Static musl binary + `.desktop`** — smallest, most Unix-y; manual hotkey setup.

---

## 4. Recommendation

**Option 2 (Rust + egui), shipped as a static binary + AppImage, triggered by a
user-bound hotkey, defaulting to clipboard output with best-effort auto-paste.**

Why it wins *this* brief specifically:
- **Lightest that still owns its UX** — one self-contained file, no Python/Node/WebKit
  runtime, so it genuinely "runs in multiple Linux environments" from a single download.
- **Portable by construction** — abstract the platform bits behind a small trait with X11
  and Wayland backends (`xclip`/`xdotool` vs `wl-clipboard`/`ydotool`), detected at runtime.
- **Reuses the Android architecture cleanly** — the Android app already isolates a *pure*
  `GeminiProtocol` (build request / parse / sanitize, no platform types). That ports almost
  line-for-line to a pure Rust `gemini` module, keeping the tested-core discipline.
- Only real cost vs GTK/Tauri is a slightly less "native GNOME" look — acceptable for a
  tiny floating popup.

If instead your priority is **native polish + Flathub reach**, go **Option 3**. If you want
the **fastest possible prototype tonight** and don't mind runtime deps, **Option 1** gets a
working tool in an afternoon and can be rewritten later.

---

## 5. Feature mapping (Android → Linux), under Option 2

| Android feature | Linux plan |
| --- | --- |
| Selection-toolbar `Refine` action | Global hotkey (user-bound) reads PRIMARY selection / clipboard |
| Preview dialog: edit / retry / cancel | egui popup window, same controls |
| Mode switcher (Coherent/Formal/Concise/Fix) | Same modes, re-run on click |
| Editable prompts, add/delete | Stored in a TOML/JSON config in `~/.config/refineai/` |
| In-place replace, clipboard fallback | **Inverted:** clipboard default; auto-paste/type as best-effort by backend |
| Copy button | Native |
| Local history + 30-day recycle bin | SQLite (`~/.local/share/refineai/`), mirrors Room schema + soft-delete + purge-on-launch |
| Direct replace mode | "Skip preview" → straight to clipboard/auto-paste |
| Settings / API key | Config file; key in `~/.config` with `600` perms (optionally libsecret later) |
| Gemini client + pure protocol | Pure Rust `gemini` module (ports `GeminiProtocol`) + `reqwest` client |
| `targetSdk`/Play specifics | N/A — replaced by AppImage/Flatpak packaging |

---

## 6. Rough structure (Option 2)

```
refineai-linux/
├── Cargo.toml
├── src/
│   ├── main.rs              # hotkey entry → read selection → popup
│   ├── gemini/              # PURE: request build + parse + sanitize (ports GeminiProtocol)
│   ├── refine.rs            # settings + API + history (ports RefineService)
│   ├── platform/            # trait + x11.rs / wayland.rs (selection, paste, type)
│   ├── history.rs           # SQLite, soft-delete, 30-day purge
│   ├── config.rs            # modes, key, model
│   └── ui/popup.rs          # egui dialog
├── packaging/               # AppImage recipe, .desktop, icon
└── README.md
```

---

## 7. Open decisions for you

1. **Which option** (default: **2**).
2. **Distribution**: AppImage (recommended) / Flatpak / plain binary — or several.
3. **"Replace in place" ambition**: clipboard-only (robust) vs. also attempt auto-type
   (needs `ydotool`/root on Wayland — adds friction). Default: clipboard + best-effort.
4. **X11 only first**, or **X11 + Wayland from day one**? (Wayland input-injection is the
   expensive part.)
5. **Reuse Gemini only**, or also add **local Ollama** support (several Linux peers do)?
```

