# Configuration & data

Full reference for RefineAI's settings, config file, models, auto-paste and
privacy. The README covers the common toggles; this is the complete picture.

## Where things live

| What | Where |
| --- | --- |
| Config (key, model, modes, toggles) | `~/.config/refineai/config.json` (`0600`) |
| History log | `~/.local/share/refineai/history.json` |

The config file is written with `0600` permissions so your API key isn't
world-readable. Most settings are editable from the in-app **⚙ Settings** panel;
**modes** are edited directly in the JSON.

## Settings

Open with `refineai --settings`, or the **⚙** button in the popup.

| Setting | Default | What it does |
| --- | --- | --- |
| **Gemini API key** | *(empty)* | Your key from [Google AI Studio](https://aistudio.google.com/apikey). |
| **Model** | `gemini-3.5-flash-lite` | Free-text model id — see [Models](#models). |
| **Low latency** | on | Sends `thinkingLevel: "minimal"` (Gemini 3 models only) for a faster first token. |
| **Direct replace** | off | Skip the editable preview — refine and copy straight to the clipboard. |
| **Save history** | on | Log every rewrite that returns. See [History](#history). |
| **Debug mode** | off | Show a live, timestamped activity log inside the popup. Your API key is never written to the log. |

## config.json

```jsonc
{
  "api_key": "…",                 // your Gemini key (kept 0600)
  "model": "gemini-3.5-flash-lite",
  "low_latency": true,            // thinkingLevel: "minimal" on Gemini 3
  "direct_replace": false,        // skip the preview
  "save_history": true,
  "debug": false,
  "active_mode": 0,               // index into "modes" below
  "modes": [
    { "name": "Coherent",    "prompt": "Rewrite the text so it is clear and coherent while keeping the original meaning, tone and language." },
    { "name": "Formal",      "prompt": "Rewrite the text in a polished, professional and formal register, keeping the original meaning and language." },
    { "name": "Concise",     "prompt": "Rewrite the text to be as concise as possible without losing meaning, keeping the original language." },
    { "name": "Fix grammar", "prompt": "Correct only spelling, grammar and punctuation. Do not change wording, tone or meaning more than necessary." }
  ]
}
```

### Modes

Each mode is a `{ name, prompt }` pair shown as a pill in the popup. **Add,
remove, rename, or re-word** any of them by editing `modes` in the JSON — the
`prompt` is the instruction sent to Gemini alongside your selected text.
`active_mode` is the zero-based index of the mode selected by default; if it's
out of range it falls back to the first mode.

## Models

The model is free-text in Settings, because Google rotates names.

| Model | Notes |
| --- | --- |
| `gemini-3.5-flash-lite` | **Default** — fastest, free tier. |
| `gemini-3.6-flash` | Free tier. |
| `gemini-3.5-flash` | Free tier. |
| Pro models | Require billing enabled on your key. |

Gemini 2.x is retired on new keys. **Low latency** only affects Gemini 3 models.

## History

When **Save history** is on, every rewrite that returns is logged locally to
`history.json`. Browse it from the **🕘 History** button:

- **Use** — load a past rewrite back into the editor.
- **Copy** — copy it to the clipboard.
- **Delete** — soft-delete into the **Recycle bin** tab.

Soft-deleted entries sit in the recycle bin, where you can **Restore** them or
**Delete forever**. Expired entries are swept **30 days** after deletion
(checked at launch). Turn logging off entirely with **Save history**.

## Auto-paste

**Replace** always copies the result to your clipboard, so you can paste it
yourself with `Ctrl+V`. It *also* attempts a real paste for you — but that needs
an input tool:

- **X11**: install `xdotool` and it works.
- **Wayland**: needs `ydotool`, which requires a background daemon running as
  root (`ydotoold`). Without it, Wayland's security model blocks synthetic
  input, so you'll just press `Ctrl+V` yourself. This is a Wayland limitation,
  not a RefineAI bug.

Clipboard itself uses `wl-clipboard` (Wayland) or `xclip` (X11) — one of these
is required; see the README [Install](../README.md#install) section.

## Privacy

Text you refine is sent to Google's Gemini API with your key, and nowhere else —
no backend, no telemetry. The key lives in `~/.config/refineai/` with `0600`
permissions.

> **Note:** Google may use free-tier API input to improve their models — don't
> refine anything sensitive on a free key.
