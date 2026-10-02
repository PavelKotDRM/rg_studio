use eframe::egui::{self, Align, Color32, Layout, RichText, Stroke, TextStyle};

use crate::{
    app::RgStudio,
    options::{Category, OPTIONS, OptionKind},
    preview::PreviewResult,
    regex_builder::RegexBuilder,
    theme::{self, Palette},
};

mod dialogs;
mod output;

const PREVIEW_TEXT_HEIGHT: f32 = 116.0;
const PREVIEW_FIXED_HEIGHT: f32 = 204.0;
const PREVIEW_COLLAPSED_HEIGHT: f32 = 168.0;
const MIN_OPTIONS_HEIGHT: f32 = 64.0;

pub(crate) fn render(app: &mut RgStudio, ui: &mut egui::Ui) {
    let context = ui.ctx().clone();
    app.search.poll(&context);

    render_header(app, ui);
    let palette = Palette::new(app.dark_theme);
    render_search_form(app, ui, palette);
    render_option_toolbar(app, ui, palette);

    let remaining_height = ui.available_height();
    let preview_height = if app.preview_open {
        (remaining_height - PREVIEW_FIXED_HEIGHT - MIN_OPTIONS_HEIGHT)
            .clamp(0.0, PREVIEW_TEXT_HEIGHT)
    } else {
        0.0
    };
    let preview_reserved_height = if app.preview_open {
        PREVIEW_FIXED_HEIGHT + preview_height
    } else {
        PREVIEW_COLLAPSED_HEIGHT
    };
    let option_height = (remaining_height - preview_reserved_height).max(0.0);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), option_height),
        Layout::top_down(Align::Min),
        |ui| render_option_catalog(app, ui, palette),
    );

    app.rebuild_options();
    render_preview(app, ui, palette, preview_height);
    output::render_command(app, ui, palette);
    output::search_results_window(app, &context, palette);
    dialogs::about_window(app, &context, palette);
    dialogs::regex_builder_window(app, &context, palette);
}

pub(super) fn themed_text_edit(edit: egui::TextEdit, palette: Palette) -> egui::TextEdit {
    edit.background_color(palette.field_surface)
        .text_color(palette.text)
}

fn render_header(app: &mut RgStudio, ui: &mut egui::Ui) {
    let palette = Palette::new(app.dark_theme);
    let mut toggle_theme = false;
    let mut open_about = false;

    egui::Frame::group(ui.style())
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.border))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("rg studio").strong().color(palette.accent));
                ui.label(RichText::new("A visual workspace for ripgrep").color(palette.muted_text));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    open_about = ui.button("About").clicked();
                    toggle_theme = ui
                        .button(if app.dark_theme {
                            "Light theme"
                        } else {
                            "Dark theme"
                        })
                        .clicked();
                });
            })
        });

    if toggle_theme {
        app.dark_theme = !app.dark_theme;
        theme::apply(ui.ctx(), app.dark_theme);
        let theme = if app.dark_theme {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        };
        *ui.style_mut() = (*ui.ctx().style_of(theme)).clone();
        ui.ctx().request_repaint();
    }
    if open_about {
        app.show_about = true;
    }
}

fn render_search_form(app: &mut RgStudio, ui: &mut egui::Ui, palette: Palette) {
    let mut pattern_changed = false;
    let mut path_changed = false;
    let mut open_builder = false;

    egui::Frame::group(ui.style())
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.border))
        .show(ui, |ui| {
            ui.columns(2, |columns| {
                columns[0].label(
                    RichText::new("SEARCH PATTERN (REGEX)")
                        .small()
                        .strong()
                        .color(palette.muted_text),
                );
                columns[0].horizontal(|ui| {
                    let input_width = (ui.available_width() - 128.0).max(120.0);
                    pattern_changed |= ui
                        .add_sized(
                            egui::vec2(input_width, 36.0),
                            themed_text_edit(
                                egui::TextEdit::singleline(&mut app.command.pattern),
                                palette,
                            )
                            .font(TextStyle::Monospace)
                            .hint_text("e.g. TODO|FIXME"),
                        )
                        .on_hover_text(
                            "This searches file contents. For file names, use Files > Include/exclude glob, e.g. *.rs.",
                        )
                        .changed();
                    open_builder = ui.button("Regex builder").clicked();
                });

                columns[1].label(
                    RichText::new("SEARCH IN")
                        .small()
                        .strong()
                        .color(palette.muted_text),
                );
                path_changed |= columns[1]
                    .add_sized(
                        egui::vec2(columns[1].available_width(), 36.0),
                        themed_text_edit(
                            egui::TextEdit::singleline(&mut app.command.path),
                            palette,
                        )
                        .hint_text("."),
                    )
                    .changed();
            });
        });

    if pattern_changed || path_changed {
        app.copied = false;
    }
    if open_builder {
        app.regex_builder = RegexBuilder::from_pattern(&app.command.pattern);
        app.show_regex_builder = true;
    }

    let arguments_width = ui.available_width();
    egui::Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.border))
        .show(ui, |ui| {
            ui.set_width(arguments_width);
            egui::CollapsingHeader::new(
                RichText::new("Advanced arguments")
                    .strong()
                    .color(palette.text),
            )
            .show_background(false)
            .default_open(false)
            .id_salt("advanced_arguments")
            .show(ui, |ui| {
                let response = ui.add_sized(
                    egui::vec2(ui.available_width(), 34.0),
                    themed_text_edit(
                        egui::TextEdit::singleline(&mut app.command.extra_arguments),
                        palette,
                    )
                    .font(TextStyle::Monospace)
                    .hint_text("Repeatable or newer ripgrep arguments"),
                );
                if response.changed() {
                    app.copied = false;
                }
            });
        });
}

fn render_option_toolbar(app: &mut RgStudio, ui: &mut egui::Ui, palette: Palette) {
    ui.horizontal_wrapped(|ui| {
        for category in Category::ALL {
            let selected = app.category == category;
            let button = egui::Button::new(RichText::new(category.title()).color(if selected {
                palette.text
            } else {
                palette.muted_text
            }))
            .fill(if selected {
                palette.selected_surface
            } else {
                Color32::TRANSPARENT
            })
            .stroke(Stroke::new(
                1.0,
                if selected {
                    palette.accent
                } else {
                    palette.border
                },
            ));

            if ui.add(button).clicked() {
                app.category = category;
            }
        }

        ui.separator();
        ui.add(
            themed_text_edit(egui::TextEdit::singleline(&mut app.filter), palette)
                .desired_width(210.0)
                .hint_text("Filter options"),
        );
    });
}

fn render_option_catalog(app: &mut RgStudio, ui: &mut egui::Ui, palette: Palette) {
    let filter = app.filter.trim().to_ascii_lowercase();
    let visible_count = OPTIONS
        .iter()
        .filter(|spec| spec.category == app.category && option_matches_filter(spec, &filter))
        .count();

    let heading_width = ui.available_width();
    egui::Frame::new().fill(palette.surface).show(ui, |ui| {
        ui.set_width(heading_width);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(app.category.title())
                    .strong()
                    .color(palette.text),
            );
            ui.label(
                RichText::new(format!("{visible_count} options"))
                    .small()
                    .color(palette.muted_text),
            );
        });
    });

    egui::ScrollArea::vertical()
        .min_scrolled_height(0.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if visible_count == 0 {
                ui.label(RichText::new("No options match this filter.").color(palette.muted_text));
                return;
            }

            let columns = if ui.available_width() >= 960.0 { 2 } else { 1 };
            let card_width = (ui.available_width() - 12.0 * (columns - 1) as f32) / columns as f32;
            let mut current_column = 0;
            let mut changed = false;
            let mut newly_enabled_flag = None;

            egui::Grid::new("option_grid")
                .num_columns(columns)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    for (index, (spec, state)) in
                        OPTIONS.iter().zip(&mut app.option_states).enumerate()
                    {
                        if spec.category != app.category || !option_matches_filter(spec, &filter) {
                            continue;
                        }

                        ui.push_id(index, |ui| {
                            let fill = if state.enabled {
                                palette.selected_surface
                            } else {
                                palette.surface
                            };
                            let border = if state.enabled {
                                palette.accent
                            } else {
                                palette.border
                            };

                            egui::Frame::group(ui.style())
                                .fill(fill)
                                .stroke(Stroke::new(1.0, border))
                                .show(ui, |ui| {
                                    ui.set_width((card_width - 16.0).max(180.0));
                                    match spec.kind {
                                        OptionKind::Switch => {
                                            ui.horizontal(|ui| {
                                                let response =
                                                    ui.checkbox(&mut state.enabled, spec.label);
                                                changed |= response.changed();
                                                if response.changed() && state.enabled {
                                                    newly_enabled_flag = Some(spec.flag);
                                                }
                                                ui.label(
                                                    RichText::new(spec.flag)
                                                        .small()
                                                        .monospace()
                                                        .color(palette.muted_text),
                                                );
                                            });
                                        }
                                        OptionKind::Value(hint) => {
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    let response =
                                                        ui.checkbox(&mut state.enabled, spec.label);
                                                    changed |= response.changed();
                                                    if response.changed() && state.enabled {
                                                        newly_enabled_flag = Some(spec.flag);
                                                    }
                                                    let input_width =
                                                        (ui.available_width() - 8.0).max(90.0);
                                                    let input_response =
                                                        ui.add_enabled_ui(state.enabled, |ui| {
                                                            ui.add_sized(
                                                                egui::vec2(input_width, 30.0),
                                                                themed_text_edit(
                                                                    egui::TextEdit::singleline(
                                                                        &mut state.value,
                                                                    ),
                                                                    palette,
                                                                )
                                                                .hint_text(hint),
                                                            )
                                                        });
                                                    changed |= input_response.inner.changed();
                                                });
                                                ui.label(
                                                    RichText::new(spec.flag)
                                                        .small()
                                                        .monospace()
                                                        .color(palette.muted_text),
                                                );
                                            });
                                        }
                                    }
                                });
                        });

                        current_column += 1;
                        if current_column == columns {
                            ui.end_row();
                            current_column = 0;
                        }
                    }

                    if current_column != 0 {
                        ui.end_row();
                    }
                });

            if let Some(flag) = newly_enabled_flag {
                app.enforce_exclusive_option(flag);
            }
            if changed {
                app.copied = false;
            }
        });
}

fn option_matches_filter(spec: &crate::options::OptionSpec, filter: &str) -> bool {
    filter.is_empty()
        || spec.label.to_ascii_lowercase().contains(filter)
        || spec.flag.contains(filter)
}

fn render_preview(app: &mut RgStudio, ui: &mut egui::Ui, palette: Palette, preview_height: f32) {
    let mut show_body = app.preview_open;
    egui::Frame::group(ui.style())
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.border))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button(if show_body { "Hide" } else { "Show" }).clicked() {
                    show_body = !show_body;
                }
                ui.strong("Live text preview");
                if show_body && preview_height <= 0.0 {
                    ui.label(
                        RichText::new("Increase window height to show the preview.")
                            .small()
                            .color(palette.muted_text),
                    );
                }
            });

            if show_body && preview_height > 0.0 {
                ui.label(
                    RichText::new(
                        "Regex-only preview; it does not run ripgrep or filesystem filters.",
                    )
                    .small()
                    .color(palette.muted_text),
                );

                ui.columns(2, |columns| {
                    columns[0].label(
                        RichText::new("TEXT TO SEARCH")
                            .small()
                            .strong()
                            .color(palette.muted_text),
                    );
                    columns[0].add_sized(
                        egui::vec2(columns[0].available_width(), preview_height),
                        themed_text_edit(egui::TextEdit::multiline(&mut app.sample_text), palette)
                            .desired_rows(4)
                            .font(TextStyle::Monospace)
                            .hint_text("Paste or type sample text"),
                    );

                    columns[1].label(
                        RichText::new("MATCHES")
                            .small()
                            .strong()
                            .color(palette.muted_text),
                    );
                    let result = app.preview_result();
                    egui::ScrollArea::vertical()
                        .max_height(preview_height)
                        .min_scrolled_height(0.0)
                        .auto_shrink([false, false])
                        .show(&mut columns[1], |ui| {
                            match &result {
                                PreviewResult::EmptyPattern => {
                                    ui.colored_label(
                                        palette.muted_text,
                                        "Enter a pattern to see matching text.",
                                    );
                                }
                                PreviewResult::UnsupportedEngine => {
                                    ui.colored_label(
                                        palette.warning,
                                        "Preview unavailable: PCRE2/auto engine.",
                                    );
                                }
                                PreviewResult::Invalid(error) => {
                                    ui.colored_label(
                                        palette.danger,
                                        format!("Invalid regex: {error}"),
                                    );
                                }
                                PreviewResult::Matches(spans) if spans.is_empty() => {
                                    ui.colored_label(palette.muted_text, "No matches found.");
                                }
                                PreviewResult::Matches(spans) => {
                                    ui.colored_label(
                                        palette.success,
                                        format!("{} matches", spans.len()),
                                    );
                                    if spans.iter().any(|matched| matched.start == matched.end) {
                                        ui.label(
                                            RichText::new("Zero-width matches are marked with |.")
                                                .small()
                                                .color(palette.muted_text),
                                        );
                                    }
                                }
                            }

                            if app.sample_text.is_empty() {
                                ui.label(
                                    RichText::new("Add sample text to preview the result.")
                                        .color(palette.muted_text),
                                );
                            } else {
                                let spans = match &result {
                                    PreviewResult::Matches(spans) => spans.as_slice(),
                                    _ => &[],
                                };
                                let mut layouter =
                                    |ui: &egui::Ui,
                                     text: &dyn egui::TextBuffer,
                                     wrap_width: f32| {
                                        let mut job =
                                            output::highlighted_job(text.as_str(), spans, palette);
                                        job.wrap.max_width = wrap_width;
                                        ui.fonts_mut(|fonts| fonts.layout_job(job))
                                    };
                                let output_height = ui.available_height().max(0.0);
                                ui.add_sized(
                                    egui::vec2(ui.available_width(), output_height),
                                    egui::TextEdit::multiline(&mut app.sample_text)
                                        .id_salt("preview_matches")
                                        .font(TextStyle::Monospace)
                                        .desired_rows(4)
                                        .background_color(palette.surface)
                                        .text_color(Color32::WHITE)
                                        .frame(egui::Frame::NONE)
                                        .interactive(false)
                                        .layouter(&mut layouter),
                                );
                            }
                        });
                });
            }
        });

    app.preview_open = show_body;
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Color32, Context, Pos2, RawInput, Rect, vec2};

    use crate::{app::RgStudio, theme};

    use super::output::{
        ansi_layout_job, highlighted_job, raw_page_match_ranges, render_paged_output,
    };
    use super::render;

    fn rendered_bottom(size: eframe::egui::Vec2, preview_open: bool) -> f32 {
        let context = Context::default();
        let mut app = RgStudio {
            preview_open,
            ..RgStudio::default()
        };
        theme::apply(&context, app.dark_theme);

        let screen_rect = Rect::from_min_size(Pos2::ZERO, size);
        let mut bottom = 0.0;
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ui| {
                render(&mut app, ui);
                bottom = ui.min_rect().bottom();
            },
        );
        output.textures_delta.clear();
        bottom
    }

    #[test]
    fn expanded_preview_and_command_fit_the_default_viewport() {
        let bottom = rendered_bottom(vec2(1000.0, 640.0), true);
        assert!(bottom <= 640.0, "rendered content ends at y={bottom}");
    }

    #[test]
    fn command_fits_the_minimum_viewport_with_preview_collapsed() {
        let bottom = rendered_bottom(vec2(700.0, 440.0), false);
        assert!(bottom <= 440.0, "rendered content ends at y={bottom}");
    }

    #[test]
    fn command_fits_the_minimum_viewport_with_preview_open() {
        let bottom = rendered_bottom(vec2(700.0, 440.0), true);
        assert!(bottom <= 440.0, "rendered content ends at y={bottom}");
    }

    #[test]
    fn highlighted_preview_preserves_spaces_around_matches() {
        let text = "  before  match  after  \nnext ";
        let start = text.find("match").unwrap();
        let matches = std::iter::once(start..start + "match".len()).collect::<Vec<_>>();
        let job = highlighted_job(text, &matches, theme::Palette::new(true));

        assert_eq!(job.text, text);
        assert!(job.keep_trailing_whitespace);
    }

    #[test]
    fn renders_ansi_color_codes_as_styled_text() {
        let raw = "src/file.rs:1:\u{1b}[1;38;2;255;80;0mneedle\u{1b}[0m";
        let job = ansi_layout_job(raw, Color32::WHITE);
        let colored_match = job
            .sections
            .iter()
            .find(|section| section.format.color == Color32::from_rgb(255, 80, 0));

        assert_eq!(job.text, "src/file.rs:1:needle");
        assert!(colored_match.is_some());
    }

    #[test]
    fn highlights_matches_in_unparsed_raw_pages_without_rewriting_text() {
        let matcher = regex::Regex::new("needle").unwrap();
        let raw = "src/file.rs:1:before needle after";
        let ranges = raw_page_match_ranges(raw, 0..raw.len(), Some(&matcher));

        assert_eq!(ranges.len(), 1);
        assert_eq!(&raw[ranges[0].clone()], "needle");
        assert!(
            raw_page_match_ranges(
                "src/file.rs:1:\u{1b}[31mneedle\u{1b}[0m",
                0.."src/file.rs:1:\u{1b}[31mneedle\u{1b}[0m".len(),
                Some(&matcher)
            )
            .is_empty()
        );
    }

    #[test]
    fn cli_output_wraps_long_lines_to_the_window_width() {
        let width = 360.0;
        let context = Context::default();
        let palette = theme::Palette::new(true);
        let text = "x".repeat(8192);
        let pages = std::iter::once(0..text.len()).collect::<Vec<_>>();
        let mut page = 0;
        let mut right = 0.0;
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 440.0))),
                ..Default::default()
            },
            |ui| {
                render_paged_output(
                    ui,
                    &text,
                    &pages,
                    &mut page,
                    palette.text,
                    eframe::egui::Id::new("long_cli_output"),
                    &[],
                );
                right = ui.min_rect().right();
            },
        );
        output.textures_delta.clear();

        assert!(right <= width, "rendered output extends to x={right}");
    }
}
