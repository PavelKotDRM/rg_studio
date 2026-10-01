use std::ops::Range;

use eframe::egui::{self, Align, Color32, Layout, RichText, Stroke, TextStyle};

use crate::{
    app::RgStudio,
    options::{Category, OPTIONS, OptionKind},
    preview::PreviewResult,
    regex_builder::{Atom, Quantifier, RegexBuilder, RegexPart},
    theme::{self, Palette},
};

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
    render_command(app, ui, palette);
    search_results_window(app, &context, palette);
    about_window(app, &context, palette);
    regex_builder_window(app, &context, palette);
}

fn themed_text_edit(edit: egui::TextEdit, palette: Palette) -> egui::TextEdit {
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
                                            highlighted_job(text.as_str(), spans, palette);
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

fn highlighted_job(
    text: &str,
    matches: &[Range<usize>],
    palette: Palette,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.keep_trailing_whitespace = true;
    let normal = egui::TextFormat {
        font_id: egui::FontId::monospace(14.0),
        color: palette.text,
        ..Default::default()
    };
    let highlighted = egui::TextFormat {
        font_id: egui::FontId::monospace(14.0),
        color: palette.match_text,
        background: palette.match_background,
        ..Default::default()
    };
    let zero_width = egui::TextFormat {
        font_id: egui::FontId::monospace(14.0),
        color: palette.accent,
        background: palette.match_background,
        ..Default::default()
    };
    let mut cursor = 0;

    for matched in matches {
        if cursor < matched.start {
            job.append(&text[cursor..matched.start], 0.0, normal.clone());
        }
        if matched.start == matched.end {
            job.append("|", 0.0, zero_width.clone());
        } else {
            job.append(&text[matched.start..matched.end], 0.0, highlighted.clone());
        }
        cursor = matched.end;
    }

    if cursor < text.len() {
        job.append(&text[cursor..], 0.0, normal);
    }

    job
}

fn render_command(app: &mut RgStudio, ui: &mut egui::Ui, palette: Palette) {
    let command = app.command.command();
    let command_error = app.command.arguments().err();
    let mut command_text = command
        .as_deref()
        .unwrap_or("Cannot build command: fix Advanced arguments.")
        .to_owned();
    let mut run_search = false;
    egui::Frame::group(ui.style())
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.border))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Generated command");
                ui.label(
                    RichText::new(
                        command_error
                            .as_deref()
                            .unwrap_or("Ready to paste into your terminal"),
                    )
                    .small()
                    .color(if command_error.is_some() {
                        palette.danger
                    } else {
                        palette.muted_text
                    }),
                );
            });
            ui.horizontal(|ui| {
                let input_width = (ui.available_width() - 300.0).max(120.0);
                ui.add_sized(
                    egui::vec2(input_width, 36.0),
                    themed_text_edit(egui::TextEdit::singleline(&mut command_text), palette)
                        .font(TextStyle::Monospace)
                        .interactive(false),
                );
                let run_label = if app.search.running {
                    "Searching..."
                } else if command_error.is_some() {
                    "Fix command first"
                } else {
                    "Run ripgrep"
                };
                run_search = ui
                    .add_enabled(
                        !app.search.running && command_error.is_none(),
                        egui::Button::new(RichText::new(run_label).color(palette.accent_text))
                            .fill(palette.accent)
                            .min_size(egui::vec2(132.0, 36.0)),
                    )
                    .clicked();

                let label = if app.copied { "Copied" } else { "Copy command" };
                let button_fill = if app.copied {
                    palette.success
                } else {
                    palette.raised_surface
                };
                let button_text = if app.copied {
                    palette.accent_text
                } else {
                    palette.text
                };
                if ui
                    .add_enabled(
                        command_error.is_none(),
                        egui::Button::new(RichText::new(label).color(button_text))
                            .fill(button_fill)
                            .stroke(Stroke::new(1.0, palette.border))
                            .min_size(egui::vec2(142.0, 36.0)),
                    )
                    .clicked()
                {
                    if let Ok(command) = &command {
                        ui.ctx().copy_text(command.clone());
                        app.copied = true;
                    }
                }
            });
        });

    if run_search {
        app.search.start(app.command.clone());
        ui.ctx().request_repaint();
    }
}

fn search_results_window(app: &mut RgStudio, context: &egui::Context, palette: Palette) {
    let crate::search::SearchController {
        running,
        show_results,
        output,
        error,
        output_page,
        output_stream,
        ..
    } = &mut app.search;

    if !*show_results {
        return;
    }

    egui::Window::new("Ripgrep results")
        .open(show_results)
        .default_width(900.0)
        .default_height(560.0)
        .resizable(true)
        .show(context, |ui| {
            if *running {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Searching...");
                });
            }

            if let Some(error) = error.as_deref() {
                ui.colored_label(palette.danger, error);
            }

            if let Some(result) = output.as_ref() {
                let (status, status_detail, color) = if result.success {
                    (
                        "Search completed",
                        "ripgrep finished successfully.",
                        palette.success,
                    )
                } else {
                    match result.exit_code {
                        Some(1) => (
                            "No matches found",
                            "Exit code 1 is normal when ripgrep completes a search without matches.",
                            palette.muted_text,
                        ),
                        Some(_) => (
                            "ripgrep reported an error",
                            "Open Diagnostics to see ripgrep's error messages.",
                            palette.danger,
                        ),
                        None => (
                            "ripgrep ended without an exit code",
                            "The process may have been interrupted; check Diagnostics for details.",
                            palette.danger,
                        ),
                    }
                };
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(color, RichText::new(status).strong());
                    ui.separator();
                    ui.label(match result.exit_code {
                        Some(code) => format!("Exit code: {code}"),
                        None => "Exit code: unavailable".to_owned(),
                    });
                    ui.separator();
                    ui.label(format!("Elapsed: {:.3} s", result.elapsed.as_secs_f64()));
                });
                ui.label(
                    RichText::new(status_detail)
                        .small()
                        .color(palette.muted_text),
                );
                if !result.success
                    && result.exit_code != Some(1)
                    && let Some(explanation) = diagnostic_explanation(&result.stderr)
                {
                    egui::Frame::group(ui.style())
                        .fill(palette.raised_surface)
                        .stroke(Stroke::new(1.0, palette.warning))
                        .show(ui, |ui| {
                            ui.strong("What this means");
                            ui.label(explanation);
                        });
                }
                ui.label(
                    RichText::new(format!("Executable: {}", result.executable))
                        .small()
                        .color(palette.muted_text),
                );

                egui::CollapsingHeader::new("Command used")
                    .id_salt("ripgrep_command")
                    .show(ui, |ui| {
                        egui::ScrollArea::horizontal()
                            .max_height(36.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&result.command).monospace(),
                                    )
                                    .selectable(true)
                                    .wrap_mode(egui::TextWrapMode::Extend),
                                );
                            });
                    });
                ui.separator();

                ui.horizontal_wrapped(|ui| {
                    let output_tab = ui
                        .selectable_label(
                            *output_stream == crate::search::OutputStream::Stdout,
                            format!("Results ({})", output_line_count_label(&result.stdout)),
                        )
                        .on_hover_text(
                            "Matches, file lists, or other output selected by ripgrep options.",
                        );
                    if output_tab.clicked() {
                        *output_stream = crate::search::OutputStream::Stdout;
                        *output_page = 0;
                    }

                    let diagnostics_tab = ui
                        .selectable_label(
                            *output_stream == crate::search::OutputStream::Stderr,
                            format!("Messages ({})", output_line_count_label(&result.stderr)),
                        )
                        .on_hover_text("Warnings and errors reported by ripgrep.");
                    if diagnostics_tab.clicked() {
                        *output_stream = crate::search::OutputStream::Stderr;
                        *output_page = 0;
                    }
                });

                let (text, pages, text_color) = match *output_stream {
                    crate::search::OutputStream::Stdout => (
                        &result.stdout,
                        &result.stdout_pages,
                        palette.text,
                    ),
                    crate::search::OutputStream::Stderr => (
                        &result.stderr,
                        &result.stderr_pages,
                        palette.warning,
                    ),
                };

                if pages.is_empty() {
                    let empty_message = match *output_stream {
                        crate::search::OutputStream::Stdout if result.exit_code == Some(1) => {
                            "No matching lines or files were found."
                        }
                        crate::search::OutputStream::Stdout if result.success => {
                            "ripgrep wrote no output. This can be expected for quiet output modes."
                        }
                        crate::search::OutputStream::Stdout => {
                            "No result output was written. Check Diagnostics for details."
                        }
                        crate::search::OutputStream::Stderr if result.success => {
                            "No warnings or errors were reported."
                        }
                        crate::search::OutputStream::Stderr => {
                            "No detailed message was returned."
                        }
                    };
                    ui.label(RichText::new(empty_message).color(palette.muted_text));
                } else if *output_stream == crate::search::OutputStream::Stdout {
                    if let Some(readable_results) = result.readable_results.as_ref() {
                        render_readable_results(ui, readable_results, palette);
                        egui::CollapsingHeader::new("Raw ripgrep output")
                            .id_salt("raw_ripgrep_output")
                            .show(ui, |ui| {
                                render_paged_output(
                                    ui,
                                    text,
                                    pages,
                                    output_page,
                                    text_color,
                                    *output_stream,
                                );
                            });
                    } else {
                        ui.label(
                            RichText::new(
                                "This output uses a special format, so the original text is shown.",
                            )
                            .small()
                            .color(palette.muted_text),
                        );
                        render_paged_output(
                            ui,
                            text,
                            pages,
                            output_page,
                            text_color,
                            *output_stream,
                        );
                    }
                } else {
                    render_paged_output(
                        ui,
                        text,
                        pages,
                        output_page,
                        text_color,
                        *output_stream,
                    );
                }
            } else if error.is_none() && !*running {
                ui.label(
                    RichText::new("Run a search to see its output.").color(palette.muted_text),
                );
            }
        });
}

fn output_line_count_label(output: &str) -> String {
    let count = output.lines().count();
    format!("{count} {}", if count == 1 { "line" } else { "lines" })
}

fn render_readable_results(
    ui: &mut egui::Ui,
    results: &crate::search::ReadableResults,
    palette: Palette,
) {
    if results.truncated {
        ui.strong(format!(
            "Showing the first {} of {} result lines from at least {} files.",
            results.lines.len(),
            results.total_lines,
            results.file_count
        ));
    } else {
        let file_label = if results.file_count == 1 {
            "file"
        } else {
            "files"
        };
        ui.strong(format!(
            "{} result lines in {} {file_label}.",
            results.total_lines, results.file_count
        ));
    }
    ui.label(
        RichText::new(
            "Each card names the file and shows the line containing a result. Long lines are shortened.",
        )
        .small()
        .color(palette.muted_text),
    );
    if results
        .lines
        .iter()
        .any(|line| line.path.to_ascii_lowercase().ends_with(".d"))
    {
        ui.label(
            RichText::new(
                "A .d file is a build dependency list; it can contain many paths on one long line.",
            )
            .small()
            .color(palette.muted_text),
        );
    }

    egui::ScrollArea::vertical()
        .max_height(320.0)
        .min_scrolled_height(0.0)
        .show(ui, |ui| {
            for (index, line) in results.lines.iter().enumerate() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(format!("Result {}", index + 1));
                        ui.add(
                            egui::Label::new(
                                RichText::new(&line.path).monospace().color(palette.accent),
                            )
                            .wrap(),
                        );
                        if let Some(line_number) = line.line_number {
                            ui.label(format!("Line {line_number}"));
                        }
                    });

                    let matches = line.match_range.clone().into_iter().collect::<Vec<_>>();
                    ui.add(
                        egui::Label::new(highlighted_job(&line.preview, &matches, palette)).wrap(),
                    );
                    if line.shortened {
                        ui.label(
                            RichText::new(
                                "Line shortened; expand Raw ripgrep output for the full text.",
                            )
                            .small()
                            .color(palette.muted_text),
                        );
                    }
                });
            }
        });
}

fn render_paged_output(
    ui: &mut egui::Ui,
    text: &str,
    pages: &[Range<usize>],
    page: &mut usize,
    text_color: egui::Color32,
    stream: crate::search::OutputStream,
) {
    *page = (*page).min(pages.len() - 1);
    let current_page = *page;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(current_page > 0, egui::Button::new("Previous page"))
            .clicked()
        {
            *page -= 1;
        }
        ui.label(format!("Page {} of {}", current_page + 1, pages.len()));
        if ui
            .add_enabled(
                current_page + 1 < pages.len(),
                egui::Button::new("Next page"),
            )
            .clicked()
        {
            *page += 1;
        }
    });

    let range = &pages[current_page];
    egui::ScrollArea::both()
        .id_salt(("ripgrep_output_page", stream, current_page))
        .max_height(360.0)
        .min_scrolled_height(0.0)
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(&text[range.clone()])
                        .monospace()
                        .color(text_color),
                )
                .selectable(true)
                .wrap_mode(egui::TextWrapMode::Extend),
            );
        });
}

fn diagnostic_explanation(stderr: &str) -> Option<&'static str> {
    if stderr.is_empty() {
        None
    } else if stderr.contains("repetition operator missing expression") {
        Some(
            "`*` repeats the item before it, but this pattern starts with `*`. Use `.*` to match any characters in a line. For file names, use Files > Include/exclude glob, for example `*.rs`.",
        )
    } else if stderr.contains("regex parse error:") {
        Some(
            "The search pattern is a regular expression and ripgrep could not parse it. Enable Literal strings for exact text, or use a glob option in Files to filter file names.",
        )
    } else if stderr.contains("PCRE2 is not available") {
        Some(
            "This ripgrep executable was built without PCRE2 support. Use the default regex engine or install a PCRE2-enabled ripgrep build.",
        )
    } else {
        Some(
            "ripgrep could not complete this request. Check the messages below and verify that the installed ripgrep version supports the selected options.",
        )
    }
}

fn about_window(app: &mut RgStudio, context: &egui::Context, palette: Palette) {
    egui::Window::new("About rg studio")
        .open(&mut app.show_about)
        .resizable(false)
        .show(context, |ui| {
            ui.heading("rg studio");
            ui.label("Visual command composer for ripgrep");
            ui.separator();
            egui::Grid::new("build_info")
                .spacing([18.0, 8.0])
                .show(ui, |ui| {
                    for (label, value) in [
                        ("Version", env!("CARGO_PKG_VERSION")),
                        ("Commit", option_env!("VERGEN_GIT_SHA").unwrap_or("unknown")),
                        (
                            "Built",
                            option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown"),
                        ),
                        (
                            "Rust",
                            option_env!("VERGEN_RUSTC_SEMVER").unwrap_or("unknown"),
                        ),
                        (
                            "Target",
                            option_env!("VERGEN_CARGO_TARGET_TRIPLE").unwrap_or("unknown"),
                        ),
                    ] {
                        ui.strong(label);
                        ui.monospace(RichText::new(value).color(palette.muted_text));
                        ui.end_row();
                    }
                });
        });
}

fn regex_builder_window(app: &mut RgStudio, context: &egui::Context, palette: Palette) {
    if !app.show_regex_builder {
        return;
    }

    let mut open = app.show_regex_builder;
    let mut remove = None;
    let mut move_up = None;
    let mut move_down = None;
    egui::Window::new("Regex builder")
        .open(&mut open)
        .default_width(860.0)
        .min_width(600.0)
        .resizable(true)
        .show(context, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut app.regex_builder.start_anchor, "Start of line ^");
                ui.checkbox(&mut app.regex_builder.end_anchor, "End of line $");
                ui.separator();
                if ui.button("Clear").clicked() {
                    app.regex_builder.parts.clear();
                }
            });

            ui.horizontal_wrapped(|ui| {
                ui.strong("Add atom");
                for (label, atom) in [
                    ("Literal", Atom::Literal),
                    ("Raw", Atom::Raw),
                    ("Any", Atom::AnyCharacter),
                    ("Digit", Atom::Digit),
                    ("Word", Atom::WordCharacter),
                    ("Space", Atom::Whitespace),
                    ("Class", Atom::CharacterClass),
                    ("Not class", Atom::NegatedClass),
                    ("Group", Atom::CaptureGroup),
                    ("No capture", Atom::NonCaptureGroup),
                    ("Boundary", Atom::WordBoundary),
                    ("OR", Atom::Alternation),
                ] {
                    if ui.button(label).clicked() {
                        app.regex_builder.parts.push(RegexPart {
                            atom,
                            ..RegexPart::default()
                        });
                    }
                }
            });

            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    for (index, part) in app.regex_builder.parts.iter_mut().enumerate() {
                        ui.push_id(index, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(format!("{}.", index + 1));
                                egui::ComboBox::from_id_salt(("atom", index))
                                    .selected_text(part.atom.title())
                                    .width(148.0)
                                    .show_ui(ui, |ui| {
                                        for atom in Atom::ALL {
                                            ui.selectable_value(&mut part.atom, atom, atom.title());
                                        }
                                    });

                                if part.atom.accepts_value() {
                                    ui.add_sized(
                                        egui::vec2(190.0, 30.0),
                                        themed_text_edit(
                                            egui::TextEdit::singleline(&mut part.value),
                                            palette,
                                        )
                                        .hint_text("content"),
                                    );
                                } else {
                                    ui.add_space(198.0);
                                }

                                if part.atom.accepts_quantifier() {
                                    egui::ComboBox::from_id_salt(("quantifier", index))
                                        .selected_text(part.quantifier.title())
                                        .width(92.0)
                                        .show_ui(ui, |ui| {
                                            for quantifier in Quantifier::ALL {
                                                ui.selectable_value(
                                                    &mut part.quantifier,
                                                    quantifier,
                                                    quantifier.title(),
                                                );
                                            }
                                        });
                                    match part.quantifier {
                                        Quantifier::Exactly => {
                                            ui.add(
                                                egui::DragValue::new(&mut part.minimum)
                                                    .range(0..=9999),
                                            );
                                        }
                                        Quantifier::Range => {
                                            ui.add(
                                                egui::DragValue::new(&mut part.minimum)
                                                    .range(0..=9999),
                                            );
                                            // Keep the upper bound valid even if the lower
                                            // bound was just raised above it, otherwise the
                                            // generated pattern (e.g. `{5,3}`) fails to compile.
                                            part.maximum = part.maximum.max(part.minimum);
                                            ui.label("to");
                                            ui.add(
                                                egui::DragValue::new(&mut part.maximum)
                                                    .range(part.minimum..=9999),
                                            );
                                        }
                                        _ => {}
                                    }
                                } else {
                                    ui.label(
                                        RichText::new("No repetition")
                                            .small()
                                            .color(palette.muted_text),
                                    );
                                }

                                if ui.small_button("Up").clicked() {
                                    move_up = Some(index);
                                }
                                if ui.small_button("Down").clicked() {
                                    move_down = Some(index);
                                }
                                if ui.small_button("Remove").clicked() {
                                    remove = Some(index);
                                }
                            });
                        });
                    }
                });

            ui.separator();
            let pattern = app.regex_builder.pattern();
            ui.label("Generated regex");
            ui.add_sized(
                egui::vec2(ui.available_width(), 34.0),
                themed_text_edit(egui::TextEdit::singleline(&mut pattern.as_str()), palette)
                    .font(TextStyle::Monospace)
                    .interactive(false),
            );
            if let Some(error) = app.regex_builder.validation_error() {
                ui.colored_label(palette.danger, error);
            } else {
                ui.colored_label(palette.success, "Valid regex");
            }
        });

    if let Some(index) = remove {
        app.regex_builder.parts.remove(index);
    } else if let Some(index) = move_up
        && index > 0
    {
        app.regex_builder.parts.swap(index, index - 1);
    } else if let Some(index) = move_down
        && index + 1 < app.regex_builder.parts.len()
    {
        app.regex_builder.parts.swap(index, index + 1);
    }

    app.show_regex_builder = open;
    let pattern = app.regex_builder.pattern();
    if app.command.pattern != pattern {
        app.copied = false;
    }
    app.command.pattern = pattern;
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Context, Pos2, RawInput, Rect, vec2};

    use crate::{app::RgStudio, theme};

    use super::{diagnostic_explanation, highlighted_job, output_line_count_label, render};

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
    fn output_line_count_excludes_the_trailing_newline() {
        assert_eq!(output_line_count_label("one\n"), "1 line");
        assert_eq!(output_line_count_label("one\ntwo\n"), "2 lines");
        assert_eq!(output_line_count_label(""), "0 lines");
    }

    #[test]
    fn highlighted_preview_preserves_spaces_around_matches() {
        let text = "  before  match  after  \nnext ";
        let start = text.find("match").unwrap();
        let job = highlighted_job(
            text,
            &[start..start + "match".len()],
            theme::Palette::new(true),
        );

        assert_eq!(job.text, text);
        assert!(job.keep_trailing_whitespace);
    }

    #[test]
    fn explains_regex_quantifier_errors_as_pattern_or_file_glob_issues() {
        let explanation = diagnostic_explanation(
            "rg: regex parse error:\n    (?:*.)\n       ^\nerror: repetition operator missing expression",
        )
        .unwrap();

        assert!(explanation.contains("`.*`"));
        assert!(explanation.contains("`*.rs`"));
    }

    #[test]
    fn explains_generic_regex_parse_errors() {
        let explanation = diagnostic_explanation("regex parse error: invalid group").unwrap();
        assert!(explanation.contains("regular expression"));
        assert!(explanation.contains("Literal strings"));
    }
}
