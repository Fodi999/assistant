//! Telegram inline keyboard builders for the "Світло Ікони" bot.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct InlineKeyboardMarkup {
    pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InlineKeyboardButton {
    pub text: String,
    pub callback_data: String,
}

impl InlineKeyboardButton {
    pub fn new(text: impl Into<String>, callback_data: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            callback_data: callback_data.into(),
        }
    }
}

/// The keyboard shown under `/start` and re-attached to `/today` replies so
/// users can keep navigating without retyping commands.
pub fn main_menu_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup {
        inline_keyboard: vec![
            vec![InlineKeyboardButton::new("☦️ Сьогодні", "today")],
            vec![
                InlineKeyboardButton::new("🙏 Молитва", "prayer"),
                InlineKeyboardButton::new("📖 Євангеліє", "gospel"),
            ],
            vec![InlineKeyboardButton::new("🕯 Святий дня", "saint")],
            vec![InlineKeyboardButton::new("⚙️ Налаштування", "settings")],
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_menu_has_all_five_actions() {
        let markup = main_menu_keyboard();
        let labels: Vec<&str> = markup
            .inline_keyboard
            .iter()
            .flatten()
            .map(|b| b.text.as_str())
            .collect();

        assert_eq!(labels.len(), 5);
        assert!(labels.contains(&"☦️ Сьогодні"));
        assert!(labels.contains(&"🙏 Молитва"));
        assert!(labels.contains(&"📖 Євангеліє"));
        assert!(labels.contains(&"🕯 Святий дня"));
        assert!(labels.contains(&"⚙️ Налаштування"));
    }

    #[test]
    fn callback_data_matches_command_router() {
        let markup = main_menu_keyboard();
        let data: Vec<&str> = markup
            .inline_keyboard
            .iter()
            .flatten()
            .map(|b| b.callback_data.as_str())
            .collect();

        for expected in ["today", "prayer", "gospel", "saint", "settings"] {
            assert!(
                data.contains(&expected),
                "missing callback_data {expected:?}"
            );
        }
    }
}
