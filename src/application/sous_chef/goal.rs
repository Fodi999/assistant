//! Goal detection — pure Rust, zero dependencies.
//!
//! Normalizes user query and detects intent (weight loss, protein, breakfast, etc.)

/// Detected user goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    WeightLoss,
    HighProtein,
    QuickBreakfast,
    FromIngredients,
    HealthyDay,
    Generic,
}

impl Goal {
    pub fn slug(&self) -> &'static str {
        match self {
            Goal::WeightLoss => "weight_loss",
            Goal::HighProtein => "high_protein",
            Goal::QuickBreakfast => "quick_breakfast",
            Goal::FromIngredients => "from_ingredients",
            Goal::HealthyDay => "healthy_day",
            Goal::Generic => "generic",
        }
    }

    pub fn detect(q: &str) -> Self {
        let q = q.to_lowercase();
        if q.contains("похуд") || q.contains("дефицит") || q.contains("калори")
            || q.contains("odchudz") || q.contains("schudnąć") || q.contains("weight loss")
            || q.contains("lose weight") || q.contains("схуднути") || q.contains("калорій")
        {
            return Goal::WeightLoss;
        }
        if q.contains("белок") || q.contains("белк") || q.contains("протеин")
            || q.contains("protein") || q.contains("białko") || q.contains("білок")
        {
            return Goal::HighProtein;
        }
        if q.contains("завтрак") || q.contains("śniadani") || q.contains("breakfast")
            || q.contains("сніданок") || q.contains("быстр") || q.contains("szybk")
            || q.contains("quick")
        {
            return Goal::QuickBreakfast;
        }
        if q.contains("приготовить из") || q.contains("з чого") || q.contains("what to cook")
            || q.contains("co ugotować") || q.contains("из:") || q.contains("from:")
        {
            return Goal::FromIngredients;
        }
        if q.contains("здоров") || q.contains("zdrowy") || q.contains("healthy") {
            return Goal::HealthyDay;
        }
        Goal::Generic
    }
}

/// Normalize query for cache key generation.
pub fn normalize_query(q: &str) -> String {
    let q = q.trim().to_lowercase();
    let q: String = q
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == ':' || c == ',' { c } else { ' ' })
        .collect();
    q.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Public helper — returns goal slug for in-memory cache key.
pub fn normalize_for_cache(q: &str) -> String {
    let norm = normalize_query(q);
    Goal::detect(&norm).slug().to_string()
}

/// Build DB cache key: "sous_chef:{goal}:{lang}"
pub fn build_cache_key(goal: Goal, lang: &str) -> String {
    format!("sous_chef:{}:{}", goal.slug(), lang)
}
