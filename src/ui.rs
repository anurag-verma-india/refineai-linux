//! The floating popup. One eframe window with a small state machine:
//! Loading → Editing (preview) / Error, plus a Settings view and a NeedKey
//! prompt when no API key is configured.

use crate::config::{Config, Mode};
use crate::debuglog::DebugLog;
use crate::history;
use crate::platform;
use crate::refine;
use crate::theme;
use crate::util::unix_secs;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

enum View {
    NeedKey,
    Loading,
    Editing,
    Error(String),
    Settings,
    History,
}

pub struct App {
    cfg: Config,
    original: String,
    result: String,
    view: View,
    /// Mode selected for the current session. Starts at `cfg.default_mode`; the
    /// pills in the editing view flip it, but the persisted default is untouched
    /// unless it's changed in Settings.
    active_mode: usize,
    rx: Option<Receiver<Result<String, String>>>,
    status: Option<String>,
    came_from_settings_only: bool,
    log: DebugLog,
    /// Cached rows for the History view; reloaded on open and after edits.
    history_items: Vec<history::Entry>,
    /// History view showing the recycle bin (soft-deleted) rather than active rows.
    history_bin: bool,
    /// Set when the app opens straight into Loading: the refine must be kicked
    /// off on the first `update` frame, once we have an egui `Context`.
    pending_refine: bool,
}

impl App {
    /// Shared field initialization; `new` and `settings_only` only differ in the
    /// starting view and a couple of flags, set by the caller.
    fn base(cfg: Config, original: String, view: View) -> Self {
        let active_mode = cfg.default_mode;
        App {
            cfg,
            original,
            result: String::new(),
            view,
            active_mode,
            rx: None,
            status: None,
            came_from_settings_only: false,
            log: DebugLog::default(),
            history_items: Vec::new(),
            history_bin: false,
            pending_refine: false,
        }
    }

    pub fn new(cfg: Config, original: String) -> Self {
        let mut app = App::base(cfg, original, View::Loading);
        if app.cfg.api_key.trim().is_empty() {
            app.view = View::NeedKey;
        } else if app.original.trim().is_empty() {
            app.view = View::Error("No text selected. Highlight some text, then trigger RefineAI.".into());
        } else {
            // Valid key + selection: refine right away. We can't start the worker
            // here (no egui Context yet), so flag it for the first `update` frame.
            app.pending_refine = true;
        }
        app
    }

    /// Open straight into settings (used by `refineai --settings`).
    pub fn settings_only(cfg: Config) -> Self {
        let mut app = App::base(cfg, String::new(), View::Settings);
        app.came_from_settings_only = true;
        app
    }

    fn start_refine(&mut self, ctx: &egui::Context) {
        self.status = None;
        self.view = View::Loading;
        self.log.clear();
        self.log.log("starting refine");
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let cfg = self.cfg.clone();
        let text = self.original.clone();
        let mode = self.active_mode;
        let log = self.log.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            // Convert an unexpected panic into an error instead of leaving the
            // popup stuck on "Refining…" forever.
            let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                refine::refine(&cfg, &text, mode, &log)
            }))
            .unwrap_or_else(|_| {
                log.log("worker thread panicked");
                Err("Something went wrong while refining (worker crashed).".to_string())
            });
            let _ = tx.send(out);
            ctx.request_repaint();
        });
    }

    fn apply(&mut self, ctx: &egui::Context, close: bool) {
        if let Err(e) = platform::set_clipboard(&self.result) {
            self.status = Some(format!("Clipboard error: {e}"));
            return;
        }
        if close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            // Best-effort auto-paste once focus returns to the original app.
            thread::spawn(|| {
                thread::sleep(Duration::from_millis(150));
                platform::try_paste();
            });
        } else {
            self.status = Some("Copied to clipboard.".into());
        }
    }

    fn open_history(&mut self) {
        self.status = None;
        self.history_bin = false;
        self.view = View::History;
        self.reload_history();
    }

    fn reload_history(&mut self) {
        self.history_items = if self.history_bin {
            history::list_deleted()
        } else {
            history::list_active()
        };
    }

    /// Leave a secondary view (Settings / History) back to where it makes sense.
    fn go_back(&mut self, ctx: &egui::Context) {
        if self.came_from_settings_only {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.cfg.api_key.trim().is_empty() {
            self.view = View::NeedKey;
        } else if !self.result.is_empty() {
            self.view = View::Editing;
        } else {
            self.start_refine(ctx);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // First frame after opening straight into Loading: start the refine now
        // that we have a Context. Without this the popup sits on "Refining…"
        // forever (and the debug panel stays empty, since nothing is logged).
        if self.pending_refine {
            self.pending_refine = false;
            self.start_refine(ctx);
        }

        // Collect any finished background refine.
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(res) => {
                    match res {
                        Ok(text) => {
                            self.result = text;
                            if self.cfg.direct_replace {
                                self.apply(ctx, true);
                            } else {
                                self.view = View::Editing;
                            }
                        }
                        Err(e) => self.view = View::Error(e),
                    }
                    self.rx = None;
                }
                Err(TryRecvError::Disconnected) => {
                    self.log.log("worker channel disconnected");
                    self.view = View::Error("The refine worker stopped unexpectedly.".into());
                    self.rx = None;
                }
                Err(TryRecvError::Empty) => {
                    // Still running: keep polling so we never get stuck on Loading
                    // even if the one-shot repaint is missed, and stream the log.
                    ctx.request_repaint_after(Duration::from_millis(120));
                }
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            theme::header(ui, |ui| {
                if ui.button("⚙").on_hover_text("Settings").clicked() {
                    self.view = View::Settings;
                }
                if ui.button("🕘").on_hover_text("History").clicked() {
                    self.open_history();
                }
            });
            ui.add_space(12.0);

            theme::card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                match &self.view {
                    View::NeedKey => {
                        ui.label(
                            egui::RichText::new("Connect Gemini to get started")
                                .size(15.0)
                                .strong(),
                        );
                        ui.add_space(4.0);
                        ui.colored_label(
                            theme::SECONDARY,
                            "Grab a free key at aistudio.google.com/apikey, then add it in Settings.",
                        );
                        ui.add_space(12.0);
                        if theme::primary_button(ui, "Open Settings", None).clicked() {
                            self.view = View::Settings;
                        }
                    }
                    View::Loading => {
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.add_space(4.0);
                            ui.colored_label(theme::SECONDARY, "Refining…");
                        });
                        ui.add_space(12.0);
                    }
                    View::Error(msg) => {
                        let msg = msg.clone();
                        ui.colored_label(theme::DANGER, &msg);
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            let can_retry = !self.original.trim().is_empty()
                                && !self.cfg.api_key.trim().is_empty();
                            if can_retry && theme::primary_button(ui, "Retry", None).clicked() {
                                self.start_refine(ctx);
                            }
                            if ui.button("Settings").clicked() {
                                self.view = View::Settings;
                            }
                            if ui.button("Close").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                    }
                    View::Editing => {
                        self.editing_view(ctx, ui);
                    }
                    View::Settings => {
                        self.settings_view(ctx, ui);
                    }
                    View::History => {
                        self.history_view(ctx, ui);
                    }
                }

                // Live activity log — only when debug mode is on.
                if self.cfg.debug && !self.log.is_empty() {
                    ui.add_space(10.0);
                    self.debug_panel(ui);
                }
            });

            if let Some(s) = &self.status {
                ui.add_space(8.0);
                ui.colored_label(theme::SECONDARY, s);
            }
        });
    }
}

impl App {
    fn editing_view(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Mode switcher — pills, re-runs instantly on change.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            let mut changed = None;
            for (i, m) in self.cfg.modes.iter().enumerate() {
                if theme::pill(ui, &m.name, self.active_mode == i).clicked()
                    && self.active_mode != i
                {
                    changed = Some(i);
                }
            }
            if let Some(i) = changed {
                self.active_mode = i;
                self.start_refine(ctx);
            }
        });
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(160.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.result)
                        .desired_width(f32::INFINITY)
                        .desired_rows(6)
                        .margin(egui::Margin::same(10.0)),
                );
            });

        ui.add_space(4.0);
        ui.collapsing(
            egui::RichText::new("Show original").size(13.0).color(theme::SECONDARY),
            |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.original.as_str())
                        .desired_width(f32::INFINITY)
                        .margin(egui::Margin::same(10.0)),
                );
            },
        );

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if theme::primary_button(ui, "Replace", None)
                .on_hover_text("Copy & paste into the app")
                .clicked()
            {
                self.apply(ctx, true);
            }
            if ui.button("Copy").clicked() {
                self.apply(ctx, false);
            }
            if ui.button("Retry").clicked() {
                self.start_refine(ctx);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }

    /// A collapsible, monospace panel streaming the current run's activity log.
    fn debug_panel(&self, ui: &mut egui::Ui) {
        let header = egui::RichText::new("Debug log")
            .size(12.0)
            .strong()
            .color(theme::SECONDARY);
        egui::CollapsingHeader::new(header)
            .default_open(true)
            .show(ui, |ui| {
                egui::Frame::none()
                    .fill(theme::MUTED)
                    .rounding(egui::Rounding::same(theme::R_SM))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::ScrollArea::vertical()
                            .max_height(150.0)
                            .auto_shrink([false, true])
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for line in self.log.lines() {
                                    ui.label(
                                        egui::RichText::new(line)
                                            .monospace()
                                            .size(11.0)
                                            .color(theme::FG),
                                    );
                                }
                            });
                    });
            });
    }

    fn settings_view(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_width(ui.available_width());
            field_label(ui, "Gemini API key");
            ui.add(
                egui::TextEdit::singleline(&mut self.cfg.api_key)
                    .password(true)
                    .desired_width(f32::INFINITY)
                    .margin(egui::Margin::same(10.0))
                    .hint_text("Paste your key"),
            );
            ui.add_space(12.0);

            field_label(ui, "Model");
            ui.add(
                egui::TextEdit::singleline(&mut self.cfg.model)
                    .desired_width(f32::INFINITY)
                    .margin(egui::Margin::same(10.0))
                    .font(egui::TextStyle::Monospace),
            );
            ui.add_space(14.0);

            ui.checkbox(&mut self.cfg.low_latency, "Low latency (Gemini 3 only)");
            ui.add_space(2.0);
            ui.checkbox(&mut self.cfg.direct_replace, "Direct replace (skip preview)");
            ui.add_space(2.0);
            ui.checkbox(&mut self.cfg.save_history, "Save history");
            ui.add_space(2.0);
            ui.checkbox(&mut self.cfg.debug, "Debug mode (show activity log in the popup)");

            ui.add_space(16.0);
            self.modes_editor(ui);

            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if theme::primary_button(ui, "Save", None).clicked() {
                    match self.cfg.save() {
                        Ok(_) => self.status = Some("Settings saved.".into()),
                        Err(e) => self.status = Some(format!("Save failed: {e}")),
                    }
                }
                let back = if self.came_from_settings_only { "Close" } else { "Back" };
                if ui.button(back).clicked() {
                    self.go_back(ctx);
                }
            });
            ui.add_space(10.0);
            ui.colored_label(
                theme::SECONDARY,
                egui::RichText::new(format!(
                    "Session: {} · config: {}",
                    platform::server_name(),
                    Config::path().display()
                ))
                .size(11.0),
            );
        });
    }

    /// The Modes editor: add, rename, re-word, remove, and choose the default
    /// mode — the one a fresh popup opens in. Each mode is one card.
    fn modes_editor(&mut self, ui: &mut egui::Ui) {
        field_label(ui, "Modes");
        ui.colored_label(
            theme::SECONDARY,
            egui::RichText::new(
                "Rewrite styles shown as pills in the popup. The default is selected when RefineAI opens.",
            )
            .size(11.0),
        );
        ui.add_space(8.0);

        // Deferred so we don't mutate `modes` while iterating it.
        enum ModeAct {
            SetDefault(usize),
            Remove(usize),
        }
        let mut act: Option<ModeAct> = None;
        let default_mode = self.cfg.default_mode;
        let can_remove = self.cfg.modes.len() > 1;

        for (i, mode) in self.cfg.modes.iter_mut().enumerate() {
            theme::card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let is_default = default_mode == i;
                    let label = if is_default { "✓ Default" } else { "Set default" };
                    if theme::pill(ui, label, is_default).clicked() && !is_default {
                        act = Some(ModeAct::SetDefault(i));
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut mode.name)
                            .desired_width(f32::INFINITY)
                            .margin(egui::Margin::same(8.0))
                            .hint_text("Mode name"),
                    );
                });
                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::multiline(&mut mode.prompt)
                        .desired_width(f32::INFINITY)
                        .desired_rows(2)
                        .margin(egui::Margin::same(8.0))
                        .hint_text("Instruction sent to Gemini with your text"),
                );
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(can_remove, |ui| {
                        let btn = ui.button("Remove");
                        let btn = if can_remove {
                            btn.on_hover_text("Delete this mode")
                        } else {
                            btn.on_hover_text("Keep at least one mode")
                        };
                        if btn.clicked() {
                            act = Some(ModeAct::Remove(i));
                        }
                    });
                });
            });
            ui.add_space(8.0);
        }

        if ui.button("+ Add mode").clicked() {
            self.cfg.modes.push(Mode {
                name: "New mode".into(),
                prompt: String::new(),
            });
        }

        match act {
            Some(ModeAct::SetDefault(i)) => self.cfg.default_mode = i,
            Some(ModeAct::Remove(i)) => {
                self.cfg.modes.remove(i);
                self.cfg.default_mode = adjust_index(self.cfg.default_mode, i, self.cfg.modes.len());
                self.active_mode = adjust_index(self.active_mode, i, self.cfg.modes.len());
            }
            None => {}
        }
    }

    fn history_view(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Segmented toggle: active history vs. recycle bin.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if theme::pill(ui, "History", !self.history_bin).clicked() && self.history_bin {
                self.history_bin = false;
                self.reload_history();
            }
            if theme::pill(ui, "Recycle bin", self.history_bin).clicked() && !self.history_bin {
                self.history_bin = true;
                self.reload_history();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Back").clicked() {
                    self.go_back(ctx);
                }
            });
        });
        ui.add_space(10.0);

        if self.history_items.is_empty() {
            ui.add_space(8.0);
            ui.colored_label(
                theme::SECONDARY,
                if self.history_bin {
                    "Recycle bin is empty."
                } else {
                    "No history yet. Refined text shows up here."
                },
            );
            ui.add_space(8.0);
            return;
        }

        // Deferred mutation so we don't edit the list while iterating it.
        enum Act {
            Use(usize),
            Delete(String),
            Restore(String),
            Purge(String),
        }
        let mut act: Option<Act> = None;

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (idx, e) in self.history_items.iter().enumerate() {
                    theme::card().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&e.mode)
                                    .size(12.0)
                                    .strong()
                                    .color(theme::ACCENT),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.colored_label(
                                        theme::SECONDARY,
                                        egui::RichText::new(time_ago(e.created_at)).size(11.0),
                                    );
                                },
                            );
                        });
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(truncate(&e.result, 240)).size(13.0));
                        if !e.original.is_empty() {
                            ui.add_space(2.0);
                            ui.colored_label(
                                theme::SECONDARY,
                                egui::RichText::new(format!("from: {}", truncate(&e.original, 120)))
                                    .size(11.0),
                            );
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            if self.history_bin {
                                if theme::primary_button(ui, "Restore", None).clicked() {
                                    act = Some(Act::Restore(e.id.clone()));
                                }
                                if ui.button("Copy").clicked() {
                                    let _ = platform::set_clipboard(&e.result);
                                }
                                if ui
                                    .button("Delete forever")
                                    .on_hover_text("Remove permanently")
                                    .clicked()
                                {
                                    act = Some(Act::Purge(e.id.clone()));
                                }
                            } else {
                                if theme::primary_button(ui, "Use", None)
                                    .on_hover_text("Load into the editor")
                                    .clicked()
                                {
                                    act = Some(Act::Use(idx));
                                }
                                if ui.button("Copy").clicked() {
                                    let _ = platform::set_clipboard(&e.result);
                                }
                                if ui
                                    .button("Delete")
                                    .on_hover_text("Move to recycle bin")
                                    .clicked()
                                {
                                    act = Some(Act::Delete(e.id.clone()));
                                }
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
            });

        match act {
            Some(Act::Use(idx)) => {
                if let Some(e) = self.history_items.get(idx) {
                    self.result = e.result.clone();
                    self.original = e.original.clone();
                    self.status = None;
                    self.view = View::Editing;
                }
            }
            Some(Act::Delete(id)) => {
                history::soft_delete(&id);
                self.reload_history();
                self.status = Some("Moved to recycle bin.".into());
            }
            Some(Act::Restore(id)) => {
                history::restore(&id);
                self.reload_history();
                self.status = Some("Restored.".into());
            }
            Some(Act::Purge(id)) => {
                history::purge(&id);
                self.reload_history();
                self.status = Some("Deleted permanently.".into());
            }
            None => {}
        }
    }
}

/// Collapse a possibly-long, multi-line string to a single trimmed preview line.
fn truncate(s: &str, max: usize) -> String {
    let one_line = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() > max {
        let cut: String = one_line.chars().take(max).collect();
        format!("{}…", cut.trim_end())
    } else {
        one_line
    }
}

/// A coarse "x ago" label for a unix-seconds timestamp.
fn time_ago(created_at: u64) -> String {
    let diff = unix_secs().saturating_sub(created_at);
    match diff {
        0..=59 => "just now".to_string(),
        60..=3599 => format!("{}m ago", diff / 60),
        3600..=86399 => format!("{}h ago", diff / 3600),
        _ => format!("{}d ago", diff / 86400),
    }
}

/// Keep a stored index valid after the item at `removed` is deleted from a list
/// that now has `len` items: shift down if it sat after the gap, and clamp so it
/// never points past the end.
fn adjust_index(idx: usize, removed: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let shifted = if idx > removed { idx - 1 } else { idx };
    shifted.min(len - 1)
}

/// A small uppercase field caption above an input.
fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .color(theme::SECONDARY)
            .strong(),
    );
    ui.add_space(4.0);
}
