use crate::web::{language, pages::i18n};

pub fn render(lang: language::Lang) -> String {
    i18n::pack(lang).booking_html.to_string()
}
