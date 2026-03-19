/// Deterministic tokenizer for ingredient text.
///
/// Steps:
/// 1. Lowercase
/// 2. Replace punctuation (, . ; : / \n \t) with space
/// 3. Split on whitespace
/// 4. Strip numbers & units ("100g salmon" → "salmon")
/// 5. Remove stop-words ("и", "с", "with", "and", …)
/// 6. Merge known multi-word tokens (e.g. "soy" + "sauce" → "soy-sauce")
/// 7. Drop empty / too-short tokens
///
/// Returns at most `max` tokens (protection against huge inputs).

/// Known multi-word ingredient slugs (2 words).
static MULTI_WORDS: &[&[&str]] = &[
    &["olive", "oil"],
    &["soy", "sauce"],
    &["sesame", "oil"],
    &["coconut", "oil"],
    &["coconut", "milk"],
    &["fish", "sauce"],
    &["rice", "vinegar"],
    &["balsamic", "vinegar"],
    &["cream", "cheese"],
    &["sour", "cream"],
    &["ground", "meat"],
    &["green", "peas"],
    &["green", "onion"],
    &["frozen", "vegetables"],
    &["chili", "pepper"],
    &["bell", "pepper"],
    &["black", "pepper"],
    &["white", "pepper"],
    &["bay", "leaf"],
    &["maple", "syrup"],
    &["peanut", "butter"],
    &["whipped", "cream"],
    &["dark", "chocolate"],
    &["milk", "chocolate"],
    &["white", "chocolate"],
    &["brown", "sugar"],
    &["powdered", "sugar"],
    &["vanilla", "extract"],
    &["heavy", "cream"],
    &["mozzarella", "cheese"],
    &["parmesan", "cheese"],
    &["goat", "cheese"],
    &["feta", "cheese"],
    &["cheddar", "cheese"],
    &["ricotta", "cheese"],
    &["mascarpone", "cheese"],
];

/// Stop-words in EN/RU/UK/PL that carry no ingredient meaning.
static STOP_WORDS: &[&str] = &[
    // EN
    "and", "with", "or", "the", "a", "an", "of", "to", "in", "for", "on", "some", "fresh",
    "chopped", "sliced", "diced", "minced", "grated", "peeled", "optional",
    // RU
    "и", "с", "или", "на", "для", "по", "из", "не", "от", "до", "без",
    "немного", "свежий", "свежая", "свежее", "нарезанный", "тёртый",
    // UK
    "та", "або", "для", "від", "до", "без", "трохи", "свіжий", "свіжа",
    // PL
    "i", "z", "lub", "na", "do", "od", "bez", "trochę", "świeży", "świeża",
    // Units (often left in text)
    "g", "gr", "kg", "ml", "l", "oz", "lb", "cup", "tbsp", "tsp",
    "gram", "grams", "грамм", "грам", "кг", "мл", "литр",
    "штук", "штуки", "шт", "szt",
];

/// Returns true if the word is purely numeric or a number+unit like "100g", "200мл".
fn is_numeric_token(w: &str) -> bool {
    if w.is_empty() {
        return true;
    }
    // Pure number
    if w.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return true;
    }
    // Number followed by letters: "100g", "200ml", "50мл"
    let digit_prefix: String = w.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    if !digit_prefix.is_empty() && digit_prefix.len() < w.len() {
        let suffix = &w[digit_prefix.len()..];
        // If the rest is a known unit or very short (≤3 chars), it's numeric
        let short_units = ["g", "gr", "kg", "ml", "l", "oz", "lb", "гр", "мл", "кг", "л", "szt"];
        if suffix.len() <= 3 || short_units.contains(&suffix) {
            return true;
        }
    }
    false
}

fn is_stop_word(w: &str) -> bool {
    STOP_WORDS.contains(&w)
}

pub fn tokenize(text: &str, max: usize) -> Vec<String> {
    let lower = text.to_lowercase();

    // Replace punctuation with space
    let cleaned: String = lower
        .chars()
        .map(|c| match c {
            ',' | '.' | ';' | ':' | '/' | '\\' | '\n' | '\t' | '\r' | '(' | ')' | '[' | ']'
            | '{' | '}' | '"' | '\'' | '!' | '?' | '—' | '–' | '·' | '•' | '…' | '-' => ' ',
            _ => c,
        })
        .collect();

    // Split, filter noise
    let words: Vec<&str> = cleaned
        .split_whitespace()
        .filter(|w| !is_numeric_token(w) && !is_stop_word(w) && w.len() > 1)
        .collect();

    let mut tokens: Vec<String> = Vec::new();
    let mut i = 0;

    while i < words.len() && tokens.len() < max {
        // Try multi-word merge
        let mut merged = false;
        if i + 1 < words.len() {
            for mw in MULTI_WORDS {
                if mw.len() == 2 && words[i] == mw[0] && words[i + 1] == mw[1] {
                    tokens.push(format!("{}-{}", mw[0], mw[1]));
                    i += 2;
                    merged = true;
                    break;
                }
            }
        }
        if !merged {
            tokens.push(words[i].to_string());
            i += 1;
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_split() {
        let tokens = tokenize("salmon rice avocado", 15);
        assert_eq!(tokens, vec!["salmon", "rice", "avocado"]);
    }

    #[test]
    fn punctuation_handling() {
        let tokens = tokenize("лосось, рис; авокадо. лимон", 15);
        assert_eq!(tokens, vec!["лосось", "рис", "авокадо", "лимон"]);
    }

    #[test]
    fn multi_word_merge() {
        let tokens = tokenize("olive oil soy sauce salmon", 15);
        assert_eq!(tokens, vec!["olive-oil", "soy-sauce", "salmon"]);
    }

    #[test]
    fn limit_enforcement() {
        let input = (0..30).map(|i| format!("word{}", i)).collect::<Vec<_>>().join(" ");
        let tokens = tokenize(&input, 15);
        assert_eq!(tokens.len(), 15);
    }

    #[test]
    fn strip_numbers_and_units() {
        let tokens = tokenize("100g salmon 200мл rice", 15);
        assert_eq!(tokens, vec!["salmon", "rice"]);
    }

    #[test]
    fn stop_words_removed() {
        let tokens = tokenize("salmon and rice with avocado", 15);
        assert_eq!(tokens, vec!["salmon", "rice", "avocado"]);
    }

    #[test]
    fn russian_stop_words_removed() {
        let tokens = tokenize("лосось и рис с авокадо", 15);
        assert_eq!(tokens, vec!["лосось", "рис", "авокадо"]);
    }

    #[test]
    fn complex_recipe_text() {
        let tokens = tokenize("лосось 200 грамм, рис 100g, свежий авокадо", 15);
        assert_eq!(tokens, vec!["лосось", "рис", "авокадо"]);
    }
}
