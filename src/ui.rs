//! The floating popup. One eframe window with a small state machine:
//! Loading → Editing (preview) / Error, plus a Settings view and a NeedKey
//! prompt when no API key is configured.

use crate::config::Config;
use crate::debuglog::DebugLog;
use crate::history;
use crate::platform;
use crate::refine;
use crate::theme;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    rx: Option<Receiver<Result<String, String>>>,
    status: Option<String>,
    came_from_settings_only: bool,
    log: DebugLog,
    /// Cached rows for the History view; reloaded on open and after edits.
    history_items: Vec<history::Entry>,
    /// History view showing the recycle bin (soft-deleted) rather than active rows.
    history_bin: bool,
}

impl App {
    pub fn new(cfg: Config, original: String) -> Self {
        let mut app = App {
            cfg,
            original,
            result: String::new(),
            view: View::Loading,
            rx: None,
            status: None,
            came_from_settings_only: false,
            log: DebugLog::default(),
            history_items: Vec::new(),
            history_bin: false,
        };
        if app.cfg.api_key.trim().is_empty() {
            app.view = View::NeedKey;
        } else if app.original.trim().is_empty() {
            app.view = View::Error("No text selected. Highlight some text, then trigger RefineAI.".into());
        }
        app
    }

    /// Open straight into settings (used by `refineai --settings`).
    pub fn settings_only(cfg: Config) -> Self {
        App {
            cfg,
            original: String::new(),
            result: String::new(),
            view: View::Settings,
            rx: None,
            status: None,
            came_from_settings_only: true,
            log: DebugLog::default(),
            history_items: Vec::new(),
            history_bin: false,
        }
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
        let mode = self.cfg.active_mode;
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
                if theme::pill(ui, &m.name, self.cfg.active_mode == i).clicked()
                    && self.cfg.active_mode != i
                {
                    changed = Some(i);
                }
            }
            if let Some(i) = changed {
                self.cfg.active_mode = i;
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
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let diff = now.saturating_sub(created_at);
    match diff {
        0..=59 => "just now".to_string(),
        60..=3599 => format!("{}m ago", diff / 60),
        3600..=86399 => format!("{}h ago", diff / 3600),
        _ => format!("{}d ago", diff / 86400),
    }
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
