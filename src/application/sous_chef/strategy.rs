//! Meal strategies — pure data, no calories, no DB.
//!
//! A strategy = list of catalog slugs + grams.
//! Calories/macros are resolved at runtime from IngredientCache.

use super::goal::Goal;

/// Localized text: (ru, pl, uk, en)
#[derive(Debug, Clone)]
pub struct L10n(pub &'static str, pub &'static str, pub &'static str, pub &'static str);

impl L10n {
    pub fn pick(&self, lang: &str) -> String {
        match lang {
            "ru" => self.0, "pl" => self.1, "uk" => self.2, _ => self.3,
        }.to_string()
    }
}

/// A single ingredient pick: slug from catalog + grams.
#[derive(Debug, Clone)]
pub struct IngredientPick {
    pub slug: &'static str,
    pub grams: f32,
    pub labels: L10n,
    pub amounts: L10n,
}

/// One variant strategy (light / balanced / rich).
#[derive(Debug, Clone)]
pub struct MealStrategy {
    pub picks: Vec<IngredientPick>,
    pub level: &'static str,
    pub emoji: &'static str,
    pub titles: L10n,
    pub descs: L10n,
}

/// Shorthand: ingredient from catalog.
fn p(slug: &'static str, grams: f32, labels: L10n, amounts: L10n) -> IngredientPick {
    IngredientPick { slug, grams, labels, amounts }
}

/// Pick strategies for a given goal.
pub fn strategies_for(goal: Goal) -> Vec<MealStrategy> {
    match goal {
        Goal::WeightLoss => weight_loss(),
        Goal::HighProtein => high_protein(),
        Goal::QuickBreakfast => quick_breakfast(),
        Goal::FromIngredients | Goal::HealthyDay | Goal::Generic => generic(),
    }
}

// ── Weight Loss ───────────────────────────────────────────────────────────────

fn weight_loss() -> Vec<MealStrategy> {
    vec![
        MealStrategy {
            level: "light", emoji: "🟢",
            titles: L10n("Лёгкий", "Lekki", "Легкий", "Light"),
            descs: L10n("овсянка + банан",
                        "owsianka + banan",
                        "вівсянка + банан",
                        "oatmeal + banana"),
            picks: vec![
                p("oatmeal", 60.0,
                  L10n("Овсянка", "Płatki owsiane", "Вівсянка", "Oatmeal"),
                  L10n("60г", "60g", "60г", "60g")),
                p("banana", 120.0,
                  L10n("Банан", "Banan", "Банан", "Banana"),
                  L10n("1 шт", "1 szt", "1 шт", "1 pc")),
            ],
        },
        MealStrategy {
            level: "balanced", emoji: "🟡",
            titles: L10n("Баланс", "Balans", "Баланс", "Balanced"),
            descs: L10n("курица + рис + брокколи",
                        "kurczak + ryż + brokuły",
                        "курка + рис + броколі",
                        "chicken + rice + broccoli"),
            picks: vec![
                p("chicken-breast", 150.0,
                  L10n("Куриная грудка", "Pierś z kurczaka", "Куряче філе", "Chicken breast"),
                  L10n("150г", "150g", "150г", "150g")),
                p("rice", 80.0,
                  L10n("Рис", "Ryż", "Рис", "Rice"),
                  L10n("80г", "80g", "80г", "80g")),
                p("broccoli", 150.0,
                  L10n("Брокколи", "Brokuły", "Броколі", "Broccoli"),
                  L10n("150г", "150g", "150г", "150g")),
            ],
        },
        MealStrategy {
            level: "rich", emoji: "🔴",
            titles: L10n("Сытный", "Sycący", "Ситний", "Hearty"),
            descs: L10n("лосось + киноа + авокадо",
                        "łosoś + quinoa + awokado",
                        "лосось + кіноа + авокадо",
                        "salmon + quinoa + avocado"),
            picks: vec![
                p("salmon", 150.0,
                  L10n("Лосось", "Łosoś", "Лосось", "Salmon"),
                  L10n("150г", "150g", "150г", "150g")),
                p("quinoa", 80.0,
                  L10n("Киноа", "Quinoa", "Кіноа", "Quinoa"),
                  L10n("80г", "80g", "80г", "80g")),
                p("avocado", 70.0,
                  L10n("Авокадо", "Awokado", "Авокадо", "Avocado"),
                  L10n("½ шт", "½ szt", "½ шт", "½ pc")),
            ],
        },
    ]
}

// ── High Protein ──────────────────────────────────────────────────────────────

fn high_protein() -> Vec<MealStrategy> {
    vec![
        MealStrategy {
            level: "light", emoji: "🟢",
            titles: L10n("Лёгкий", "Lekki", "Легкий", "Light"),
            descs: L10n("творог + ягоды + орехи",
                        "twaróg + jagody + orzechy",
                        "сир + ягоди + горіхи",
                        "cottage cheese + berries + nuts"),
            picks: vec![
                p("cottage-cheese", 200.0,
                  L10n("Творог 5%", "Twaróg", "Сир кисломолочний", "Cottage cheese"),
                  L10n("200г", "200g", "200г", "200g")),
                p("blueberry", 100.0,
                  L10n("Ягоды", "Jagody", "Ягоди", "Mixed berries"),
                  L10n("100г", "100g", "100г", "100g")),
                p("walnuts", 20.0,
                  L10n("Грецкие орехи", "Orzechy włoskie", "Волоські горіхи", "Walnuts"),
                  L10n("20г", "20g", "20г", "20g")),
            ],
        },
        MealStrategy {
            level: "balanced", emoji: "🟡",
            titles: L10n("Баланс", "Balans", "Баланс", "Balanced"),
            descs: L10n("курица + гречка + брокколи",
                        "kurczak + kasza gryczana + brokuły",
                        "курка + гречка + броколі",
                        "chicken + buckwheat + broccoli"),
            picks: vec![
                p("chicken-breast", 180.0,
                  L10n("Куриная грудка", "Pierś z kurczaka", "Куряче філе", "Chicken breast"),
                  L10n("180г", "180g", "180г", "180g")),
                p("buckwheat", 80.0,
                  L10n("Гречка", "Kasza gryczana", "Гречка", "Buckwheat"),
                  L10n("80г", "80g", "80г", "80g")),
                p("broccoli", 120.0,
                  L10n("Брокколи", "Brokuły", "Броколі", "Broccoli"),
                  L10n("120г", "120g", "120г", "120g")),
            ],
        },
        MealStrategy {
            level: "rich", emoji: "🔴",
            titles: L10n("Сытный", "Sycący", "Ситній", "Hearty"),
            descs: L10n("стейк + батат + шпинат",
                        "stek wołowy + batat + szpinak",
                        "стейк + батат + шпінат",
                        "beef steak + sweet potato + spinach"),
            picks: vec![
                p("beef", 200.0,
                  L10n("Говяжий стейк", "Stek wołowy", "Яловичий стейк", "Beef steak"),
                  L10n("200г", "200g", "200г", "200g")),
                p("potato", 150.0,
                  L10n("Батат", "Batat", "Батат", "Sweet potato"),
                  L10n("150г", "150g", "150г", "150g")),
                p("spinach", 100.0,
                  L10n("Шпинат", "Szpinak", "Шпінат", "Spinach"),
                  L10n("100г", "100g", "100г", "100g")),
            ],
        },
    ]
}

// ── Quick Breakfast ───────────────────────────────────────────────────────────

fn quick_breakfast() -> Vec<MealStrategy> {
    vec![
        MealStrategy {
            level: "light", emoji: "🟢",
            titles: L10n("Экспресс", "Ekspres", "Експрес", "Express"),
            descs: L10n("тост + банан",
                        "tost + banan",
                        "тост + банан",
                        "toast + banana"),
            picks: vec![
                p("bread", 60.0,
                  L10n("Тост", "Tost", "Тост", "Toast"),
                  L10n("2 шт", "2 szt", "2 шт", "2 pcs")),
                p("banana", 120.0,
                  L10n("Банан", "Banan", "Банан", "Banana"),
                  L10n("1 шт", "1 szt", "1 шт", "1 pc")),
            ],
        },
        MealStrategy {
            level: "balanced", emoji: "🟡",
            titles: L10n("Классика", "Klasyk", "Класика", "Classic"),
            descs: L10n("яичница + хлеб + помидор",
                        "jajecznica + chleb + pomidor",
                        "яєчня + хліб + помідор",
                        "eggs + bread + tomato"),
            picks: vec![
                p("chicken-eggs", 100.0,
                  L10n("Яйца", "Jajka", "Яйця", "Eggs"),
                  L10n("2 шт", "2 szt", "2 шт", "2 pcs")),
                p("bread", 60.0,
                  L10n("Хлеб ржаной", "Chleb żytni", "Хліб житній", "Rye bread"),
                  L10n("2 ломтика", "2 kromki", "2 скибочки", "2 slices")),
                p("tomato", 100.0,
                  L10n("Помидор", "Pomidor", "Помідор", "Tomato"),
                  L10n("1 шт", "1 szt", "1 шт", "1 pc")),
            ],
        },
        MealStrategy {
            level: "rich", emoji: "🔴",
            titles: L10n("Сытный", "Sycący", "Ситній", "Hearty"),
            descs: L10n("сырники + мёд",
                        "serniczki + miód",
                        "сирники + мед",
                        "pancakes + honey"),
            picks: vec![
                p("cottage-cheese", 200.0,
                  L10n("Сырники", "Serniczki", "Сирники", "Cottage cheese pancakes"),
                  L10n("3 шт", "3 szt", "3 шт", "3 pcs")),
                p("honey", 10.0,
                  L10n("Мёд", "Miód", "Мед", "Honey"),
                  L10n("1 ч.л.", "1 łyżeczka", "1 ч.л.", "1 tsp")),
            ],
        },
    ]
}

// ── Generic / Healthy Day ─────────────────────────────────────────────────────

fn generic() -> Vec<MealStrategy> {
    vec![
        MealStrategy {
            level: "light", emoji: "🟢",
            titles: L10n("Лёгкий", "Lekki", "Легкий", "Light"),
            descs: L10n("тунец + салат + оливковое масло",
                        "tuńczyk + sałatka + oliwa",
                        "тунець + салат + олія",
                        "tuna + salad + olive oil"),
            picks: vec![
                p("canned-tuna", 120.0,
                  L10n("Тунец", "Tuńczyk", "Тунець", "Canned tuna"),
                  L10n("120г", "120g", "120г", "120g")),
                p("lettuce", 100.0,
                  L10n("Микс салат", "Mix sałat", "Мікс салат", "Mixed greens"),
                  L10n("100г", "100g", "100г", "100g")),
                p("olive-oil", 13.0,
                  L10n("Оливковое масло", "Oliwa z oliwek", "Оливкова олія", "Olive oil"),
                  L10n("1 ст.л.", "1 łyżka", "1 ст.л.", "1 tbsp")),
                p("tomato", 100.0,
                  L10n("Помидоры черри", "Pomidorki", "Помідори черрі", "Cherry tomatoes"),
                  L10n("100г", "100g", "100г", "100g")),
            ],
        },
        MealStrategy {
            level: "balanced", emoji: "🟡",
            titles: L10n("Баланс", "Balans", "Баланс", "Balanced"),
            descs: L10n("паста + фарш + томатный соус",
                        "makaron + mielone + sos",
                        "паста + фарш + соус",
                        "pasta + ground meat + sauce"),
            picks: vec![
                p("pasta", 100.0,
                  L10n("Паста", "Makaron", "Паста", "Pasta"),
                  L10n("100г", "100g", "100г", "100g")),
                p("ground-meat", 120.0,
                  L10n("Фарш", "Mielone", "Фарш", "Ground meat"),
                  L10n("120г", "120g", "120г", "120g")),
                p("canned-tomatoes", 80.0,
                  L10n("Томатный соус", "Sos pomidorowy", "Томатний соус", "Tomato sauce"),
                  L10n("80г", "80g", "80г", "80g")),
            ],
        },
        MealStrategy {
            level: "rich", emoji: "🔴",
            titles: L10n("Праздничный", "Na okazję", "Святковий", "Celebration"),
            descs: L10n("стейк + картофель + грибы",
                        "stek + ziemniaki + grzyby",
                        "стейк + картопля + гриби",
                        "steak + potatoes + mushrooms"),
            picks: vec![
                p("beef", 200.0,
                  L10n("Стейк", "Stek wołowy", "Стейк", "Beef steak"),
                  L10n("200г", "200g", "200г", "200g")),
                p("potato", 200.0,
                  L10n("Картофель", "Ziemniaki", "Картопля", "Potatoes"),
                  L10n("200г", "200g", "200г", "200g")),
                p("button-mushroom", 50.0,
                  L10n("Грибы", "Grzyby", "Гриби", "Mushrooms"),
                  L10n("50г", "50g", "50г", "50g")),
            ],
        },
    ]
}
