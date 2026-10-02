use std::ops::Range;

use eframe::egui::{self, RichText, Stroke, TextStyle};

use crate::{app::RgStudio, theme::Palette};

use super::themed_text_edit;

pub(super) fn highlighted_job(
    text: &str,
    matches: &[Range<usize>],
    palette: Palette,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob {
        keep_trailing_whitespace: true,
        ..Default::default()
    };
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

pub(super) fn render_command(app: &mut RgStudio, ui: &mut egui::Ui, palette: Palette) {
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
                    && let Ok(command) = &command
                {
                    ui.ctx().copy_text(command.clone());
                    app.copied = true;
                }
            });
        });

    if run_search {
        app.search.start(app.command.clone());
        ui.ctx().request_repaint();
    }
}

pub(super) fn search_results_window(app: &mut RgStudio, context: &egui::Context, palette: Palette) {
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

pub(super) fn raw_page_match_ranges(
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

pub(super) fn render_paged_output(
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

pub(super) fn ansi_layout_job(text: &str, base_color: egui::Color32) -> egui::text::LayoutJob {
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
