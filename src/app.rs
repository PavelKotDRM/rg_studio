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
    /// Disables any option that contradicts the newly enabled `flag`.
    ///
    /// ripgrep applies the last matching flag on the command line, so leaving
    /// both sides of a conflicting pair enabled (e.g. `--line-number` and
    /// `--no-line-number`) silently produces a command whose behaviour does
    /// not match what both checkboxes suggest. Call this right after a
    /// checkbox is switched on.
    pub(crate) fn enforce_exclusive_option(&mut self, flag: &'static str) {
        for conflicting_flag in crate::options::conflicting_flags(flag) {
            if *conflicting_flag == flag {
                continue;
            }
            if let Some(index) = OPTIONS
                .iter()
                .position(|spec| spec.flag == *conflicting_flag)
            {
                self.option_states[index].enabled = false;
            }
        }
    }

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

    fn is_enabled(app: &RgStudio, flag: &str) -> bool {
        let index = OPTIONS
            .iter()
            .position(|spec| spec.flag == flag)
            .expect("option flag should exist");
        app.option_states[index].enabled
    }

    #[test]
    fn enabling_a_case_mode_disables_the_other_case_modes() {
        let mut app = RgStudio::default();
        enable_option(&mut app, "--case-sensitive", None);
        enable_option(&mut app, "--smart-case", None);
        app.enforce_exclusive_option("--smart-case");

        assert!(!is_enabled(&app, "--case-sensitive"));
        assert!(is_enabled(&app, "--smart-case"));
        assert!(!is_enabled(&app, "--ignore-case"));
    }

    #[test]
    fn enabling_no_line_number_disables_line_number() {
        let mut app = RgStudio::default();
        enable_option(&mut app, "--line-number", None);
        enable_option(&mut app, "--no-line-number", None);
        app.enforce_exclusive_option("--no-line-number");

        assert!(!is_enabled(&app, "--line-number"));
        assert!(is_enabled(&app, "--no-line-number"));
    }

    #[test]
    fn unrelated_options_are_unaffected_by_conflict_enforcement() {
        let mut app = RgStudio::default();
        enable_option(&mut app, "--hidden", None);
        app.enforce_exclusive_option("--hidden");

        assert!(is_enabled(&app, "--hidden"));
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
