use std::sync::Arc;

use eframe::egui::{self, Color32, Context, FontId, Stroke, TextStyle};

#[derive(Clone, Copy)]
pub(crate) struct Palette {
    pub(crate) surface: Color32,
    pub(crate) raised_surface: Color32,
    pub(crate) field_surface: Color32,
    pub(crate) selected_surface: Color32,
    pub(crate) border: Color32,
    pub(crate) text: Color32,
    pub(crate) muted_text: Color32,
    pub(crate) accent: Color32,
    pub(crate) accent_text: Color32,
    pub(crate) match_background: Color32,
    pub(crate) match_text: Color32,
    pub(crate) success: Color32,
    pub(crate) danger: Color32,
    pub(crate) warning: Color32,
}

impl Palette {
    pub(crate) const fn new(dark: bool) -> Self {
        if dark {
            Self {
                surface: Color32::from_rgb(20, 29, 44),
                raised_surface: Color32::from_rgb(27, 39, 58),
                field_surface: Color32::from_rgb(16, 25, 38),
                selected_surface: Color32::from_rgb(25, 55, 67),
                border: Color32::from_rgb(47, 64, 87),
                text: Color32::from_rgb(232, 239, 248),
                muted_text: Color32::from_rgb(160, 176, 198),
                accent: Color32::from_rgb(74, 206, 186),
                accent_text: Color32::from_rgb(13, 19, 30),
                match_background: Color32::from_rgb(92, 70, 24),
                match_text: Color32::from_rgb(255, 226, 143),
                success: Color32::from_rgb(100, 211, 157),
                danger: Color32::from_rgb(255, 126, 139),
                warning: Color32::from_rgb(240, 195, 110),
            }
        } else {
            Self {
                surface: Color32::from_rgb(255, 255, 255),
                raised_surface: Color32::from_rgb(239, 244, 249),
                field_surface: Color32::from_rgb(241, 246, 251),
                selected_surface: Color32::from_rgb(222, 245, 240),
                border: Color32::from_rgb(205, 216, 228),
                text: Color32::from_rgb(29, 42, 59),
                muted_text: Color32::from_rgb(89, 107, 128),
                accent: Color32::from_rgb(15, 125, 112),
                accent_text: Color32::WHITE,
                match_background: Color32::from_rgb(255, 237, 178),
                match_text: Color32::from_rgb(91, 61, 0),
                success: Color32::from_rgb(30, 128, 84),
                danger: Color32::from_rgb(186, 47, 65),
                warning: Color32::from_rgb(132, 73, 0),
            }
        }
    }
}

pub(crate) fn apply(context: &Context, dark: bool) {
    let palette = Palette::new(dark);
    let theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.override_text_color = Some(palette.text);
    visuals.weak_text_color = Some(palette.muted_text);
    visuals.weak_text_alpha = 1.0;
    visuals.disabled_alpha = 0.72;
    visuals.panel_fill = if dark {
        Color32::from_rgb(13, 19, 30)
    } else {
        Color32::from_rgb(244, 247, 251)
    };
    visuals.window_fill = palette.surface;
    visuals.extreme_bg_color = palette.raised_surface;
    visuals.faint_bg_color = palette.raised_surface;
    visuals.code_bg_color = palette.raised_surface;
    visuals.text_edit_bg_color = Some(palette.field_surface);
    visuals.collapsing_header_frame = false;
    visuals.hyperlink_color = palette.accent;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.danger;
    visuals.selection.bg_fill = palette.selected_surface;
    visuals.selection.stroke = Stroke::new(1.0, palette.accent);

    visuals.widgets.noninteractive.bg_fill = palette.surface;
    visuals.widgets.noninteractive.weak_bg_fill = palette.surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.inactive.bg_fill = palette.raised_surface;
    visuals.widgets.inactive.weak_bg_fill = palette.raised_surface;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.hovered.bg_fill = palette.selected_surface;
    visuals.widgets.hovered.weak_bg_fill = palette.selected_surface;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.active.bg_fill = palette.selected_surface;
    visuals.widgets.active.weak_bg_fill = palette.selected_surface;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.open.bg_fill = palette.selected_surface;
    visuals.widgets.open.weak_bg_fill = palette.selected_surface;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, palette.text);

    for style_theme in [egui::Theme::Dark, egui::Theme::Light] {
        let mut style = (*context.style_of(style_theme)).clone();
        style.visuals = visuals.clone();
        style.spacing.item_spacing = egui::vec2(11.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.interact_size = egui::vec2(40.0, 28.0);
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(14.0));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(26.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace(14.0));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(12.5));

        context.set_style_of(style_theme, Arc::new(style));
    }
    context.set_theme(theme);
}

#[cfg(test)]
mod tests {
    use super::{Palette, apply};
    use eframe::egui::{Color32, Context, Theme};

    fn luminance(color: Color32) -> f32 {
        let channel = |value: u8| {
            let value = f32::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };

        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    fn contrast_ratio(first: Color32, second: Color32) -> f32 {
        let first = luminance(first);
        let second = luminance(second);
        let (lighter, darker) = if first > second {
            (first, second)
        } else {
            (second, first)
        };

        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn text_colors_meet_wcag_aa_contrast_in_both_themes() {
        assert!(luminance(Palette::new(false).field_surface) > 0.85);

        for dark in [true, false] {
            let palette = Palette::new(dark);
            for background in [
                palette.surface,
                palette.raised_surface,
                palette.field_surface,
                palette.selected_surface,
            ] {
                assert!(contrast_ratio(palette.text, background) >= 4.5);
                assert!(contrast_ratio(palette.muted_text, background) >= 4.5);
            }
            assert!(contrast_ratio(palette.accent, palette.surface) >= 4.5);
            assert!(contrast_ratio(palette.accent_text, palette.accent) >= 4.5);
            assert!(contrast_ratio(palette.accent_text, palette.success) >= 4.5);
            assert!(contrast_ratio(palette.match_text, palette.match_background) >= 4.5);
            assert!(contrast_ratio(palette.success, palette.surface) >= 4.5);
            assert!(contrast_ratio(palette.danger, palette.surface) >= 4.5);
            assert!(contrast_ratio(palette.warning, palette.surface) >= 4.5);
        }
    }

    #[test]
    fn apply_updates_text_and_surface_colors_in_both_theme_styles() {
        let context = Context::default();

        for dark in [true, false] {
            let palette = Palette::new(dark);
            let active_theme = if dark { Theme::Dark } else { Theme::Light };
            apply(&context, dark);

            for style_theme in [Theme::Dark, Theme::Light] {
                let visuals = &context.style_of(style_theme).visuals;
                assert_eq!(visuals.override_text_color, Some(palette.text));
                assert_eq!(visuals.weak_text_color, Some(palette.muted_text));
                assert_eq!(visuals.widgets.active.fg_stroke.color, palette.text);
                assert_eq!(visuals.window_fill, palette.surface);
                assert_eq!(visuals.text_edit_bg_color, Some(palette.field_surface));
                assert!(!visuals.collapsing_header_frame);
                assert_eq!(visuals.widgets.noninteractive.bg_fill, palette.surface);
                assert_eq!(visuals.widgets.inactive.bg_fill, palette.raised_surface);
                assert_eq!(visuals.widgets.active.bg_fill, palette.selected_surface);
                assert_eq!(visuals.widgets.hovered.bg_fill, palette.selected_surface);
                assert_eq!(visuals.widgets.open.bg_fill, palette.selected_surface);
                assert_eq!(visuals.dark_mode, dark);
            }
            assert_eq!(context.style_of(active_theme).visuals.dark_mode, dark);
        }
    }
}
