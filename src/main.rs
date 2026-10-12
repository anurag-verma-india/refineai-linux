//! RefineAI for Linux — entry point.
//!
//! Launched by a user-bound global hotkey (see README). It grabs the current
//! selection, opens the popup, and exits when the popup closes.
//!
//!   refineai              read the selection and refine it
//!   refineai --settings   open the settings window
//!   refineai --help       usage

mod config;
mod debuglog;
mod gemini;
mod history;
mod platform;
mod refine;
mod store;
mod theme;
mod ui;
mod util;

use config::Config;

/// Builds the eframe `App` once egui has handed us a `CreationContext`. Boxed so
/// the selection-reading and settings-only paths can share one `run_native` call.
type AppBuilder = Box<dyn FnOnce(&eframe::CreationContext<'_>) -> Box<dyn eframe::App>>;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "RefineAI — rewrite the selected text with Gemini.\n\n\
             Usage:\n  refineai             Refine the current selection\n  \
             refineai --settings  Open settings\n  refineai --help      This help\n\n\
             Bind `refineai` to a keyboard shortcut in your desktop settings."
        );
        return Ok(());
    }

    // 30-day recycle-bin sweep, like the Android launch sweep.
    history::purge_expired();

    let cfg = Config::load();
    let settings_only = args.iter().any(|a| a == "--settings");

    let app: AppBuilder = if settings_only {
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            Box::new(ui::App::settings_only(cfg))
        })
    } else {
        let selection = platform::read_selection().unwrap_or_default();
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            Box::new(ui::App::new(cfg, selection))
        })
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([440.0, 380.0])
            .with_min_inner_size([360.0, 240.0])
            .with_resizable(true)
            .with_always_on_top()
            .with_title("RefineAI"),
        ..Default::default()
    };

    eframe::run_native("RefineAI", options, Box::new(|cc| Ok(app(cc))))
}
