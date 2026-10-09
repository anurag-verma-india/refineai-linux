//! Centralized design tokens + reusable widgets for the "Minimalist Modern"
//! look (see design.md): off-white canvas, white card surfaces, electric-blue
//! gradient accents, generous-but-practical spacing and visible focus rings.
//!
//! egui ships its own fonts, so the brief's Inter / Calistoga / JetBrains Mono
//! faces are approximated with the bundled proportional + monospace families at
//! a deliberate type scale rather than bundling extra font files.

use egui::{Color32, FontId, Margin, Rounding, Stroke, TextStyle};

// ── Color tokens ──────────────────────────────────────────────────────────
pub const BG: Color32 = Color32::from_rgb(0xFA, 0xFA, 0xFA); // canvas
pub const SURFACE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF); // cards/inputs
pub const FG: Color32 = Color32::from_rgb(0x0F, 0x17, 0x2A); // foreground text
pub const MUTED: Color32 = Color32::from_rgb(0xF1, 0xF5, 0xF9); // muted surface
pub const SECONDARY: Color32 = Color32::from_rgb(0x64, 0x74, 0x8B); // secondary text
pub const BORDER: Color32 = Color32::from_rgb(0xE2, 0xE8, 0xF0); // hairline borders
pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0x52, 0xFF); // primary accent
pub const ACCENT2: Color32 = Color32::from_rgb(0x4D, 0x7C, 0xFF); // accent secondary
pub const DANGER: Color32 = Color32::from_rgb(0xDC, 0x26, 0x26); // error text

// ── Radii ─────────────────────────────────────────────────────────────────
pub const R_SM: f32 = 10.0;
pub const R_MD: f32 = 12.0;
pub const R_LG: f32 = 14.0;

/// Install the global style/visuals. Call once with the egui context.
pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut v = egui::Visuals::light();

    v.override_text_color = Some(FG);
    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.extreme_bg_color = SURFACE; // TextEdit background
    v.faint_bg_color = MUTED;
    v.code_bg_color = MUTED;
    v.hyperlink_color = ACCENT;

    v.window_rounding = Rounding::same(R_LG);
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    v.window_shadow = egui::epaint::Shadow::NONE;
    v.popup_shadow = egui::epaint::Shadow::NONE;

    let w = |bg: Color32, weak: Color32, stroke: Color32, fg: Color32, r: f32, exp: f32| {
        egui::style::WidgetVisuals {
            bg_fill: bg,
            weak_bg_fill: weak,
            bg_stroke: Stroke::new(1.0_f32, stroke),
            fg_stroke: Stroke::new(1.0_f32, fg),
            rounding: Rounding::same(r),
            expansion: exp,
        }
    };
    // Resting label/separator styling.
    v.widgets.noninteractive = w(BG, BG, BORDER, FG, R_MD, 0.0);
    // Secondary (outline) buttons + inputs at rest.
    v.widgets.inactive = w(SURFACE, SURFACE, BORDER, FG, R_SM, 0.0);
    // Hover: muted fill, accent-tinted border.
    v.widgets.hovered = w(MUTED, MUTED, ACCENT2, FG, R_SM, 1.0);
    // Active / keyboard-focused: accent border = visible focus ring.
    v.widgets.active = w(MUTED, MUTED, ACCENT, FG, R_SM, 1.0);
    v.widgets.open = w(SURFACE, SURFACE, BORDER, FG, R_SM, 0.0);

    // Text selection + selected highlights.
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0x00, 0x52, 0xFF, 48);
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);

    style.visuals = v;

    // Type scale — bold, characterful headings; clean, readable body.
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(22.0)),
        (TextStyle::Body, FontId::proportional(14.0)),
        (TextStyle::Button, FontId::proportional(14.0)),
        (TextStyle::Small, FontId::proportional(12.0)),
        (TextStyle::Monospace, FontId::monospace(12.5)),
    ]
    .into();

    // Spacing — generous but practical.
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
    style.spacing.interact_size.y = 30.0;
    style.spacing.window_margin = Margin::same(16.0);
    style.spacing.menu_margin = Margin::same(8.0);

    ctx.set_style(style);
}

/// A white card surface with a hairline border and rounded corners.
pub fn card() -> egui::Frame {
    egui::Frame::none()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .rounding(Rounding::same(R_LG))
        .inner_margin(Margin::same(14.0))
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let c = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(c(a.r(), b.r()), c(a.g(), b.g()), c(a.b(), b.b()))
}

/// Paint a diagonal (135°) gradient into a rounded rect. Uses the tessellator so
/// the fill honors the corner radius, then recolors each vertex by position.
pub fn paint_gradient(painter: &egui::Painter, rect: egui::Rect, rounding: Rounding, c0: Color32, c1: Color32) {
    // Feathering must stay OFF: egui's anti-aliased fill is built from overlapping
    // translucent triangles that assume one uniform color. Recoloring each vertex
    // for the gradient makes those overlaps blend against each other, which shows
    // up as diagonal seams. A non-feathered fill is a clean opaque mesh, so the
    // per-vertex gradient interpolates smoothly.
    // Crisp anti-aliased silhouette underneath, in the average color. The
    // non-feathered gradient drawn on top covers the interior exactly, leaving
    // only this sub-pixel AA rim to smooth the rounded corners.
    painter.rect_filled(rect, rounding, lerp_color(c0, c1, 0.5));

    let shape = egui::epaint::RectShape::filled(rect, rounding, Color32::WHITE);
    let mut mesh = egui::epaint::Mesh::default();
    let opts = egui::epaint::TessellationOptions {
        feathering: false,
        ..Default::default()
    };
    let mut tess = egui::epaint::Tessellator::new(painter.ctx().pixels_per_point(), opts, [1, 1], vec![]);
    tess.tessellate_rect(&shape, &mut mesh);
    let w = rect.width().max(1.0);
    let h = rect.height().max(1.0);
    for vert in mesh.vertices.iter_mut() {
        let tx = (vert.pos.x - rect.left()) / w;
        let ty = (vert.pos.y - rect.top()) / h;
        vert.color = lerp_color(c0, c1, (tx + ty) * 0.5);
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// The signature gradient primary action. `min_width` lets a row of primaries
/// line up; pass `None` to size to content.
pub fn primary_button(ui: &mut egui::Ui, text: &str, min_width: Option<f32>) -> egui::Response {
    let padding = egui::vec2(16.0, 8.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(14.0), Color32::WHITE);
    let mut size = galley.size() + padding * 2.0;
    size.y = size.y.max(32.0);
    if let Some(mw) = min_width {
        size.x = size.x.max(mw);
    }
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let (c0, c1) = if resp.is_pointer_button_down_on() {
            (lerp_color(ACCENT, FG, 0.14), lerp_color(ACCENT2, FG, 0.14)) // pressed: darker
        } else if resp.hovered() {
            (lerp_color(ACCENT, SURFACE, 0.08), lerp_color(ACCENT2, SURFACE, 0.08)) // hover: lift
        } else {
            (ACCENT, ACCENT2)
        };
        paint_gradient(ui.painter(), rect, Rounding::same(R_MD), c0, c1);
        let pos = rect.center() - galley.size() / 2.0;
        ui.painter().galley(pos, galley, Color32::WHITE);
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// A compact pill used for the mode switcher. Selected = gradient fill with
/// white text; unselected = muted surface with an accent border on hover.
pub fn pill(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let padding = egui::vec2(13.0, 6.0);
    let color = if selected { Color32::WHITE } else { SECONDARY };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(13.0), color);
    let size = galley.size() + padding * 2.0;
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let rounding = Rounding::same(rect.height() / 2.0);
        if selected {
            paint_gradient(ui.painter(), rect, rounding, ACCENT, ACCENT2);
        } else {
            let border = if resp.hovered() { ACCENT2 } else { BORDER };
            ui.painter().rect(rect, rounding, MUTED, Stroke::new(1.0_f32, border));
        }
        let pos = rect.center() - galley.size() / 2.0;
        ui.painter().galley(pos, galley, color);
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// The branded header: gradient "logo" chip + wordmark, plus a trailing slot.
pub fn header(ui: &mut egui::Ui, trailing: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
        paint_gradient(ui.painter(), rect, Rounding::same(9.0), ACCENT, ACCENT2);
        let g = ui
            .painter()
            .layout_no_wrap("R".to_owned(), FontId::proportional(17.0), Color32::WHITE);
        ui.painter().galley(rect.center() - g.size() / 2.0, g, Color32::WHITE);
        ui.add_space(4.0);
        ui.heading("RefineAI");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), trailing);
    });
}
