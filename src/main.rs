mod command;
mod options;
mod regex_builder;

use command::CommandState;
use eframe::egui;
use options::{Category, OPTIONS, OptionKind};
use regex_builder::{Atom, Quantifier, RegexBuilder, RegexPart};

#[derive(Debug, Default)]
struct OptionState {
    enabled: bool,
    value: String,
}

struct RgStudio {
    command: CommandState,
    option_states: Vec<OptionState>,
    category: Category,
    filter: String,
    show_about: bool,
    show_regex_builder: bool,
    regex_builder: RegexBuilder,
    dark_theme: bool,
    copied: bool,
}

impl Default for RgStudio {
    fn default() -> Self {
        Self {
            command: CommandState::default(),
            option_states: (0..OPTIONS.len()).map(|_| OptionState::default()).collect(),
            category: Category::Search,
            filter: String::new(),
            show_about: false,
            show_regex_builder: false,
            regex_builder: RegexBuilder::default(),
            dark_theme: true,
            copied: false,
        }
    }
}

impl RgStudio {
    fn rebuild_options(&mut self) {
        self.command.options = OPTIONS
            .iter()
            .zip(&self.option_states)
            .filter_map(|(spec, state)| {
                if !state.enabled {
                    return None;
                }
                match spec.kind {
                    OptionKind::Switch => Some(spec.flag.to_owned()),
                    OptionKind::Value(_) if !state.value.is_empty() => {
                        Some(format!("{}={}", spec.flag, state.value))
                    }
                    OptionKind::Value(_) => None,
                }
            })
            .collect();
    }

    fn option_catalog(&mut self, ui: &mut egui::Ui) {
        let filter = self.filter.trim().to_ascii_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("option_grid")
                .num_columns(3)
                .spacing([12.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    for (spec, state) in OPTIONS.iter().zip(&mut self.option_states) {
                        if spec.category != self.category
                            || (!filter.is_empty()
                                && !spec.label.to_ascii_lowercase().contains(&filter)
                                && !spec.flag.contains(&filter))
                        {
                            continue;
                        }

                        ui.checkbox(&mut state.enabled, spec.label);
                        ui.monospace(spec.flag);
                        match spec.kind {
                            OptionKind::Switch => {
                                ui.label("");
                            }
                            OptionKind::Value(hint) => {
                                ui.add_enabled(
                                    state.enabled,
                                    egui::TextEdit::singleline(&mut state.value)
                                        .hint_text(hint)
                                        .desired_width(220.0),
                                );
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn about_window(&mut self, context: &egui::Context) {
        egui::Window::new("About rg studio")
            .open(&mut self.show_about)
            .resizable(false)
            .show(context, |ui| {
                ui.heading("rg studio");
                ui.label("Visual command composer for ripgrep");
                ui.separator();
                egui::Grid::new("build_info")
                    .spacing([18.0, 7.0])
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
                            ui.monospace(value);
                            ui.end_row();
                        }
                    });
            });
    }

    fn regex_builder_window(&mut self, context: &egui::Context) {
        if !self.show_regex_builder {
            return;
        }

        let mut open = self.show_regex_builder;
        let mut remove = None;
        let mut move_up = None;
        let mut move_down = None;
        egui::Window::new("Regex builder")
            .open(&mut open)
            .default_width(860.0)
            .min_width(700.0)
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.regex_builder.start_anchor, "Start of line ^");
                    ui.checkbox(&mut self.regex_builder.end_anchor, "End of line $");
                    ui.separator();
                    if ui.button("Clear").clicked() {
                        self.regex_builder.parts.clear();
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    ui.strong("Add");
                    for (label, atom) in [
                        ("Literal", Atom::Literal),
                        ("Any", Atom::AnyCharacter),
                        ("Digit", Atom::Digit),
                        ("Word", Atom::WordCharacter),
                        ("Space", Atom::Whitespace),
                        ("Class", Atom::CharacterClass),
                        ("Group", Atom::CaptureGroup),
                        ("Raw", Atom::Raw),
                        ("OR", Atom::Alternation),
                    ] {
                        if ui.button(label).clicked() {
                            self.regex_builder.parts.push(RegexPart {
                                atom,
                                ..RegexPart::default()
                            });
                        }
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .show(ui, |ui| {
                        for (index, part) in self.regex_builder.parts.iter_mut().enumerate() {
                            ui.push_id(index, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}.", index + 1));
                                    egui::ComboBox::from_id_salt("atom")
                                        .selected_text(part.atom.title())
                                        .width(150.0)
                                        .show_ui(ui, |ui| {
                                            for atom in Atom::ALL {
                                                ui.selectable_value(
                                                    &mut part.atom,
                                                    atom,
                                                    atom.title(),
                                                );
                                            }
                                        });

                                    if part.atom.accepts_value() {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut part.value)
                                                .hint_text("content")
                                                .desired_width(190.0),
                                        );
                                    } else {
                                        ui.add_space(198.0);
                                    }

                                    egui::ComboBox::from_id_salt("quantifier")
                                        .selected_text(part.quantifier.title())
                                        .width(88.0)
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
                                            ui.label("to");
                                            ui.add(
                                                egui::DragValue::new(&mut part.maximum)
                                                    .range(part.minimum..=9999),
                                            );
                                        }
                                        _ => {}
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
                let pattern = self.regex_builder.pattern();
                ui.label("Preview");
                ui.add(
                    egui::TextEdit::singleline(&mut pattern.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
                if let Some(error) = self.regex_builder.validation_error() {
                    ui.colored_label(egui::Color32::from_rgb(230, 90, 90), error);
                } else {
                    ui.colored_label(egui::Color32::from_rgb(80, 190, 120), "Valid regex");
                }
            });

        if let Some(index) = remove {
            self.regex_builder.parts.remove(index);
        } else if let Some(index) = move_up
            && index > 0
        {
            self.regex_builder.parts.swap(index, index - 1);
        } else if let Some(index) = move_down
            && index + 1 < self.regex_builder.parts.len()
        {
            self.regex_builder.parts.swap(index, index + 1);
        }

        self.show_regex_builder = open;
        if open {
            self.command.pattern = self.regex_builder.pattern();
        }
    }
}

impl eframe::App for RgStudio {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
        ui.style_mut().spacing.item_spacing = egui::vec2(10.0, 8.0);

        ui.horizontal(|ui| {
            ui.heading("rg studio");
            ui.separator();
            ui.label("Pattern");
            ui.add(
                egui::TextEdit::singleline(&mut self.command.pattern)
                    .hint_text("regular expression")
                    .desired_width(260.0),
            );
            if ui.button("Regex builder").clicked() {
                self.regex_builder = RegexBuilder::from_pattern(&self.command.pattern);
                self.show_regex_builder = true;
            }
            ui.label("Path");
            ui.add(
                egui::TextEdit::singleline(&mut self.command.path)
                    .hint_text(".")
                    .desired_width(220.0),
            );
        });

        ui.horizontal(|ui| {
            for category in Category::ALL {
                ui.selectable_value(&mut self.category, category, category.title());
            }
            ui.separator();
            ui.label("Filter");
            ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(150.0));
            ui.separator();
            if ui.checkbox(&mut self.dark_theme, "Dark").changed() {
                context.set_visuals(if self.dark_theme {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
            }
            if ui.button("About").clicked() {
                self.show_about = true;
            }
        });
        ui.separator();

        let catalog_height = (ui.available_height() - 112.0).max(120.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), catalog_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| self.option_catalog(ui),
        );

        self.rebuild_options();
        ui.separator();
        ui.horizontal(|ui| {
            ui.strong("Additional arguments");
            ui.add(
                egui::TextEdit::singleline(&mut self.command.extra_arguments)
                    .hint_text("repeatable or newly added rg arguments")
                    .desired_width(f32::INFINITY),
            );
        });
        ui.label("Generated command");
        let command = self.command.command();
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut command.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            );
            if ui.button("Copy to Clipboard").clicked() {
                context.copy_text(command);
                self.copied = true;
            }
            if self.copied {
                ui.label("Copied");
            }
        });

        self.about_window(&context);
        self.regex_builder_window(&context);
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([820.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "rg studio",
        options,
        Box::new(|creation_context| {
            creation_context.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::<RgStudio>::default())
        }),
    )
}
