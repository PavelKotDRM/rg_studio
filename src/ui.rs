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
        result_id,
        stdout_page,
        stderr_page,
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
            let content_width = ui.available_width();
            egui::ScrollArea::vertical()
                .max_width(content_width)
                .show(ui, |ui| {
                    ui.set_width(content_width);
                if *running {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Searching...");
                    });
                }

                if let Some(error) = error.as_deref() {
                    egui::Frame::group(ui.style())
                        .stroke(Stroke::new(1.0, palette.danger))
                        .show(ui, |ui| {
                            ui.colored_label(
                                palette.danger,
                                RichText::new("Unable to start the search").strong(),
                            );
                            ui.label(error);
                        });
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
                                "Select stderr in CLI output to inspect ripgrep's error details.",
                                palette.danger,
                            ),
                            None => (
                                "ripgrep ended without an exit code",
                                "The process may have been interrupted.",
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
                    ui.label(
                        RichText::new(format!("Executable: {}", result.executable))
                            .small()
                            .color(palette.muted_text),
                    );

                    egui::CollapsingHeader::new("Command used")
                        .id_salt(("ripgrep_command", *result_id))
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
                    render_cli_output(
                            ui,
                            result,
                            palette,
                            *result_id,
                            output_stream,
                            stdout_page,
                            stderr_page,
                        );
                } else if error.is_none() && !*running {
                    ui.label(
                        RichText::new("Run a search to see its output.").color(palette.muted_text),
                    );
                }
                });
        });
}

fn render_cli_output(
    ui: &mut egui::Ui,
    result: &crate::search::SearchOutput,
    palette: Palette,
    result_id: u64,
    stream: &mut crate::search::OutputStream,
    stdout_page: &mut usize,
    stderr_page: &mut usize,
) {
    ui.heading("CLI output");
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(
            stream,
            crate::search::OutputStream::Stdout,
            format!("stdout · {} pages", result.stdout_pages.len()),
        );
        ui.selectable_value(
            stream,
            crate::search::OutputStream::Stderr,
            format!("stderr · {} pages", result.stderr_pages.len()),
        );
    });
    match *stream {
        crate::search::OutputStream::Stdout => render_raw_stream(
            ui,
            result,
            crate::search::OutputStream::Stdout,
            stdout_page,
            result_id,
            palette,
        ),
        crate::search::OutputStream::Stderr => render_raw_stream(
            ui,
            result,
            crate::search::OutputStream::Stderr,
            stderr_page,
            result_id,
            palette,
        ),
    }
}

fn render_raw_stream(
    ui: &mut egui::Ui,
    result: &crate::search::SearchOutput,
    stream: crate::search::OutputStream,
    page: &mut usize,
    result_id: u64,
    palette: Palette,
) {
    let (source, text, is_utf8, pages, text_color) = match stream {
        crate::search::OutputStream::Stdout => (
            "stdout",
            &result.stdout,
            result.stdout_is_utf8,
            &result.stdout_pages,
            palette.text,
        ),
        crate::search::OutputStream::Stderr => (
            "stderr",
            &result.stderr,
            result.stderr_is_utf8,
            &result.stderr_pages,
            palette.warning,
        ),
    };
    *page = (*page).min(pages.len().saturating_sub(1));
    ui.horizontal(|ui| {
        ui.strong(source);
        if let Some(range) = pages.get(*page)
            && ui.button(format!("Copy {source} page")).clicked()
        {
            ui.ctx().copy_text(text[range.clone()].to_owned());
        }
    });
    if !is_utf8 {
        ui.weak(
            RichText::new(
                "Non-UTF-8 bytes use reversible \\xNN escapes; literal backslashes are doubled.",
            )
            .small(),
        );
    }
    if pages.is_empty() {
        ui.weak("(empty)");
    } else {
        let viewer_id = ui.make_persistent_id(("raw_stream", result_id, stream));
        let match_ranges = if stream == crate::search::OutputStream::Stdout {
            raw_page_match_ranges(
                text,
                pages[*page].clone(),
                result.raw_highlight_matcher.as_ref(),
            )
        } else {
            Vec::new()
        };
        render_paged_output(ui, text, pages, page, text_color, viewer_id, &match_ranges);
    }
}

fn raw_page_match_ranges(
    text: &str,
    page: Range<usize>,
    matcher: Option<&regex::Regex>,
) -> Vec<Range<usize>> {
    let Some(page_text) = text.get(page) else {
        return Vec::new();
    };
    if page_text.contains("\x1b[") {
        return Vec::new();
    }

    matcher.map_or_else(Vec::new, |matcher| {
        matcher
            .find_iter(page_text)
            .map(|matched| matched.range())
            .collect()
    })
}

fn render_paged_output(
    ui: &mut egui::Ui,
    text: &str,
    pages: &[Range<usize>],
    page: &mut usize,
    text_color: egui::Color32,
    viewer_id: egui::Id,
    match_ranges: &[Range<usize>],
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
    let page_text = &text[range.clone()];
    let mut job = if match_ranges.is_empty() {
        ansi_layout_job(page_text, text_color)
    } else {
        highlighted_job(
            page_text,
            match_ranges,
            Palette::new(ui.visuals().dark_mode),
        )
    };
    let page_width = ui.available_width();
    job.wrap.max_width = page_width;
    egui::ScrollArea::vertical()
        .id_salt(("ripgrep_output_page", viewer_id, current_page))
        .max_width(page_width)
        .max_height(360.0)
        .min_scrolled_height(0.0)
        .show(ui, |ui| {
            ui.set_width(page_width);
            ui.add(
                egui::Label::new(job)
                    .selectable(true)
                    .wrap_mode(egui::TextWrapMode::Wrap),
            );
        });
}

#[derive(Clone, Copy)]
struct AnsiStyle {
    foreground: egui::Color32,
    background: egui::Color32,
}

fn ansi_layout_job(text: &str, base_color: egui::Color32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob {
        keep_trailing_whitespace: true,
        ..Default::default()
    };
    let mut style = AnsiStyle {
        foreground: base_color,
        background: egui::Color32::TRANSPARENT,
    };
    let mut cursor = 0;

    while let Some(relative_start) = text[cursor..].find("\x1b[") {
        let start = cursor + relative_start;
        append_ansi_text(&mut job, &text[cursor..start], style);

        let parameters_start = start + 2;
        let Some(relative_end) = text[parameters_start..].find('m') else {
            append_ansi_text(&mut job, &text[start..], style);
            return job;
        };
        let end = parameters_start + relative_end;
        apply_ansi_sgr(&mut style, &text[parameters_start..end], base_color);
        cursor = end + 1;
    }

    append_ansi_text(&mut job, &text[cursor..], style);
    job
}

fn append_ansi_text(job: &mut egui::text::LayoutJob, text: &str, style: AnsiStyle) {
    if text.is_empty() {
        return;
    }

    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::monospace(14.0),
            color: style.foreground,
            background: style.background,
            ..Default::default()
        },
    );
}

fn apply_ansi_sgr(style: &mut AnsiStyle, parameters: &str, base_color: egui::Color32) {
    let codes = if parameters.is_empty() {
        vec![0]
    } else {
        parameters
            .split(';')
            .map(|parameter| {
                if parameter.is_empty() {
                    0
                } else {
                    parameter.parse::<i32>().unwrap_or(-1)
                }
            })
            .collect()
    };
    let mut index = 0;

    while index < codes.len() {
        match codes[index] {
            0 => {
                style.foreground = base_color;
                style.background = egui::Color32::TRANSPARENT;
            }
            30..=37 => style.foreground = ansi_color((codes[index] - 30) as u8),
            39 => style.foreground = base_color,
            40..=47 => style.background = ansi_color((codes[index] - 40) as u8),
            49 => style.background = egui::Color32::TRANSPARENT,
            90..=97 => style.foreground = ansi_color((codes[index] - 90 + 8) as u8),
            100..=107 => style.background = ansi_color((codes[index] - 100 + 8) as u8),
            38 | 48 if index + 2 < codes.len() && codes[index + 1] == 5 => {
                let color = ansi_color(codes[index + 2].clamp(0, 255) as u8);
                if codes[index] == 38 {
                    style.foreground = color;
                } else {
                    style.background = color;
                }
                index += 2;
            }
            38 | 48 if index + 4 < codes.len() && codes[index + 1] == 2 => {
                let color = egui::Color32::from_rgb(
                    codes[index + 2].clamp(0, 255) as u8,
                    codes[index + 3].clamp(0, 255) as u8,
                    codes[index + 4].clamp(0, 255) as u8,
                );
                if codes[index] == 38 {
                    style.foreground = color;
                } else {
                    style.background = color;
                }
                index += 4;
            }
            _ => {}
        }
        index += 1;
    }
}

fn ansi_color(index: u8) -> egui::Color32 {
    const BASIC_COLORS: [egui::Color32; 16] = [
        egui::Color32::from_rgb(0, 0, 0),
        egui::Color32::from_rgb(205, 49, 49),
        egui::Color32::from_rgb(13, 188, 121),
        egui::Color32::from_rgb(229, 229, 16),
        egui::Color32::from_rgb(36, 114, 200),
        egui::Color32::from_rgb(188, 63, 188),
        egui::Color32::from_rgb(17, 168, 205),
        egui::Color32::from_rgb(229, 229, 229),
        egui::Color32::from_rgb(102, 102, 102),
        egui::Color32::from_rgb(241, 76, 76),
        egui::Color32::from_rgb(35, 209, 139),
        egui::Color32::from_rgb(245, 245, 67),
        egui::Color32::from_rgb(59, 142, 234),
        egui::Color32::from_rgb(214, 112, 214),
        egui::Color32::from_rgb(41, 184, 219),
        egui::Color32::from_rgb(255, 255, 255),
    ];
    if index < 16 {
        return BASIC_COLORS[index as usize];
    }
    if index < 232 {
        let color_level = |component| match component {
            0 => 0,
            1 => 95,
            2 => 135,
            3 => 175,
            4 => 215,
            _ => 255,
        };
        let color_index = index - 16;
        return egui::Color32::from_rgb(
            color_level(color_index / 36),
            color_level((color_index % 36) / 6),
            color_level(color_index % 6),
        );
    }
    let gray = 8 + (index - 232) * 10;
    egui::Color32::from_gray(gray)
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
    use eframe::egui::{Color32, Context, Pos2, RawInput, Rect, vec2};

    use crate::{app::RgStudio, theme};

    use super::{
        ansi_layout_job, highlighted_job, raw_page_match_ranges, render, render_paged_output,
    };

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
        let job = highlighted_job(
            text,
            &[start..start + "match".len()],
            theme::Palette::new(true),
        );

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
