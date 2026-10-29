use eframe::egui::{self, Color32, RichText, Vec2};

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub surface: Color32,
    pub field: Color32,
    pub selected: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub tint: Color32,
    pub green: Color32,
    pub red: Color32,
}
impl Palette {
    pub fn new(dark: bool) -> Self {
        let c = Color32::from_rgb;
        if dark {
            Self {
                background: c(24, 25, 28),
                surface: c(34, 35, 39),
                field: c(26, 27, 30),
                selected: c(35, 112, 238),
                ink: c(242, 242, 245),
                muted: c(164, 166, 175),
                border: c(57, 59, 65),
                accent: c(125, 177, 255),
                tint: c(37, 47, 64),
                green: c(107, 222, 177),
                red: c(255, 143, 153),
            }
        } else {
            Self {
                background: c(245, 245, 247),
                surface: Color32::WHITE,
                field: c(237, 238, 241),
                selected: c(42, 94, 221),
                ink: c(30, 31, 36),
                muted: c(101, 104, 115),
                border: c(220, 222, 227),
                accent: c(38, 87, 207),
                tint: c(231, 239, 255),
                green: c(14, 125, 88),
                red: c(186, 43, 66),
            }
        }
    }
    pub fn of(ui: &egui::Ui) -> Self {
        Self::new(ui.visuals().dark_mode)
    }
}

pub fn install(ctx: &egui::Context) {
    // Use the locally installed macOS font; do not redistribute it.
    let mut fonts = egui::FontDefinitions::default();
    fonts.families.insert(
        egui::FontFamily::Name("Semibold".into()),
        fonts.families[&egui::FontFamily::Proportional].clone(),
    );
    if let Ok(bytes) = std::fs::read("/System/Library/Fonts/SFNS.ttf") {
        // SF defaults to optical size 28, which is too delicate for compact controls.
        // Explicit text-size optics and real weights keep labels legible at 12–14 pt.
        let mut regular = egui::FontData::from_owned(bytes);
        regular.tweak.coords = egui::epaint::text::VariationCoords::new([
            ("opsz", 17.0),
            ("wght", 400.0),
            ("wdth", 100.0),
        ]);
        regular.tweak.hinting = Some(false);
        let mut semibold = regular.clone();
        semibold.tweak.coords.push("wght", 600.0);
        fonts.font_data.insert("macOS".into(), regular.into());
        fonts
            .font_data
            .insert("macOS Semibold".into(), semibold.into());
        let mut heading_fallbacks = fonts.families[&egui::FontFamily::Proportional].clone();
        heading_fallbacks.insert(0, "macOS Semibold".into());
        fonts
            .families
            .insert(egui::FontFamily::Name("Semibold".into()), heading_fallbacks);
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "macOS".into());
    }
    ctx.set_fonts(fonts);
    ctx.set_theme(egui::ThemePreference::System);
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let p = Palette::new(theme == egui::Theme::Dark);
        ctx.style_mut_of(theme, |s| {
            s.spacing.item_spacing = Vec2::new(10.0, 10.0);
            s.spacing.button_padding = Vec2::new(12.0, 7.0);
            s.spacing.window_margin = egui::Margin::same(24);
            s.spacing.menu_margin = egui::Margin::same(12);
            s.spacing.tooltip_width = 340.0;
            s.spacing.interact_size.y = 32.0;
            s.spacing.combo_height = 360.0;
            s.visuals.override_text_color = Some(p.ink);
            s.visuals.panel_fill = p.background;
            s.visuals.window_fill = p.surface;
            s.visuals.window_stroke = egui::Stroke::new(1.0, p.border);
            s.visuals.window_corner_radius = egui::CornerRadius::same(16);
            s.visuals.window_highlight_topmost = false;
            s.visuals.handle_shape = egui::style::HandleShape::Circle;
            s.visuals.selection.bg_fill = p.selected;
            s.visuals.selection.stroke = egui::Stroke::new(1.0, Color32::WHITE);
            s.visuals.hyperlink_color = p.accent;
            s.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
            for v in [
                &mut s.visuals.widgets.inactive,
                &mut s.visuals.widgets.hovered,
                &mut s.visuals.widgets.active,
                &mut s.visuals.widgets.open,
            ] {
                v.corner_radius = egui::CornerRadius::same(9);
                v.bg_stroke = egui::Stroke::NONE;
                v.fg_stroke = egui::Stroke::new(1.0, p.ink);
                v.bg_fill = p.field;
                v.weak_bg_fill = p.field;
                v.expansion = 0.0;
            }
            s.visuals.widgets.hovered.bg_fill = p.tint;
            s.visuals.widgets.hovered.weak_bg_fill = p.tint;
            s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.accent);
            s.visuals.widgets.active.bg_fill = p.tint;
            s.visuals.widgets.active.weak_bg_fill = p.tint;
            s.visuals.widgets.open.bg_fill = p.tint;
            s.visuals.widgets.open.weak_bg_fill = p.tint;
            for (style, size) in [
                (egui::TextStyle::Body, 13.0),
                (egui::TextStyle::Button, 13.0),
                (egui::TextStyle::Small, 12.0),
                (egui::TextStyle::Heading, 20.0),
            ] {
                s.text_styles
                    .insert(style, egui::FontId::proportional(size));
            }
        });
    }
}

pub fn segments<T: Copy + PartialEq>(ui: &mut egui::Ui, value: &mut T, items: &[(T, &str)]) {
    let p = Palette::of(ui);
    let width = ui.available_width();
    egui::Frame::new()
        .fill(p.field)
        .corner_radius(9)
        .inner_margin(3)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.spacing_mut().button_padding.x = 6.0;
            ui.horizontal(|ui| {
                let cell =
                    ((width - 6.0 - 3.0 * (items.len() - 1) as f32) / items.len() as f32).max(30.0);
                for &(item, text) in items {
                    let active = *value == item;
                    if ui
                        .add_sized(
                            [cell, 30.0],
                            egui::Button::selectable(
                                active,
                                RichText::new(text)
                                    .size(12.5)
                                    .variation("wght", if active { 600.0 } else { 400.0 })
                                    .color(if active { p.ink } else { p.muted }),
                            )
                            .fill(if active {
                                if ui.visuals().dark_mode {
                                    Color32::from_rgb(65, 67, 74)
                                } else {
                                    Color32::WHITE
                                }
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(6)
                            .truncate(),
                        )
                        .clicked()
                    {
                        *value = item;
                    }
                }
            });
        });
}

pub fn preset(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    active: bool,
    logo: bool,
) -> egui::Response {
    let p = Palette::of(ui);
    let (r, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::Role::Button,
            ui.is_enabled(),
            active,
            format!("{title}. {subtitle}"),
        )
    });
    let focused = response.hovered() || response.has_focus();
    let painter = ui.painter();
    painter.rect_filled(r, 13, if active || focused { p.tint } else { p.surface });
    painter.rect_stroke(
        r,
        13,
        egui::Stroke::new(
            if active { 1.5 } else { 1.0 },
            if active || focused {
                p.accent
            } else {
                p.border
            },
        ),
        egui::StrokeKind::Inside,
    );
    let icon =
        egui::Rect::from_center_size(r.left_center() + Vec2::new(25.0, 0.0), Vec2::splat(22.0));
    let compact = r.width() < 230.0;
    if compact {
        // Narrow sidebar presets keep their full text without a decorative icon.
    } else if logo {
        let c = icon.center();
        painter.add(egui::Shape::closed_line(
            vec![
                c + Vec2::new(0.0, -11.0),
                c + Vec2::new(10.0, 0.0),
                c + Vec2::new(0.0, 11.0),
                c + Vec2::new(-10.0, 0.0),
            ],
            egui::Stroke::new(1.7, p.accent),
        ));
        painter.circle_filled(c, 2.5, p.accent);
    } else {
        image_mark(painter, icon, p.accent);
    }
    painter.text(
        r.min + Vec2::new(if compact { 14.0 } else { 48.0 }, 18.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::new(13.0, egui::FontFamily::Name("Semibold".into())),
        p.ink,
    );
    painter.text(
        r.min + Vec2::new(if compact { 14.0 } else { 48.0 }, 38.0),
        egui::Align2::LEFT_CENTER,
        subtitle,
        egui::FontId::proportional(12.0),
        p.muted,
    );
    if active {
        painter.circle_filled(r.right_top() + Vec2::new(-14.0, 14.0), 3.0, p.accent);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn image_mark(painter: &egui::Painter, rect: egui::Rect, color: Color32) {
    let stroke = egui::Stroke::new(1.6, color);
    painter.rect_stroke(rect, 4, stroke, egui::StrokeKind::Inside);
    painter.circle_filled(
        rect.min + Vec2::new(rect.width() * 0.27, rect.height() * 0.28),
        1.8,
        color,
    );
    painter.add(egui::Shape::line(
        vec![
            rect.left_bottom() + Vec2::new(3.0, -4.0),
            rect.center() + Vec2::new(-2.0, 0.0),
            rect.center() + Vec2::new(3.0, 4.0),
            rect.right_top() + Vec2::new(-3.0, rect.height() * 0.4),
        ],
        stroke,
    ));
}

pub fn upload_mark(ui: &mut egui::Ui) {
    let p = Palette::of(ui);
    let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(r, 14, p.tint);
    let c = r.center();
    let s = egui::Stroke::new(1.8, p.accent);
    painter.line_segment([c + Vec2::new(0.0, -10.0), c + Vec2::new(0.0, 5.0)], s);
    painter.add(egui::Shape::line(
        vec![
            c + Vec2::new(-5.0, 0.0),
            c + Vec2::new(0.0, 5.0),
            c + Vec2::new(5.0, 0.0),
        ],
        s,
    ));
    painter.add(egui::Shape::line(
        vec![
            c + Vec2::new(-10.0, 3.0),
            c + Vec2::new(-10.0, 10.0),
            c + Vec2::new(10.0, 10.0),
            c + Vec2::new(10.0, 3.0),
        ],
        s,
    ));
}

/// A vector gear keeps the Settings icon sharp at every display scale.
pub fn gear_mark(painter: &egui::Painter, rect: egui::Rect, color: Color32) {
    let radius = rect.width() / 2.0;
    let points = (0..48)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 48.0;
            let r = radius * if matches!(i % 6, 1..=4) { 1.0 } else { 0.76 };
            rect.center() + Vec2::angled(angle) * r
        })
        .collect();
    let stroke = egui::Stroke::new(1.35, color);
    painter.add(egui::Shape::closed_line(points, stroke));
    painter.circle_stroke(rect.center(), radius * 0.32, stroke);
}
