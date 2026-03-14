use crate::domain::ProcessingState;

/// Static translations for state name suffixes.
/// These are deterministic — no AI needed.
#[derive(Debug, Clone)]
pub struct StateSuffixTranslations {
    pub en: &'static str,
    pub pl: &'static str,
    pub ru: &'static str,
    pub uk: &'static str,
}

/// Get pre-defined translations for each processing state suffix.
/// Example: "Salmon" + "boiled" → "Salmon (boiled)" / "Лосось (варёный)"
pub fn get_state_suffix(state: ProcessingState) -> StateSuffixTranslations {
    match state {
        ProcessingState::Raw => StateSuffixTranslations {
            en: "raw",
            pl: "surowy",
            ru: "сырой",
            uk: "сирий",
        },
        ProcessingState::Boiled => StateSuffixTranslations {
            en: "boiled",
            pl: "gotowany",
            ru: "варёный",
            uk: "варений",
        },
        ProcessingState::Fried => StateSuffixTranslations {
            en: "fried",
            pl: "smażony",
            ru: "жареный",
            uk: "смажений",
        },
        ProcessingState::Baked => StateSuffixTranslations {
            en: "baked",
            pl: "pieczony",
            ru: "запечённый",
            uk: "запечений",
        },
        ProcessingState::Grilled => StateSuffixTranslations {
            en: "grilled",
            pl: "grillowany",
            ru: "на гриле",
            uk: "на грилі",
        },
        ProcessingState::Steamed => StateSuffixTranslations {
            en: "steamed",
            pl: "gotowany na parze",
            ru: "на пару",
            uk: "на парі",
        },
        ProcessingState::Smoked => StateSuffixTranslations {
            en: "smoked",
            pl: "wędzony",
            ru: "копчёный",
            uk: "копчений",
        },
        ProcessingState::Frozen => StateSuffixTranslations {
            en: "frozen",
            pl: "mrożony",
            ru: "замороженный",
            uk: "заморожений",
        },
        ProcessingState::Dried => StateSuffixTranslations {
            en: "dried",
            pl: "suszony",
            ru: "сушёный",
            uk: "сушений",
        },
        ProcessingState::Pickled => StateSuffixTranslations {
            en: "pickled",
            pl: "marynowany",
            ru: "маринованный",
            uk: "маринований",
        },
    }
}

/// Notes templates for each state (cooking guidelines)
#[derive(Debug, Clone)]
pub struct StateNotes {
    pub en: &'static str,
    pub pl: &'static str,
    pub ru: &'static str,
    pub uk: &'static str,
}

pub fn get_state_notes(state: ProcessingState) -> StateNotes {
    match state {
        ProcessingState::Raw => StateNotes {
            en: "Fresh, unprocessed ingredient. Store refrigerated.",
            pl: "Świeży, nieprzetworzony składnik. Przechowywać w lodówce.",
            ru: "Свежий необработанный продукт. Хранить в холодильнике.",
            uk: "Свіжий необроблений продукт. Зберігати в холодильнику.",
        },
        ProcessingState::Boiled => StateNotes {
            en: "Cooked in boiling water. Softer texture, some nutrient loss to water.",
            pl: "Gotowany w wodzie. Miększa tekstura, pewna utrata składników do wody.",
            ru: "Варёный в воде. Мягкая текстура, часть нутриентов переходит в воду.",
            uk: "Зварений у воді. М'яка текстура, частина нутрієнтів переходить у воду.",
        },
        ProcessingState::Fried => StateNotes {
            en: "Pan-fried or deep-fried in oil. Higher fat and calorie content.",
            pl: "Smażony na patelni lub w głębokim oleju. Wyższa zawartość tłuszczu i kalorii.",
            ru: "Жареный на сковороде или во фритюре. Повышенное содержание жира и калорий.",
            uk: "Смажений на сковороді або у фритюрі. Підвищений вміст жиру та калорій.",
        },
        ProcessingState::Baked => StateNotes {
            en: "Oven-baked. Moderate water loss, flavour concentration.",
            pl: "Pieczony w piekarniku. Umiarkowana utrata wody, koncentracja smaku.",
            ru: "Запечённый в духовке. Умеренная потеря воды, концентрация вкуса.",
            uk: "Запечений у духовці. Помірна втрата води, концентрація смаку.",
        },
        ProcessingState::Grilled => StateNotes {
            en: "Grilled over high heat. Fat drips off, smoky flavour.",
            pl: "Grillowany na dużym ogniu. Tłuszcz odcieknie, wędzony smak.",
            ru: "Приготовленный на гриле. Жир стекает, дымный аромат.",
            uk: "Приготовлений на грилі. Жир стікає, димний аромат.",
        },
        ProcessingState::Steamed => StateNotes {
            en: "Steamed. Minimal nutrient loss, retains natural texture.",
            pl: "Gotowany na parze. Minimalna utrata składników, zachowuje naturalną teksturę.",
            ru: "Приготовленный на пару. Минимальная потеря нутриентов, сохраняет текстуру.",
            uk: "Приготовлений на парі. Мінімальна втрата нутрієнтів, зберігає текстуру.",
        },
        ProcessingState::Smoked => StateNotes {
            en: "Smoked. Extended shelf life, concentrated flavour, lower moisture.",
            pl: "Wędzony. Dłuższy okres przydatności, skoncentrowany smak, mniejsza wilgotność.",
            ru: "Копчёный. Увеличенный срок хранения, концентрированный вкус, меньше влаги.",
            uk: "Копчений. Збільшений термін зберігання, концентрований смак, менше вологи.",
        },
        ProcessingState::Frozen => StateNotes {
            en: "Frozen at -18°C. Nutrition virtually unchanged. Thaw before use.",
            pl: "Zamrożony w -18°C. Wartości odżywcze praktycznie niezmienione. Rozmrozić przed użyciem.",
            ru: "Заморожен при -18°C. Пищевая ценность практически не изменяется. Разморозить перед использованием.",
            uk: "Заморожений при -18°C. Харчова цінність практично не змінюється. Розморозити перед використанням.",
        },
        ProcessingState::Dried => StateNotes {
            en: "Dried/dehydrated. Highly concentrated nutrients. Rehydrate before cooking.",
            pl: "Suszony/odwodniony. Silnie skoncentrowane składniki. Namoczyć przed gotowaniem.",
            ru: "Сушёный/обезвоженный. Сильно концентрированные нутриенты. Замочить перед приготовлением.",
            uk: "Сушений/зневоднений. Сильно концентровані нутрієнти. Замочити перед приготуванням.",
        },
        ProcessingState::Pickled => StateNotes {
            en: "Pickled in brine or vinegar. Acidic taste, extended shelf life.",
            pl: "Marynowany w solance lub occie. Kwaśny smak, dłuższy okres przydatności.",
            ru: "Маринованный в рассоле или уксусе. Кислый вкус, увеличенный срок хранения.",
            uk: "Маринований у розсолі або оцті. Кислий смак, збільшений термін зберігання.",
        },
    }
}
