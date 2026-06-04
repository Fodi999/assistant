pub mod en;
pub mod pl;
pub mod ru;

pub use pl as active;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lang {
    Pl,
    Ru,
    En,
}

pub struct LanguagePack {
    pub code: &'static str,
    pub label: &'static str,
    pub shell: &'static ShellText,
    pub titles: &'static PageTitles,
    pub js: &'static JsText,
}

pub const LANG_COOKIE: &str = "chef-lang";

pub const LANG_OPTIONS: &[(&str, &str)] = &[("pl", "PL"), ("ru", "RU"), ("en", "EN")];

impl Lang {
    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().to_ascii_lowercase().as_str() {
            "pl" | "pl-pl" => Some(Self::Pl),
            "ru" | "ru-ru" => Some(Self::Ru),
            "en" | "en-us" | "en-gb" => Some(Self::En),
            _ => None,
        }
    }

    pub fn from_cookie_header(cookie_header: &str) -> Option<Self> {
        cookie_header.split(';').find_map(|part| {
            let (name, value) = part.trim().split_once('=')?;
            (name == LANG_COOKIE)
                .then(|| Self::from_code(value))
                .flatten()
        })
    }

    pub fn resolve(query_lang: Option<&str>, cookie_header: Option<&str>) -> Self {
        query_lang
            .and_then(Self::from_code)
            .or_else(|| cookie_header.and_then(Self::from_cookie_header))
            .unwrap_or(Self::Pl)
    }

    pub fn pack(self) -> LanguagePack {
        match self {
            Self::Pl => LanguagePack {
                code: "pl",
                label: "PL",
                shell: &pl::SHELL,
                titles: &pl::TITLES,
                js: &pl::JS,
            },
            Self::Ru => LanguagePack {
                code: "ru",
                label: "RU",
                shell: &ru::SHELL,
                titles: &ru::TITLES,
                js: &ru::JS,
            },
            Self::En => LanguagePack {
                code: "en",
                label: "EN",
                shell: &en::SHELL,
                titles: &en::TITLES,
                js: &en::JS,
            },
        }
    }
}

pub struct ShellText {
    pub html_lang: &'static str,
    pub brand_plain: &'static str,
    pub brand_accent: &'static str,
    pub nav_start: &'static str,
    pub nav_menu: &'static str,
    pub nav_delivery: &'static str,
    pub nav_booking: &'static str,
    pub nav_about: &'static str,
    pub nav_table: &'static str,
    pub nav_order: &'static str,
    pub nav_language: &'static str,
    pub aria_menu: &'static str,
    pub cookie_aria: &'static str,
    pub cookie_title: &'static str,
    pub cookie_intro: &'static str,
    pub cookie_necessary: &'static str,
    pub cookie_accept: &'static str,
    pub trust_delivery: &'static str,
    pub trust_pickup: &'static str,
    pub trust_booking: &'static str,
    pub trust_author: &'static str,
    pub footer_tagline: &'static str,
    pub footer_guests: &'static str,
    pub footer_restaurant: &'static str,
    pub footer_contact: &'static str,
    pub footer_menu: &'static str,
    pub footer_delivery: &'static str,
    pub footer_booking: &'static str,
    pub footer_blog: &'static str,
    pub footer_about: &'static str,
    pub footer_haccp: &'static str,
    pub footer_privacy: &'static str,
    pub footer_terms: &'static str,
    pub footer_cookie: &'static str,
    pub footer_manage_cookie: &'static str,
    pub footer_copy: &'static str,
}

pub struct PageTitles {
    pub start: &'static str,
    pub menu: &'static str,
    pub chef_blog: &'static str,
    pub delivery: &'static str,
    pub booking: &'static str,
    pub recipe_detail: &'static str,
    pub about: &'static str,
    pub cookie: &'static str,
    pub privacy: &'static str,
    pub terms: &'static str,
    pub not_found: &'static str,
}

pub struct JsText {
    pub order_added: &'static str,
}
