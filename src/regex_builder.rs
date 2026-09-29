/// The expression represented by one visual regex row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Atom {
    Literal,
    Raw,
    AnyCharacter,
    Digit,
    WordCharacter,
    Whitespace,
    CharacterClass,
    NegatedClass,
    CaptureGroup,
    NonCaptureGroup,
    WordBoundary,
    Alternation,
}

impl Atom {
    /// All atom types shown by the visual editor.
    pub const ALL: [Self; 12] = [
        Self::Literal,
        Self::Raw,
        Self::AnyCharacter,
        Self::Digit,
        Self::WordCharacter,
        Self::Whitespace,
        Self::CharacterClass,
        Self::NegatedClass,
        Self::CaptureGroup,
        Self::NonCaptureGroup,
        Self::WordBoundary,
        Self::Alternation,
    ];

    /// Human-readable atom name.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Literal => "Literal text",
            Self::Raw => "Raw regex",
            Self::AnyCharacter => "Any character",
            Self::Digit => "Digit",
            Self::WordCharacter => "Word character",
            Self::Whitespace => "Whitespace",
            Self::CharacterClass => "Character class",
            Self::NegatedClass => "Negated class",
            Self::CaptureGroup => "Capture group",
            Self::NonCaptureGroup => "Non-capture group",
            Self::WordBoundary => "Word boundary",
            Self::Alternation => "Alternative (|)",
        }
    }

    /// Whether this atom requires editable content.
    pub const fn accepts_value(self) -> bool {
        matches!(
            self,
            Self::Literal
                | Self::Raw
                | Self::CharacterClass
                | Self::NegatedClass
                | Self::CaptureGroup
                | Self::NonCaptureGroup
        )
    }

    pub(crate) const fn accepts_quantifier(self) -> bool {
        !matches!(self, Self::Alternation | Self::WordBoundary)
    }
}

/// Repetition applied to a regex atom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Quantifier {
    #[default]
    Once,
    Optional,
    ZeroOrMore,
    OneOrMore,
    Exactly,
    Range,
}

impl Quantifier {
    /// All repetitions shown by the visual editor.
    pub const ALL: [Self; 6] = [
        Self::Once,
        Self::Optional,
        Self::ZeroOrMore,
        Self::OneOrMore,
        Self::Exactly,
        Self::Range,
    ];

    /// Compact repetition label.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Optional => "?",
            Self::ZeroOrMore => "*",
            Self::OneOrMore => "+",
            Self::Exactly => "{n}",
            Self::Range => "{min,max}",
        }
    }
}

/// One editable row in the visual regex builder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegexPart {
    /// Atom represented by this row.
    pub atom: Atom,
    /// Atom-specific text or raw regex.
    pub value: String,
    /// Repetition applied to the atom.
    pub quantifier: Quantifier,
    /// Exact count or lower range bound.
    pub minimum: u32,
    /// Upper range bound.
    pub maximum: u32,
}

impl Default for RegexPart {
    fn default() -> Self {
        Self {
            atom: Atom::Literal,
            value: String::new(),
            quantifier: Quantifier::Once,
            minimum: 1,
            maximum: 3,
        }
    }
}

impl RegexPart {
    fn expression(&self) -> String {
        let atom = match self.atom {
            Atom::Literal => regex::escape(&self.value),
            Atom::Raw => self.value.clone(),
            Atom::AnyCharacter => ".".to_owned(),
            Atom::Digit => r"\d".to_owned(),
            Atom::WordCharacter => r"\w".to_owned(),
            Atom::Whitespace => r"\s".to_owned(),
            Atom::CharacterClass => format!("[{}]", self.value),
            Atom::NegatedClass => format!("[^{}]", self.value),
            Atom::CaptureGroup => format!("({})", self.value),
            Atom::NonCaptureGroup => format!("(?:{})", self.value),
            Atom::WordBoundary => r"\b".to_owned(),
            Atom::Alternation => "|".to_owned(),
        };

        if !self.atom.accepts_quantifier() {
            return atom;
        }

        let atom = if self.atom == Atom::Raw && self.quantifier != Quantifier::Once {
            format!("(?:{atom})")
        } else {
            atom
        };

        let suffix = match self.quantifier {
            Quantifier::Once => String::new(),
            Quantifier::Optional => "?".to_owned(),
            Quantifier::ZeroOrMore => "*".to_owned(),
            Quantifier::OneOrMore => "+".to_owned(),
            Quantifier::Exactly => format!("{{{}}}", self.minimum),
            Quantifier::Range => format!("{{{},{}}}", self.minimum, self.maximum),
        };
        format!("{atom}{suffix}")
    }
}

/// State and generator for the visual regular-expression editor.
#[derive(Debug, Default)]
pub struct RegexBuilder {
    /// Match only at the start of a line.
    pub start_anchor: bool,
    /// Match only at the end of a line.
    pub end_anchor: bool,
    /// Ordered expression rows.
    pub parts: Vec<RegexPart>,
}

impl RegexBuilder {
    /// Creates a builder that preserves an existing pattern as raw regex.
    #[must_use]
    pub fn from_pattern(pattern: &str) -> Self {
        let parts = if pattern.is_empty() {
            vec![RegexPart::default()]
        } else {
            vec![RegexPart {
                atom: Atom::Raw,
                value: pattern.to_owned(),
                ..RegexPart::default()
            }]
        };
        Self {
            start_anchor: false,
            end_anchor: false,
            parts,
        }
    }

    /// Builds the regular expression represented by all rows.
    #[must_use]
    pub fn pattern(&self) -> String {
        let mut pattern = String::new();
        if self.start_anchor {
            pattern.push('^');
        }
        for part in &self.parts {
            pattern.push_str(&part.expression());
        }
        if self.end_anchor {
            pattern.push('$');
        }
        pattern
    }

    /// Validates the generated expression with the default ripgrep-compatible engine.
    pub fn validation_error(&self) -> Option<String> {
        regex::Regex::new(&self.pattern())
            .err()
            .map(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{Atom, Quantifier, RegexBuilder, RegexPart};

    #[test]
    fn builds_and_escapes_a_structured_pattern() {
        let builder = RegexBuilder {
            start_anchor: true,
            end_anchor: true,
            parts: vec![
                RegexPart {
                    value: "file.".into(),
                    ..RegexPart::default()
                },
                RegexPart {
                    atom: Atom::Digit,
                    quantifier: Quantifier::OneOrMore,
                    ..RegexPart::default()
                },
            ],
        };

        assert_eq!(builder.pattern(), r"^file\.\d+$");
        assert_eq!(builder.validation_error(), None);
    }

    #[test]
    fn preserves_an_existing_pattern() {
        let builder = RegexBuilder::from_pattern(r"foo\s+bar");
        assert_eq!(builder.pattern(), r"foo\s+bar");
    }

    #[test]
    fn quantifier_applies_to_the_entire_raw_expression() {
        let builder = RegexBuilder {
            parts: vec![RegexPart {
                atom: Atom::Raw,
                value: "foo|bar".into(),
                quantifier: Quantifier::OneOrMore,
                ..RegexPart::default()
            }],
            ..RegexBuilder::default()
        };

        assert_eq!(builder.pattern(), "(?:foo|bar)+");
        assert_eq!(builder.validation_error(), None);
    }
}
