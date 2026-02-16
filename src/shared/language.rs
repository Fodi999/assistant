use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,  // English
    Pl,  // Polish
    Uk,  // Ukrainian
    Ru,  // Russian
}

impl Language {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "en" | "english" => Ok(Language::En),
            "pl" | "polish" | "polski" => Ok(Language::Pl),
            "uk" | "ukrainian" | "українська" => Ok(Language::Uk),
            "ru" | "russian" | "русский" => Ok(Language::Ru),
            _ => Err(format!("Unsupported language: {}", s)),
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "en" => Some(Language::En),
            "pl" => Some(Language::Pl),
            "uk" => Some(Language::Uk),
            "ru" => Some(Language::Ru),
            _ => None,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Pl => "pl",
            Language::Uk => "uk",
            Language::Ru => "ru",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Pl => "Polski",
            Language::Uk => "Українська",
            Language::Ru => "Русский",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Language::En, Language::Pl, Language::Uk, Language::Ru]
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::En
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_str("en").unwrap(), Language::En);
        assert_eq!(Language::from_str("pl").unwrap(), Language::Pl);
        assert_eq!(Language::from_str("uk").unwrap(), Language::Uk);
        assert_eq!(Language::from_str("ru").unwrap(), Language::Ru);
    }

    #[test]
    fn test_language_code() {
        assert_eq!(Language::En.code(), "en");
        assert_eq!(Language::Pl.code(), "pl");
        assert_eq!(Language::Uk.code(), "uk");
        assert_eq!(Language::Ru.code(), "ru");
    }
}
