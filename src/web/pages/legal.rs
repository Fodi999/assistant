use crate::web::{language, pages::i18n};

pub fn privacy(lang: language::Lang) -> String {
    i18n::pack(lang).privacy_html.to_string()
}

pub fn terms(lang: language::Lang) -> String {
    i18n::pack(lang).terms_html.to_string()
}
