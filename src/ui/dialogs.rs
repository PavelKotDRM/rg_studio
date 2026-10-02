use eframe::egui::{self, RichText, TextStyle};

use crate::{
    app::RgStudio,
    regex_builder::{Atom, Quantifier, RegexPart},
    theme::Palette,
};

use super::themed_text_edit;

pub(super) fn about_window(app: &mut RgStudio, context: &egui::Context, palette: Palette) {
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

pub(super) fn regex_builder_window(app: &mut RgStudio, context: &egui::Context, palette: Palette) {
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
