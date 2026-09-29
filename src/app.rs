use crate::{
    command::CommandState,
    options::{Category, OPTIONS},
    preview::{self, PreviewOptions, PreviewResult},
    regex_builder::RegexBuilder,
    search::SearchController,
};

#[derive(Debug, Default)]
pub(crate) struct OptionState {
    pub(crate) enabled: bool,
    pub(crate) value: String,
}

pub(crate) struct RgStudio {
    pub(crate) command: CommandState,
    pub(crate) option_states: Vec<OptionState>,
    pub(crate) category: Category,
    pub(crate) filter: String,
    pub(crate) show_about: bool,
    pub(crate) show_regex_builder: bool,
    pub(crate) preview_open: bool,
    pub(crate) search: SearchController,
    pub(crate) regex_builder: RegexBuilder,
    pub(crate) dark_theme: bool,
    pub(crate) copied: bool,
    pub(crate) sample_text: String,
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
            preview_open: true,
            search: SearchController::default(),
            regex_builder: RegexBuilder::default(),
            dark_theme: true,
            copied: false,
            sample_text: "fn main() {\n    // TODO: add a preview pattern\n    println!(\"hello world\");\n}\n"
                .to_owned(),
        }
    }
}

impl RgStudio {
    pub(crate) fn rebuild_options(&mut self) {
        let options = OPTIONS
            .iter()
            .zip(&self.option_states)
            .filter_map(|(spec, state)| {
                if !state.enabled {
                    return None;
                }
                match spec.kind {
                    crate::options::OptionKind::Switch => Some(spec.flag.to_owned()),
                    crate::options::OptionKind::Value(_) if !state.value.is_empty() => {
                        Some(format!("{}={}", spec.flag, state.value))
                    }
                    crate::options::OptionKind::Value(_) => None,
                }
            })
            .collect::<Vec<_>>();

        if self.command.options != options {
            self.copied = false;
        }
        self.command.options = options;
    }

    pub(crate) fn preview_result(&self) -> PreviewResult {
        let mut preview_options = PreviewOptions::default();
        let mut case_mode = CaseMode::Sensitive;
        let mut unsupported_engine = false;

        for (spec, state) in crate::options::OPTIONS.iter().zip(&self.option_states) {
            if !state.enabled {
                continue;
            }

            match spec.flag {
                "--case-sensitive" => case_mode = CaseMode::Sensitive,
                "--ignore-case" => case_mode = CaseMode::Insensitive,
                "--smart-case" => case_mode = CaseMode::Smart,
                "--fixed-strings" => preview_options.fixed_strings = true,
                "--multiline" => preview_options.multi_line = true,
                "--multiline-dotall" => {
                    preview_options.multi_line = true;
                    preview_options.dot_matches_new_line = true;
                }
                "--no-unicode" => preview_options.no_unicode = true,
                "--word-regexp" => preview_options.whole_word = true,
                "--line-regexp" => preview_options.whole_line = true,
                "--pcre2" => unsupported_engine = true,
                "--engine"
                    if state.value.eq_ignore_ascii_case("pcre2")
                        || state.value.eq_ignore_ascii_case("auto") =>
                {
                    unsupported_engine = true;
                }
                _ => {}
            }
        }

        if unsupported_engine {
            return PreviewResult::UnsupportedEngine;
        }

        preview_options.case_insensitive = match case_mode {
            CaseMode::Sensitive => false,
            CaseMode::Insensitive => true,
            CaseMode::Smart => !self.command.pattern.chars().any(char::is_uppercase),
        };

        preview::evaluate(&self.command.pattern, &self.sample_text, preview_options)
    }
}

impl eframe::App for RgStudio {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        crate::ui::render(self, ui);
    }
}

#[cfg(test)]
mod tests {
    use super::RgStudio;
    use crate::{options::OPTIONS, preview::PreviewResult};

    fn enable_option(app: &mut RgStudio, flag: &str, value: Option<&str>) {
        let index = OPTIONS
            .iter()
            .position(|spec| spec.flag == flag)
            .expect("option flag should exist");
        app.option_states[index].enabled = true;
        if let Some(value) = value {
            app.option_states[index].value = value.to_owned();
        }
    }

    #[test]
    fn preview_respects_search_options_and_regex_engine() {
        let mut app = RgStudio::default();
        app.command.pattern = "todo".to_owned();
        app.sample_text = "TODO todo".to_owned();
        enable_option(&mut app, "--ignore-case", None);
        assert_eq!(
            app.preview_result(),
            PreviewResult::Matches(vec![0..4, 5..9])
        );

        enable_option(&mut app, "--engine", Some("pcre2"));
        assert_eq!(app.preview_result(), PreviewResult::UnsupportedEngine);
    }

    #[test]
    fn smart_case_only_ignores_case_for_lowercase_patterns() {
        let mut app = RgStudio::default();
        app.command.pattern = "todo".to_owned();
        app.sample_text = "TODO todo".to_owned();
        enable_option(&mut app, "--smart-case", None);
        assert_eq!(
            app.preview_result(),
            PreviewResult::Matches(vec![0..4, 5..9])
        );

        app.command.pattern = "TODO".to_owned();
        assert_eq!(
            app.preview_result(),
            PreviewResult::Matches(std::iter::once(0..4).collect())
        );
    }
}

#[derive(Clone, Copy)]
enum CaseMode {
    Sensitive,
    Insensitive,
    Smart,
}
