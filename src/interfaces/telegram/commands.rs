//! Command parsing and pure text templates for the "Світло Ікони" bot.
//!
//! Nothing in this module touches the database or the network — it only
//! turns Telegram input into a [`Command`] and turns church-domain data
//! (already fetched by `webhook.rs` from the existing church repository
//! functions) into the Ukrainian strings sent back to the user. That split
//! keeps this module trivially unit-testable.

use crate::interfaces::http::church_content::{
    ChurchGospelDto, ChurchPrayerDto, ChurchSaintDto, PublicChurchContentPage,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Start,
    Today,
    Prayer,
    Saint,
    Gospel,
    Help,
}

impl Command {
    /// Parses a Telegram message's `text` field as a bot command.
    ///
    /// Handles the two things real Telegram clients actually send that a
    /// naive `match` on the raw text would miss:
    /// - `/start@SvitloIkonyBot` (Telegram appends `@BotName` in group chats)
    /// - `/today some trailing args` (only the first whitespace-separated
    ///   token is the command)
    pub fn parse_slash(text: &str) -> Option<Command> {
        let trimmed = text.trim();
        if !trimmed.starts_with('/') {
            return None;
        }
        let first_token = trimmed.split_whitespace().next()?;
        let name = first_token
            .trim_start_matches('/')
            .split('@')
            .next()
            .unwrap_or("");

        match name.to_ascii_lowercase().as_str() {
            "start" => Some(Command::Start),
            "today" => Some(Command::Today),
            "prayer" => Some(Command::Prayer),
            "saint" => Some(Command::Saint),
            "gospel" => Some(Command::Gospel),
            "help" => Some(Command::Help),
            _ => None,
        }
    }

    /// Maps an inline-keyboard button's `callback_data` (see `keyboards.rs`)
    /// back to the same [`Command`] variant its slash-command twin produces.
    pub fn from_callback_data(data: &str) -> Option<Command> {
        match data {
            "today" => Some(Command::Today),
            "prayer" => Some(Command::Prayer),
            "saint" => Some(Command::Saint),
            "gospel" => Some(Command::Gospel),
            "help" => Some(Command::Help),
            _ => None,
        }
    }
}

pub const START_TEXT: &str = "☦️ Вітаємо у «Світло Ікони»\n\n\
Я допоможу вам щодня бути поруч із православною традицією:\n\
🙏 молитви\n\
📖 Євангеліє\n\
☦️ церковний календар\n\
🕯 святі дня\n\
📚 духовні історії";

pub const HELP_TEXT: &str = "Доступні команди:\n\
/today — церковний календар на сьогодні\n\
/prayer — молитва\n\
/saint — святий дня\n\
/gospel — уривок з Євангелія\n\
/help — ця підказка";

pub const SETTINGS_STUB_TEXT: &str = "⚙️ Налаштування скоро з'являться.";

pub const NO_CONTENT_TODAY: &str =
    "На сьогодні ще немає опублікованого запису церковного календаря. Спробуйте пізніше 🙏";
pub const NO_PRAYER_TEXT: &str = "Молитви ще не додані. Спробуйте пізніше 🙏";
pub const NO_SAINT_TEXT: &str = "Інформацію про святого дня ще не додано. Спробуйте пізніше 🕯";
pub const NO_GOSPEL_TEXT: &str = "Уривок з Євангелія ще не додано. Спробуйте пізніше 📖";
pub const GENERIC_ERROR_TEXT: &str = "Сталася помилка. Спробуйте, будь ласка, пізніше.";

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Formats the `/today` summary: calendar day title/description plus a
/// one-line teaser for the saint, prayer and gospel reading of the day (when
/// present) so the user can decide what to open next via the inline keyboard.
pub fn format_today(page: &PublicChurchContentPage, saints: &[ChurchSaintDto]) -> String {
    let mut text = format!("☦️ {}", page.calendar_day.title);

    if let Some(description) = non_empty(&page.calendar_day.description) {
        text.push('\n');
        text.push_str(description);
    }

    if let Some(saint) = saints.first() {
        text.push_str(&format!("\n\n🕯 Святий дня: {}", saint.name));
    }
    if let Some(prayer) = page.prayers.first() {
        text.push_str(&format!("\n🙏 Молитва: {}", prayer.title));
    }
    if let Some(gospel) = page.gospel.first() {
        text.push_str(&format!(
            "\n📖 Євангеліє: {} ({})",
            gospel.title, gospel.reference
        ));
    }

    text.push_str("\n\nОберіть розділ нижче, щоб дізнатися більше 👇");
    text
}

pub fn format_prayer(prayer: &ChurchPrayerDto) -> String {
    format!("🙏 {}\n\n{}", prayer.title, prayer.text)
}

pub fn format_saint(saint: &ChurchSaintDto) -> String {
    let mut text = format!("🕯 {}", saint.name);
    if let Some(feast_day) = non_empty(&saint.feast_day) {
        text.push_str(&format!("\nДень пам'яті: {feast_day}"));
    }
    if let Some(description) = non_empty(&saint.short_description) {
        text.push_str(&format!("\n\n{description}"));
    }
    text
}

pub fn format_gospel(gospel: &ChurchGospelDto) -> String {
    format!(
        "📖 {} ({})\n\n{}",
        gospel.title, gospel.reference, gospel.text
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_start() {
        assert_eq!(Command::parse_slash("/start"), Some(Command::Start));
    }

    #[test]
    fn parses_start_with_bot_name_suffix() {
        // Telegram appends "@BotName" to commands in group chats.
        assert_eq!(
            Command::parse_slash("/start@SvitloIkonyBot"),
            Some(Command::Start)
        );
    }

    #[test]
    fn parses_command_with_trailing_args() {
        assert_eq!(Command::parse_slash("/today please"), Some(Command::Today));
    }

    #[test]
    fn is_case_insensitive() {
        assert_eq!(Command::parse_slash("/START"), Some(Command::Start));
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(Command::parse_slash("  /help  "), Some(Command::Help));
    }

    #[test]
    fn all_required_commands_parse() {
        assert_eq!(Command::parse_slash("/today"), Some(Command::Today));
        assert_eq!(Command::parse_slash("/prayer"), Some(Command::Prayer));
        assert_eq!(Command::parse_slash("/saint"), Some(Command::Saint));
        assert_eq!(Command::parse_slash("/gospel"), Some(Command::Gospel));
        assert_eq!(Command::parse_slash("/help"), Some(Command::Help));
    }

    #[test]
    fn rejects_free_text() {
        assert_eq!(Command::parse_slash("Слава Ісусу Христу"), None);
        assert_eq!(Command::parse_slash(""), None);
    }

    #[test]
    fn rejects_unknown_slash_command() {
        assert_eq!(Command::parse_slash("/unknown"), None);
    }

    #[test]
    fn callback_data_maps_to_the_same_commands_as_slash_text() {
        assert_eq!(Command::from_callback_data("today"), Some(Command::Today));
        assert_eq!(Command::from_callback_data("prayer"), Some(Command::Prayer));
        assert_eq!(Command::from_callback_data("saint"), Some(Command::Saint));
        assert_eq!(Command::from_callback_data("gospel"), Some(Command::Gospel));
        assert_eq!(Command::from_callback_data("settings"), None);
        assert_eq!(Command::from_callback_data("bogus"), None);
    }
}
