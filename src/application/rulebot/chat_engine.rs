//! ChefOS Chat Engine — handle_chat() logic.
//!
//! Entry point: `POST /public/chat`
//!
//! Flow:
//! 1. parse_input(input)          — intent + multi-intents + goal modifier
//! 2. detect_language(input)      — which language?
//! 3. session follow-up check     — "а сколько в нём калорий?" → last product
//! 4. dispatch to handler(s)      — rule-based, IngredientCache::all(), or LLM
//! 5. return ChatResponse         — text + card + reason + intents + context

use std::sync::Arc;
use std::time::Instant;

use crate::infrastructure::IngredientCache;
use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::domain::tools::unit_converter as uc;
use super::intent_router::{detect_language, parse_input, ChatLang, HealthModifier, Intent};
use super::chat_response::{Card, ChatResponse, ConversionCard, NutritionCard, ProductCard, Suggestion};
use super::session_context::SessionContext;
use super::ai_brain::AiBrain;

// ── Health goal (maps from HealthModifier + input context) ────────────────────

#[derive(Debug, Clone, Copy)]
enum HealthGoal {
    HighProtein,
    LowCalorie,
    Balanced,
}

impl HealthGoal {
    fn from_modifier(modifier: HealthModifier, input: &str) -> Self {
        match modifier {
            HealthModifier::HighProtein => Self::HighProtein,
            HealthModifier::LowCalorie  => Self::LowCalorie,
            _ => {
                // Fallback: scan input for contextual hints
                let t = input.to_lowercase();
                if t.contains("белок") || t.contains("протеин") || t.contains("мышц")
                    || t.contains("protein") || t.contains("muscle") || t.contains("białk")
                {
                    Self::HighProtein
                } else if t.contains("похуд") || t.contains("диет") || t.contains("diet")
                    || t.contains("lose weight") || t.contains("slim") || t.contains("сушк")
                {
                    Self::LowCalorie
                } else {
                    Self::Balanced
                }
            }
        }
    }
}

// ── Engine ───────────────────────────────────────────────────────────────────

pub struct ChatEngine {
    ingredient_cache: Arc<IngredientCache>,
    llm_adapter: Arc<LlmAdapter>,
    ai_brain: AiBrain,
}

impl ChatEngine {
    pub fn new(ingredient_cache: Arc<IngredientCache>, llm_adapter: Arc<LlmAdapter>) -> Self {
        let ai_brain = AiBrain::new(Arc::clone(&ingredient_cache), Arc::clone(&llm_adapter));
        Self { ingredient_cache, llm_adapter, ai_brain }
    }

    /// Main entry point — takes free-text + optional session context, returns ChatResponse.
    pub async fn handle_chat(&self, input: &str) -> ChatResponse {
        self.handle_chat_with_context(input, &SessionContext::new()).await
    }

    /// Extended entry with session context — enables follow-ups and modifier persistence.
    pub async fn handle_chat_with_context(
        &self,
        input: &str,
        ctx: &SessionContext,
    ) -> ChatResponse {
        let start = Instant::now();
        let lang = detect_language(input);
        let parsed = parse_input(input);

        // Effective modifier: current OR remembered from last turn
        let modifier = ctx.effective_modifier(parsed.modifier);
        let goal = HealthGoal::from_modifier(modifier, input);

        tracing::debug!(
            "💬 chat: intent={:?} intents={:?} modifier={:?} lang={:?} turn={}",
            parsed.intent, parsed.intents, modifier, lang, ctx.turn_count
        );

        // ── Follow-up resolution ──────────────────────────────────────────────
        // "а сколько в нём калорий?" refers to last product in context
        if ctx.is_followup(input) {
            if let Some(slug) = &ctx.last_product_slug {
                if let Some(p) = self.ingredient_cache.get(slug).await {
                    let name = p.name(lang.code()).to_string();
                    let text = format_nutrition_text(&name, &p, lang);
                    let mut resp = ChatResponse::with_card(
                        text,
                        Card::Nutrition(NutritionCard {
                            name,
                            calories_per_100g: p.calories_per_100g,
                            protein_per_100g: p.protein_per_100g,
                            fat_per_100g: p.fat_per_100g,
                            carbs_per_100g: p.carbs_per_100g,
                            image_url: p.image_url.clone(),
                        }),
                        Intent::NutritionInfo,
                        lang,
                        0,
                    );
                    resp.timing_ms = start.elapsed().as_millis() as u64;
                    return resp;
                }
            }
        }

        // ── Primary dispatch ──────────────────────────────────────────────────
        let mut response = match parsed.intent {
            Intent::Greeting       => self.handle_greeting(lang),
            Intent::HealthyProduct => self.handle_healthy_product(input, lang, goal, ctx).await,
            Intent::Conversion     => self.handle_conversion(input, lang),
            Intent::NutritionInfo  => self.handle_nutrition(input, lang).await,
            Intent::Seasonality    => self.handle_seasonality(input, lang),
            Intent::RecipeHelp     => self.handle_recipe(input, lang),
            Intent::MealIdea       => self.handle_meal_idea(lang, goal).await,
            Intent::ProductInfo    => self.handle_product_info(input, lang).await,
            // ── Layer 2: AI Brain ── LLM with tool calling for complex queries
            Intent::Unknown        => {
                tracing::info!("🧠 Escalating to AI Brain (Layer 2) for: {:?}", &input[..input.len().min(60)]);
                self.ai_brain.handle(input, lang, ctx).await
            }
        };

        // Attach multi-intent list to response (frontend can use for multi-card UI)
        if parsed.intents.len() > 1 {
            response.intents = parsed.intents;
        }

        response.timing_ms = start.elapsed().as_millis() as u64;
        response
    }

    // ── Handlers ─────────────────────────────────────────────────────────────

    fn handle_greeting(&self, lang: ChatLang) -> ChatResponse {
        let text = match lang {
            ChatLang::Ru => "Привет 👋 Я ChefOS — твой кулинарный помощник! Спроси меня:\n• «что полезного поесть»\n• «сколько калорий в шпинате»\n• «200 грамм в ложках»\n• «что приготовить на ужин»",
            ChatLang::En => "Hello 👋 I'm ChefOS — your culinary assistant! Ask me:\n• \"healthy product ideas\"\n• \"calories in spinach\"\n• \"convert 200g to tablespoons\"\n• \"dinner idea\"",
            ChatLang::Pl => "Cześć 👋 Jestem ChefOS — Twoim kulinarnym asystentem! Zapytaj mnie:\n• «zdrowy produkt»\n• «kalorie szpinaku»\n• «200 gramów na łyżki»\n• «co ugotować na obiad»",
            ChatLang::Uk => "Привіт 👋 Я ChefOS — твій кулінарний помічник! Запитай мене:\n• «що корисного з'їсти»\n• «калорії шпинату»\n• «200 грамів в ложках»\n• «що приготувати на вечерю»",
        };
        ChatResponse::text_only(text, Intent::Greeting, lang, 0)
    }

    async fn handle_healthy_product(&self, input: &str, lang: ChatLang, goal: HealthGoal, ctx: &SessionContext) -> ChatResponse {
        let exclude = ctx.excluded_slugs();
        let products = self.select_top_products(goal, 3, exclude).await;

        if products.is_empty() {
            return ChatResponse::text_only(fallback_healthy_text(lang), Intent::HealthyProduct, lang, 0);
        }

        // Build cards for all selected products
        let cards: Vec<Card> = products.iter().map(|(p, reason_tag, _)| {
            let name = p.name(lang.code()).to_string();
            let highlight = pick_highlight(p, lang, goal);
            Card::Product(ProductCard {
                slug: p.slug.clone(),
                name,
                calories_per_100g: p.calories_per_100g,
                protein_per_100g: p.protein_per_100g,
                fat_per_100g: p.fat_per_100g,
                carbs_per_100g: p.carbs_per_100g,
                image_url: p.image_url.clone(),
                highlight: Some(highlight),
                reason_tag: Some(reason_tag),
            })
        }).collect();

        // Use the top pick for text
        let (top_p, top_tag, _top_reason) = &products[0];
        let top_name = top_p.name(lang.code()).to_string();
        let top_highlight = pick_highlight(top_p, lang, goal);
        let text = format_healthy_text(&top_name, top_p, lang, &top_highlight, goal);

        // Meaningful reason: macro summary, not score numbers
        let combined_reason = format_macro_summary(top_p, lang, goal, products.len());

        // Suppress unused warning
        let _ = top_tag;

        let mut resp = ChatResponse::with_cards(
            text,
            cards,
            Intent::HealthyProduct,
            vec![],
            combined_reason,
            lang,
            0,
        );

        // ── Suggestions (next-step actions) ──
        resp.suggestions = build_healthy_suggestions(lang, goal, &top_name);

        // ── Chef tip ──
        resp.chef_tip = Some(pick_chef_tip(top_p, lang, goal));

        resp
    }

    /// Scan ALL ingredients from cache, rank by weighted normalized score.
    /// Returns up to `n` diverse products, excluding `exclude_slugs`.
    /// Uses 80/20 exploration: 20% of turns pick random from top-10.
    async fn select_top_products(
        &self,
        goal: HealthGoal,
        n: usize,
        exclude_slugs: &[String],
    ) -> Vec<(crate::infrastructure::ingredient_cache::IngredientData, &'static str, String)> {
        let all = self.ingredient_cache.all().await;
        if all.is_empty() {
            return vec![];
        }

        // Filter valid + non-excluded products
        let candidates: Vec<_> = all.into_iter()
            .filter(|p| (p.calories_per_100g > 0.0 || p.protein_per_100g > 0.0)
                && !exclude_slugs.contains(&p.slug))
            .collect();

        if candidates.is_empty() {
            return vec![];
        }

        // ── Step 1: Compute min/max for normalization ──────────────────────
        let max_protein = candidates.iter().map(|p| p.protein_per_100g).fold(0.0_f32, f32::max);
        let min_protein = candidates.iter().map(|p| p.protein_per_100g).fold(f32::MAX, f32::min);
        let max_cal = candidates.iter().map(|p| p.calories_per_100g).fold(0.0_f32, f32::max);
        let min_cal = candidates.iter().map(|p| p.calories_per_100g).fold(f32::MAX, f32::min);
        let max_fat = candidates.iter().map(|p| p.fat_per_100g).fold(0.0_f32, f32::max);
        let min_fat = candidates.iter().map(|p| p.fat_per_100g).fold(f32::MAX, f32::min);

        let norm = |x: f32, lo: f32, hi: f32| -> f64 {
            let range = (hi - lo) as f64;
            if range < 1e-6 { 0.5 } else { ((x - lo) as f64) / range }
        };

        // ── Step 2: Score each candidate ──────────────────────────────────
        let mut scored: Vec<(f64, crate::infrastructure::ingredient_cache::IngredientData)> = candidates
            .into_iter()
            .map(|p| {
                let np = norm(p.protein_per_100g, min_protein, max_protein);
                let nc = norm(p.calories_per_100g, min_cal, max_cal);
                let nf = norm(p.fat_per_100g, min_fat, max_fat);
                // density bonus: high protein AND low cal
                let density_bonus: f64 = if p.protein_per_100g > 15.0 && p.calories_per_100g < 200.0 { 0.1 } else { 0.0 };

                let score: f64 = match goal {
                    HealthGoal::HighProtein => 0.60 * np + 0.20 * (1.0 - nc) + 0.10 * nf + density_bonus,
                    HealthGoal::LowCalorie  => 0.10 * np + 0.60 * (1.0 - nc) + 0.20 * nf + density_bonus,
                    HealthGoal::Balanced    => 0.35 * np + 0.35 * (1.0 - nc) + 0.20 * nf + density_bonus,
                };
                (score, p)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // ── Step 3: 80/20 exploration on the top pick ─────────────────────
        // Use turn_count proxy: day-of-second mod 5 == 0 triggers exploration
        let top10_len = scored.len().min(10);
        let explore = {
            use std::time::{SystemTime, UNIX_EPOCH};
            let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            // 20% of turns = every 5th second bucket
            secs % 5 == 0
        };

        if explore && top10_len > 1 {
            // Swap a random top-10 product to position 0
            use std::time::{SystemTime, UNIX_EPOCH};
            let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            let pick = (secs as usize % top10_len).max(1); // never pick 0 (already top)
            scored.swap(0, pick);
        }

        // ── Step 4: Diversity — pick top-n with no repeated category ──────
        let mut result = Vec::with_capacity(n);
        let mut seen_categories: Vec<&'static str> = Vec::new();

        // Apply diversity penalty (0.5x) to same-category items already seen,
        // then re-rank and pick greedily
        let category_of = |slug: &str| -> &'static str {
            let fish = ["salmon", "tuna", "cod", "herring", "mackerel", "trout", "sardine"];
            let veg  = ["spinach", "broccoli", "kale", "carrot", "tomato", "cucumber", "beet", "celery", "asparagus", "zucchini", "peppers"];
            let meat = ["chicken", "beef", "pork", "lamb", "turkey", "duck"];
            let dairy= ["milk", "cheese", "butter", "yogurt", "cream", "eggs"];
            let grain= ["rice", "oat", "quinoa", "wheat", "pasta", "bread", "corn"];
            let nut  = ["almond", "walnut", "cashew", "pecan", "hazelnut", "peanut", "pistachio"];
            let fruit= ["apple", "banana", "orange", "mango", "berry", "blueberr", "strawberr", "lemon", "avocado"];
            let slug_l = slug.to_lowercase();
            if fish.iter().any(|k| slug_l.contains(k)) { return "fish"; }
            if veg.iter().any(|k| slug_l.contains(k))  { return "vegetable"; }
            if meat.iter().any(|k| slug_l.contains(k)) { return "meat"; }
            if dairy.iter().any(|k| slug_l.contains(k)){ return "dairy"; }
            if grain.iter().any(|k| slug_l.contains(k)){ return "grain"; }
            if nut.iter().any(|k| slug_l.contains(k))  { return "nut"; }
            if fruit.iter().any(|k| slug_l.contains(k)){ return "fruit"; }
            "other"
        };

        // Re-score with diversity penalty applied, then pick greedily
        let mut pool = scored;
        while result.len() < n && !pool.is_empty() {
            // Apply current diversity penalties
            let best_idx = pool.iter().enumerate().map(|(i, (raw_score, p))| {
                let cat = category_of(&p.slug);
                let penalty = if seen_categories.contains(&cat) { 0.5 } else { 1.0 };
                (i, raw_score * penalty)
            }).max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);

            let (score, p) = pool.remove(best_idx);
            let cat = category_of(&p.slug);
            if !seen_categories.contains(&cat) {
                seen_categories.push(cat);
            }

            // Build explainability
            let reason_idx = result.len() + 1;
            let (reason_tag, reason_text) = match goal {
                HealthGoal::HighProtein => (
                    "high_protein",
                    format!("protein: {:.1}g/100g, score {:.2} (rank #{})", p.protein_per_100g, score, reason_idx),
                ),
                HealthGoal::LowCalorie => (
                    "low_calorie",
                    format!("only {} kcal/100g, score {:.2} (rank #{})", p.calories_per_100g as i32, score, reason_idx),
                ),
                HealthGoal::Balanced => (
                    "balanced",
                    format!("balanced score {:.2}: {:.1}g protein, {} kcal/100g (rank #{})", score, p.protein_per_100g, p.calories_per_100g as i32, reason_idx),
                ),
            };

            result.push((p, reason_tag, reason_text));
        }

        result
    }

    /// Kept as single-product convenience wrapper (used by meal idea + follow-up).
    async fn select_healthy_product(
        &self,
        goal: HealthGoal,
    ) -> Option<(crate::infrastructure::ingredient_cache::IngredientData, &'static str, String)> {
        self.select_top_products(goal, 1, &[]).await.into_iter().next()
    }

    fn handle_conversion(&self, input: &str, lang: ChatLang) -> ChatResponse {
        // Try to extract a number + from/to units from text
        if let Some((value, from, to)) = extract_conversion(input) {
            let result_raw = uc::convert_units(value, &from, &to);
            let supported = result_raw.is_some();
            let result = uc::display_round(result_raw.unwrap_or(0.0));

            let text = format_conversion_text(value, &from, result, &to, supported, lang);

            return ChatResponse::with_card(
                text,
                Card::Conversion(ConversionCard {
                    value,
                    from,
                    to,
                    result,
                    supported,
                }),
                Intent::Conversion,
                lang,
                0,
            );
        }

        // Couldn't parse — guide the user
        let text = match lang {
            ChatLang::Ru => "Напиши, например: «переведи 200 грамм в унции» или «сколько ложек в 100 мл»",
            ChatLang::En => "Try something like: \"convert 200 grams to ounces\" or \"how many tablespoons in 100ml\"",
            ChatLang::Pl => "Spróbuj: «przelicz 200 gramów na uncje» lub «ile łyżek to 100 ml»",
            ChatLang::Uk => "Напиши, наприклад: «переведи 200 грамів в унції» або «скільки ложок в 100 мл»",
        };
        ChatResponse::text_only(text, Intent::Conversion, lang, 0)
    }

    async fn handle_nutrition(&self, input: &str, lang: ChatLang) -> ChatResponse {
        // Extract product slug from text by scanning cache
        let matched = self.find_ingredient_in_text(input).await;

        if let Some(p) = matched {
            let name = p.name(lang.code()).to_string();
            let text = format_nutrition_text(&name, &p, lang);
            return ChatResponse::with_card(
                text,
                Card::Nutrition(NutritionCard {
                    name,
                    calories_per_100g: p.calories_per_100g,
                    protein_per_100g: p.protein_per_100g,
                    fat_per_100g: p.fat_per_100g,
                    carbs_per_100g: p.carbs_per_100g,
                    image_url: p.image_url.clone(),
                }),
                Intent::NutritionInfo,
                lang,
                0,
            );
        }

        let text = match lang {
            ChatLang::Ru => "Укажи продукт — например: «калории шпината» или «белок в курице»",
            ChatLang::En => "Tell me which product — e.g. \"calories in spinach\" or \"protein in chicken\"",
            ChatLang::Pl => "Podaj produkt — np. «kalorie szpinaku» lub «białko w kurczaku»",
            ChatLang::Uk => "Вкажи продукт — наприклад: «калорії шпинату» або «білок у курці»",
        };
        ChatResponse::text_only(text, Intent::NutritionInfo, lang, 0)
    }

    fn handle_seasonality(&self, input: &str, lang: ChatLang) -> ChatResponse {
        // Detect fish/product keywords
        let text_lower = input.to_lowercase();
        let season_hint = detect_season_product(&text_lower);

        let text = match season_hint {
            Some(product) => format_season_text(product, lang),
            None => match lang {
                ChatLang::Ru => "Спроси о конкретном продукте — например: «сезон лосося» или «когда клубника в сезоне»".to_string(),
                ChatLang::En => "Ask about a specific product — e.g. \"salmon season\" or \"when are strawberries in season\"".to_string(),
                ChatLang::Pl => "Zapytaj o konkretny produkt — np. «sezon łososia» lub «kiedy truskawki są w sezonie»".to_string(),
                ChatLang::Uk => "Запитай про конкретний продукт — наприклад: «сезон лосося» або «коли полуниця в сезоні»".to_string(),
            },
        };

        ChatResponse::text_only(text, Intent::Seasonality, lang, 0)
    }

    fn handle_recipe(&self, input: &str, lang: ChatLang) -> ChatResponse {
        let text_lower = input.to_lowercase();

        // Detect dish keywords
        let dish = detect_dish_keyword(&text_lower);

        let text = match dish {
            Some(name) => format_recipe_hint(name, lang),
            None => match lang {
                ChatLang::Ru => "Для рецептов перейди в раздел «Рецепты» — там можно найти рецепты с подробными шагами, калориями и стоимостью ингредиентов 🍳".to_string(),
                ChatLang::En => "For recipes, visit the \"Recipes\" section — you'll find step-by-step instructions, nutrition info and ingredient costs 🍳".to_string(),
                ChatLang::Pl => "Po przepisy przejdź do sekcji «Przepisy» — znajdziesz tam krok po kroku instrukcje, kalorie i ceny składników 🍳".to_string(),
                ChatLang::Uk => "Для рецептів перейди до розділу «Рецепти» — там є покрокові інструкції, калорії та вартість інгредієнтів 🍳".to_string(),
            },
        };

        ChatResponse::text_only(text, Intent::RecipeHelp, lang, 0)
    }

    async fn handle_meal_idea(&self, lang: ChatLang, goal: HealthGoal) -> ChatResponse {
        // Goal-aware meal tables — HighProtein/LowCalorie variants included
        let ideas_ru: &[(&str, &str, &str)] = match goal {
            HealthGoal::HighProtein => &[
                ("Куриная грудка с киноа", "chicken-breast", "Высокобелковое блюдо: ~50г белка на порцию. Квиноа — полный аминокислотный профиль."),
                ("Тунец с яйцами", "tuna", "Силовой завтрак. Тунец + яйца = ~40г белка за 10 минут готовки."),
                ("Лосось с брокколи", "salmon", "Омега-3 + белок. Запекается 20 мин — идеально после тренировки."),
            ],
            HealthGoal::LowCalorie => &[
                ("Шпинатный салат", "spinach", "Всего ~25 ккал на 100г. Много железа и витамина K."),
                ("Суп из брокколи", "broccoli", "Сытный и лёгкий — около 120 ккал на порцию."),
                ("Греческий йогурт с ягодами", "blueberries", "Белок + клетчатка без лишних калорий."),
            ],
            HealthGoal::Balanced => &[
                ("Паста с курицей и шпинатом", "chicken-breast", "Быстрый и сытный ужин — готовится за 20 минут."),
                ("Омлет с овощами", "eggs", "Идеальный завтрак. Яйца — белок и витамин D."),
                ("Греческий салат", "broccoli", "Свежо, легко, вкусно — средиземноморская классика."),
                ("Куриный суп", "chicken-breast", "Согревающий и питательный. Богат коллагеном."),
                ("Запечённый лосось с овощами", "salmon", "Лосось богат омега-3. Готовится 20 мин."),
            ],
        };
        let ideas_en: &[(&str, &str, &str)] = match goal {
            HealthGoal::HighProtein => &[
                ("Chicken & Quinoa Bowl", "chicken-breast", "High-protein meal: ~50g protein per serving. Quinoa has a complete amino acid profile."),
                ("Tuna & Egg Power Bowl", "tuna", "Strength breakfast: tuna + eggs = ~40g protein in 10 min."),
                ("Baked Salmon with Broccoli", "salmon", "Omega-3 + protein. Bakes in 20 min — perfect post-workout."),
            ],
            HealthGoal::LowCalorie => &[
                ("Spinach Salad", "spinach", "Only ~25 kcal/100g. High iron and vitamin K."),
                ("Broccoli Soup", "broccoli", "Filling and light — about 120 kcal per serving."),
                ("Greek Yogurt with Berries", "blueberries", "Protein + fiber without excess calories."),
            ],
            HealthGoal::Balanced => &[
                ("Chicken & Spinach Pasta", "chicken-breast", "Quick and filling — ready in 20 minutes."),
                ("Veggie Omelette", "eggs", "Perfect breakfast. Eggs: protein and vitamin D."),
                ("Greek Salad", "broccoli", "Fresh, light, delicious — Mediterranean classic."),
                ("Chicken Soup", "chicken-breast", "Warming and nutritious. Rich in collagen."),
                ("Baked Salmon with Vegetables", "salmon", "Rich in omega-3. Ready in 20 min."),
            ],
        };
        let ideas_pl: &[(&str, &str, &str)] = match goal {
            HealthGoal::HighProtein => &[
                ("Kurczak z quinoa", "chicken-breast", "Dużo białka: ~50g na porcję."),
                ("Tuńczyk z jajkami", "tuna", "Śniadanie siłowe: tuńczyk + jajka = ~40g białka."),
                ("Łosoś z brokułami", "salmon", "Omega-3 + białko. Pieczenie 20 min."),
            ],
            HealthGoal::LowCalorie => &[
                ("Sałatka ze szpinaku", "spinach", "Tylko ~25 kcal/100g. Dużo żelaza."),
                ("Zupa z brokułów", "broccoli", "Sycąca i lekka — ok. 120 kcal na porcję."),
                ("Jogurt grecki z jagodami", "blueberries", "Białko + błonnik bez nadmiaru kalorii."),
            ],
            HealthGoal::Balanced => &[
                ("Makaron z kurczakiem i szpinakiem", "chicken-breast", "Szybki i sycący — gotowy w 20 minut."),
                ("Omlet z warzywami", "eggs", "Idealne śniadanie. Jajka: białko i witamina D."),
                ("Sałatka grecka", "broccoli", "Świeżo i smacznie — śródziemnomorski klasyk."),
                ("Zupa z kurczaka", "chicken-breast", "Rozgrzewająca. Bogata w kolagen."),
                ("Pieczony łosoś z warzywami", "salmon", "Bogaty w omega-3. Pieczenie 20 min."),
            ],
        };

        let hour = chrono::Utc::now().hour() as usize;
        let ideas = match lang {
            ChatLang::Ru => ideas_ru,
            ChatLang::En => ideas_en,
            ChatLang::Pl | ChatLang::Uk => ideas_pl,
        };
        let (meal_name, slug, description) = ideas[hour % ideas.len()];

        if let Some(p) = self.ingredient_cache.get(slug).await {
            let ingredient_name = p.name(lang.code()).to_string();
            let text = match lang {
                ChatLang::Ru => format!("🍽️ Идея на сегодня: **{}**\n\n{}\n\nГлавный ингредиент: {} ({} ккал/100г)", meal_name, description, ingredient_name, p.calories_per_100g as i32),
                ChatLang::En => format!("🍽️ Today's idea: **{}**\n\n{}\n\nMain ingredient: {} ({} kcal/100g)", meal_name, description, ingredient_name, p.calories_per_100g as i32),
                ChatLang::Pl | ChatLang::Uk => format!("🍽️ Pomysł na dziś: **{}**\n\n{}\n\nGłówny składnik: {} ({} kcal/100g)", meal_name, description, ingredient_name, p.calories_per_100g as i32),
            };
            let mut resp = ChatResponse::with_card(
                text,
                Card::Product(ProductCard {
                    slug: p.slug.clone(),
                    name: ingredient_name.clone(),
                    calories_per_100g: p.calories_per_100g,
                    protein_per_100g: p.protein_per_100g,
                    fat_per_100g: p.fat_per_100g,
                    carbs_per_100g: p.carbs_per_100g,
                    image_url: p.image_url.clone(),
                    highlight: None,
                    reason_tag: None,
                }),
                Intent::MealIdea,
                lang,
                0,
            );
            resp.suggestions = build_meal_suggestions(lang, slug);
            resp.chef_tip = Some(pick_chef_tip(&p, lang, goal));
            return resp;
        }

        let text = match lang {
            ChatLang::Ru => format!("🍽️ Идея на сегодня: **{}**\n\n{}", meal_name, description),
            ChatLang::En => format!("🍽️ Today's idea: **{}**\n\n{}", meal_name, description),
            ChatLang::Pl | ChatLang::Uk => format!("🍽️ Pomysł na dziś: **{}**\n\n{}", meal_name, description),
        };
        ChatResponse::text_only(text, Intent::MealIdea, lang, 0)
    }

    fn handle_unknown_static(&self, lang: ChatLang) -> ChatResponse {
        let text = match lang {
            ChatLang::Ru => "Не совсем понял 🤔 Попробуй:\n• «что полезного поесть»\n• «калории в шпинате»\n• «200 грамм в ложках»\n• «что приготовить на ужин»\n• «что такое лосось»",
            ChatLang::En => "I'm not sure what you mean 🤔 Try:\n• \"healthy food ideas\"\n• \"calories in spinach\"\n• \"convert 200g to tablespoons\"\n• \"dinner idea\"\n• \"what is salmon\"",
            ChatLang::Pl => "Nie rozumiem 🤔 Spróbuj:\n• «zdrowy produkt»\n• «kalorie szpinaku»\n• «przelicz 200g na łyżki»\n• «pomysł na obiad»\n• «co to jest szpinak»",
            ChatLang::Uk => "Не зовсім зрозумів 🤔 Спробуй:\n• «що корисного з'їсти»\n• «калорії шпинату»\n• «200 грамів в ложках»\n• «що приготувати на вечерю»\n• «що таке лосось»",
        };
        ChatResponse::text_only(text, Intent::Unknown, lang, 0)
    }

    /// ProductInfo — "что такое лосось", "расскажи о шпинате", "what is chicken"
    async fn handle_product_info(&self, input: &str, lang: ChatLang) -> ChatResponse {
        if let Some(p) = self.find_ingredient_in_text(input).await {
            let name = p.name(lang.code()).to_string();
            let text = format_product_info_text(&name, &p, lang);
            return ChatResponse::with_card(
                text,
                Card::Product(ProductCard {
                    slug: p.slug.clone(),
                    name,
                    calories_per_100g: p.calories_per_100g,
                    protein_per_100g: p.protein_per_100g,
                    fat_per_100g: p.fat_per_100g,
                    carbs_per_100g: p.carbs_per_100g,
                    image_url: p.image_url.clone(),
                    highlight: None,
                    reason_tag: None,
                }),
                Intent::ProductInfo,
                lang,
                0,
            );
        }

        // Product not in cache → try LLM for a short description
        let lang_name = match lang {
            ChatLang::Ru => "Russian",
            ChatLang::En => "English",
            ChatLang::Pl => "Polish",
            ChatLang::Uk => "Ukrainian",
        };
        let prompt = format!(
            "You are ChefOS, a culinary assistant. The user asked: \"{}\". \
            Answer in {} in 2-3 sentences only about the food product mentioned. \
            Include: what it is, key nutrients, typical culinary use. No markdown.",
            input, lang_name
        );
        match self.llm_adapter.groq_raw_request(&prompt, 200).await {
            Ok(text) => ChatResponse::text_only(text, Intent::ProductInfo, lang, 0),
            Err(_) => {
                let fallback = match lang {
                    ChatLang::Ru => "Продукт не найден в базе. Попробуй уточнить название.",
                    ChatLang::En => "Product not found in database. Try rephrasing the name.",
                    ChatLang::Pl => "Produkt nie znaleziony. Spróbuj innej nazwy.",
                    ChatLang::Uk => "Продукт не знайдено. Спробуй уточнити назву.",
                };
                ChatResponse::text_only(fallback, Intent::ProductInfo, lang, 0)
            }
        }
    }

    /// Legacy LLM fallback — replaced by AI Brain (Layer 2).
    /// Kept as a safety fallback in case AI Brain is disabled.
    #[allow(dead_code)]
    async fn handle_fallback_llm(&self, input: &str, lang: ChatLang) -> ChatResponse {
        let lang_name = match lang {
            ChatLang::Ru => "Russian",
            ChatLang::En => "English",
            ChatLang::Pl => "Polish",
            ChatLang::Uk => "Ukrainian",
        };
        let prompt = format!(
            "You are ChefOS, a culinary assistant. The user asked: \"{}\". \
            Reply in {} in 1-3 sentences. \
            ONLY answer if the question is about food, cooking, nutrition, ingredients, or recipes. \
            If it's unrelated to food/cooking, politely say you only help with culinary topics. \
            No markdown, no bullet points.",
            input, lang_name
        );
        match self.llm_adapter.groq_raw_request(&prompt, 250).await {
            Ok(text) => {
                tracing::debug!("🤖 LLM fallback answered: {:?}", &text[..text.len().min(100)]);
                ChatResponse::text_only(text, Intent::Unknown, lang, 0)
            }
            Err(e) => {
                tracing::warn!("⚠️ LLM fallback failed: {}", e);
                self.handle_unknown_static(lang)
            }
        }
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    /// Scan ingredient cache for any product slug/name mentioned in the input.
    async fn find_ingredient_in_text(
        &self,
        input: &str,
    ) -> Option<crate::infrastructure::ingredient_cache::IngredientData> {
        let text_lower = input.to_lowercase();

        // Check common slugs by keyword
        let slug_keywords: &[(&str, &str)] = &[
            ("spinach", "шпинат"),
            ("spinach", "spinach"),
            ("salmon", "лосось"),
            ("salmon", "salmon"),
            ("chicken-breast", "курица"),
            ("chicken-breast", "chicken"),
            ("eggs", "яйцо"),
            ("eggs", "яйца"),
            ("eggs", "eggs"),
            ("broccoli", "брокколи"),
            ("broccoli", "broccoli"),
            ("almonds", "миндаль"),
            ("almonds", "almonds"),
            ("blueberries", "черника"),
            ("blueberries", "blueberries"),
            ("tomatoes", "помидор"),
            ("tomatoes", "tomato"),
            ("potatoes", "картофель"),
            ("potatoes", "картошк"),
            ("potatoes", "potato"),
            ("carrots", "морковь"),
            ("carrots", "carrot"),
            ("onion", "лук"),
            ("onion", "onion"),
            ("garlic", "чеснок"),
            ("garlic", "garlic"),
            ("rice", "рис"),
            ("rice", "rice"),
            ("beef", "говядина"),
            ("beef", "beef"),
            ("pork", "свинина"),
            ("pork", "pork"),
            ("milk", "молоко"),
            ("milk", "milk"),
            ("butter", "масло"),
            ("butter", "butter"),
        ];

        for (slug, keyword) in slug_keywords {
            if text_lower.contains(keyword) {
                if let Some(p) = self.ingredient_cache.get(slug).await {
                    return Some(p);
                }
            }
        }

        None
    }
}

// ── Pure formatting helpers (no async, no IO) ─────────────────────────────────

fn pick_highlight(p: &crate::infrastructure::ingredient_cache::IngredientData, lang: ChatLang, goal: HealthGoal) -> String {
    match goal {
        HealthGoal::HighProtein => match lang {
            ChatLang::Ru => format!("высокий белок — {:.1}г/100г", p.protein_per_100g),
            ChatLang::En => format!("high protein — {:.1}g/100g", p.protein_per_100g),
            ChatLang::Pl => format!("wysokie białko — {:.1}g/100g", p.protein_per_100g),
            ChatLang::Uk => format!("високий білок — {:.1}г/100г", p.protein_per_100g),
        },
        HealthGoal::LowCalorie => match lang {
            ChatLang::Ru => format!("мало калорий — {} ккал/100г", p.calories_per_100g as i32),
            ChatLang::En => format!("low calorie — {} kcal/100g", p.calories_per_100g as i32),
            ChatLang::Pl => format!("mało kalorii — {} kcal/100g", p.calories_per_100g as i32),
            ChatLang::Uk => format!("мало калорій — {} ккал/100г", p.calories_per_100g as i32),
        },
        HealthGoal::Balanced => {
            if p.protein_per_100g >= 20.0 {
                match lang {
                    ChatLang::Ru => format!("высокий белок — {:.1}г/100г", p.protein_per_100g),
                    ChatLang::En => format!("high protein — {:.1}g/100g", p.protein_per_100g),
                    ChatLang::Pl => format!("wysokie białko — {:.1}g/100g", p.protein_per_100g),
                    ChatLang::Uk => format!("високий білок — {:.1}г/100г", p.protein_per_100g),
                }
            } else if p.calories_per_100g < 50.0 {
                match lang {
                    ChatLang::Ru => format!("мало калорий — {} ккал/100г", p.calories_per_100g as i32),
                    ChatLang::En => format!("low calorie — {} kcal/100g", p.calories_per_100g as i32),
                    ChatLang::Pl => format!("mało kalorii — {} kcal/100g", p.calories_per_100g as i32),
                    ChatLang::Uk => format!("мало калорій — {} ккал/100г", p.calories_per_100g as i32),
                }
            } else {
                match lang {
                    ChatLang::Ru => format!("{} ккал/100г", p.calories_per_100g as i32),
                    ChatLang::En => format!("{} kcal/100g", p.calories_per_100g as i32),
                    ChatLang::Pl => format!("{} kcal/100g", p.calories_per_100g as i32),
                    ChatLang::Uk => format!("{} ккал/100г", p.calories_per_100g as i32),
                }
            }
        }
    }
}

fn format_healthy_text(
    name: &str,
    p: &crate::infrastructure::ingredient_cache::IngredientData,
    lang: ChatLang,
    _highlight: &str,
    goal: HealthGoal,
) -> String {
    // Expert-style structured text: WHY this product + WHAT it does for the goal
    let cal = p.calories_per_100g as i32;
    let pro = p.protein_per_100g;
    let fat = p.fat_per_100g;
    let carb = p.carbs_per_100g;

    // Build benefit bullets based on actual macros
    let mut bullets_ru = Vec::new();
    let mut bullets_en = Vec::new();
    let mut bullets_pl = Vec::new();
    let mut bullets_uk = Vec::new();

    if pro >= 15.0 {
        bullets_ru.push(format!("• много белка ({:.0}г) → держит сытость", pro));
        bullets_en.push(format!("• high protein ({:.0}g) → keeps you full longer", pro));
        bullets_pl.push(format!("• dużo białka ({:.0}g) → dłużej trzyma sytość", pro));
        bullets_uk.push(format!("• багато білка ({:.0}г) → тримає ситість", pro));
    }
    if cal < 150 {
        bullets_ru.push(format!("• мало калорий ({} ккал) → можно есть больше", cal));
        bullets_en.push(format!("• low calories ({} kcal) → more food per day", cal));
        bullets_pl.push(format!("• mało kalorii ({} kcal) → możesz jeść więcej", cal));
        bullets_uk.push(format!("• мало калорій ({} ккал) → можна їсти більше", cal));
    }
    if carb < 5.0 {
        bullets_ru.push("• почти нет углеводов → не скачет инсулин".into());
        bullets_en.push("• near-zero carbs → stable insulin".into());
        bullets_pl.push("• prawie zero węglowodanów → stabilna insulina".into());
        bullets_uk.push("• майже нуль вуглеводів → стабільний інсулін".into());
    } else if carb < 15.0 {
        bullets_ru.push(format!("• мало углеводов ({:.0}г) → стабильный сахар в крови", carb));
        bullets_en.push(format!("• low carbs ({:.0}g) → stable blood sugar", carb));
        bullets_pl.push(format!("• mało węglowodanów ({:.0}g) → stabilny cukier", carb));
        bullets_uk.push(format!("• мало вуглеводів ({:.0}г) → стабільний цукор", carb));
    }
    if fat < 3.0 {
        bullets_ru.push("• минимум жира → чистый белок".into());
        bullets_en.push("• minimal fat → clean protein source".into());
        bullets_pl.push("• minimum tłuszczu → czyste białko".into());
        bullets_uk.push("• мінімум жиру → чистий білок".into());
    }

    // If no bullets matched, add generic macro line
    if bullets_ru.is_empty() {
        bullets_ru.push(format!("• {} ккал · {:.0}г белка · {:.0}г жиров · {:.0}г углеводов", cal, pro, fat, carb));
        bullets_en.push(format!("• {} kcal · {:.0}g protein · {:.0}g fat · {:.0}g carbs", cal, pro, fat, carb));
        bullets_pl.push(format!("• {} kcal · {:.0}g białka · {:.0}g tłuszczu · {:.0}g węglowodanów", cal, pro, fat, carb));
        bullets_uk.push(format!("• {} ккал · {:.0}г білка · {:.0}г жирів · {:.0}г вуглеводів", cal, pro, fat, carb));
    }

    // Goal-specific opening line (no "отличный выбор")
    match lang {
        ChatLang::Ru => {
            let opener = match goal {
                HealthGoal::LowCalorie  => format!("Для похудения **{}** — хороший вариант:", name),
                HealthGoal::HighProtein => format!("Для набора массы **{}** — сильный выбор:", name),
                HealthGoal::Balanced    => format!("**{}** — сбалансированный вариант:", name),
            };
            format!("{}\n{}", opener, bullets_ru.join("\n"))
        }
        ChatLang::En => {
            let opener = match goal {
                HealthGoal::LowCalorie  => format!("For weight loss, **{}** works well:", name),
                HealthGoal::HighProtein => format!("For muscle gain, **{}** is a strong pick:", name),
                HealthGoal::Balanced    => format!("**{}** — a balanced option:", name),
            };
            format!("{}\n{}", opener, bullets_en.join("\n"))
        }
        ChatLang::Pl => {
            let opener = match goal {
                HealthGoal::LowCalorie  => format!("Na odchudzanie **{}** — dobry wybór:", name),
                HealthGoal::HighProtein => format!("Na masę **{}** — mocny wybór:", name),
                HealthGoal::Balanced    => format!("**{}** — zbalansowana opcja:", name),
            };
            format!("{}\n{}", opener, bullets_pl.join("\n"))
        }
        ChatLang::Uk => {
            let opener = match goal {
                HealthGoal::LowCalorie  => format!("Для схуднення **{}** — хороший варіант:", name),
                HealthGoal::HighProtein => format!("Для набору маси **{}** — сильний вибір:", name),
                HealthGoal::Balanced    => format!("**{}** — збалансований варіант:", name),
            };
            format!("{}\n{}", opener, bullets_uk.join("\n"))
        }
    }
}

/// Meaningful macro summary for the "reason" field — no score numbers, just insight.
fn format_macro_summary(
    p: &crate::infrastructure::ingredient_cache::IngredientData,
    lang: ChatLang,
    goal: HealthGoal,
    total_options: usize,
) -> String {
    let pro = p.protein_per_100g;
    let fat = p.fat_per_100g;
    let cal = p.calories_per_100g as i32;

    // Classify macros
    let pro_level = if pro >= 20.0 { "high" } else if pro >= 10.0 { "moderate" } else { "low" };
    let fat_level = if fat >= 15.0 { "high" } else if fat >= 5.0 { "moderate" } else { "low" };

    let extras = if total_options > 1 {
        match lang {
            ChatLang::Ru => format!(" · +{} вариантов ниже", total_options - 1),
            ChatLang::En => format!(" · +{} more below", total_options - 1),
            ChatLang::Pl => format!(" · +{} więcej poniżej", total_options - 1),
            ChatLang::Uk => format!(" · +{} варіантів нижче", total_options - 1),
        }
    } else {
        String::new()
    };

    match lang {
        ChatLang::Ru => {
            let pro_s = match pro_level { "high" => "белок высокий", "moderate" => "белок средний", _ => "белка мало" };
            let fat_s = match fat_level { "high" => "жир высокий", "moderate" => "жир умеренный", _ => "жира минимум" };
            let action = match goal {
                HealthGoal::LowCalorie  => format!(" → {} ккал, можно улучшить", cal),
                HealthGoal::HighProtein => format!(" → {:.0}г белка/100г, хороший старт", pro),
                HealthGoal::Balanced    => " → баланс ОК".into(),
            };
            format!("{}, {}{}{}", pro_s, fat_s, action, extras)
        }
        ChatLang::En => {
            let pro_s = match pro_level { "high" => "protein high", "moderate" => "protein moderate", _ => "protein low" };
            let fat_s = match fat_level { "high" => "fat high", "moderate" => "fat moderate", _ => "fat minimal" };
            let action = match goal {
                HealthGoal::LowCalorie  => format!(" → {} kcal, room to improve", cal),
                HealthGoal::HighProtein => format!(" → {:.0}g protein/100g, good start", pro),
                HealthGoal::Balanced    => " → balance OK".into(),
            };
            format!("{}, {}{}{}", pro_s, fat_s, action, extras)
        }
        ChatLang::Pl => {
            let pro_s = match pro_level { "high" => "białko wysokie", "moderate" => "białko średnie", _ => "białka mało" };
            let fat_s = match fat_level { "high" => "tłuszcz wysoki", "moderate" => "tłuszcz umiarkowany", _ => "tłuszczu minimum" };
            let action = match goal {
                HealthGoal::LowCalorie  => format!(" → {} kcal, można poprawić", cal),
                HealthGoal::HighProtein => format!(" → {:.0}g białka/100g, dobry start", pro),
                HealthGoal::Balanced    => " → balans OK".into(),
            };
            format!("{}, {}{}{}", pro_s, fat_s, action, extras)
        }
        ChatLang::Uk => {
            let pro_s = match pro_level { "high" => "білок високий", "moderate" => "білок середній", _ => "білка мало" };
            let fat_s = match fat_level { "high" => "жир високий", "moderate" => "жир помірний", _ => "жиру мінімум" };
            let action = match goal {
                HealthGoal::LowCalorie  => format!(" → {} ккал, можна покращити", cal),
                HealthGoal::HighProtein => format!(" → {:.0}г білка/100г, хороший старт", pro),
                HealthGoal::Balanced    => " → баланс ОК".into(),
            };
            format!("{}, {}{}{}", pro_s, fat_s, action, extras)
        }
    }
}

fn fallback_healthy_text(lang: ChatLang) -> &'static str {
    match lang {
        ChatLang::Ru => "🥗 Полезные продукты: шпинат, брокколи, лосось, куриная грудка, яйца, миндаль. Спроси о конкретном — расскажу подробнее!",
        ChatLang::En => "🥗 Healthy picks: spinach, broccoli, salmon, chicken breast, eggs, almonds. Ask about a specific one for details!",
        ChatLang::Pl => "🥗 Zdrowe produkty: szpinak, brokuły, łosoś, filet z kurczaka, jajka, migdały. Zapytaj o konkretny — powiem więcej!",
        ChatLang::Uk => "🥗 Корисні продукти: шпинат, броколі, лосось, куряча грудка, яйця, мигдаль. Запитай про конкретний — розповім докладніше!",
    }
}

/// Full product info card — used by ProductInfo intent.
fn format_product_info_text(
    name: &str,
    p: &crate::infrastructure::ingredient_cache::IngredientData,
    lang: ChatLang,
) -> String {
    match lang {
        ChatLang::Ru => format!(
            "🔍 **{}**\n\nНутриенты на 100г:\n• Калории: {} ккал\n• Белки: {:.1} г\n• Жиры: {:.1} г\n• Углеводы: {:.1} г",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::En => format!(
            "🔍 **{}**\n\nNutrition per 100g:\n• Calories: {} kcal\n• Protein: {:.1}g\n• Fat: {:.1}g\n• Carbs: {:.1}g",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::Pl => format!(
            "🔍 **{}**\n\nWartości na 100g:\n• Kalorie: {} kcal\n• Białko: {:.1}g\n• Tłuszcz: {:.1}g\n• Węglowodany: {:.1}g",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::Uk => format!(
            "🔍 **{}**\n\nПоживні речовини на 100г:\n• Калорії: {} ккал\n• Білки: {:.1}г\n• Жири: {:.1}г\n• Вуглеводи: {:.1}г",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
    }
}

fn format_nutrition_text(
    name: &str,
    p: &crate::infrastructure::ingredient_cache::IngredientData,
    lang: ChatLang,
) -> String {
    match lang {
        ChatLang::Ru => format!(
            "📊 **{}** (на 100г):\n• Калории: {} ккал\n• Белки: {} г\n• Жиры: {} г\n• Углеводы: {} г",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::En => format!(
            "📊 **{}** (per 100g):\n• Calories: {} kcal\n• Protein: {}g\n• Fat: {}g\n• Carbs: {}g",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::Pl => format!(
            "📊 **{}** (na 100g):\n• Kalorie: {} kcal\n• Białko: {}g\n• Tłuszcz: {}g\n• Węglowodany: {}g",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::Uk => format!(
            "📊 **{}** (на 100г):\n• Калорії: {} ккал\n• Білки: {}г\n• Жири: {}г\n• Вуглеводи: {}г",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
    }
}

fn format_conversion_text(value: f64, from: &str, result: f64, to: &str, supported: bool, lang: ChatLang) -> String {
    if !supported {
        return match lang {
            ChatLang::Ru => format!("Не могу перевести {} {} в {} — такая конвертация не поддерживается. Попробуй: г, кг, мл, л, ст.л., ч.л.", value, from, to),
            ChatLang::En => format!("Cannot convert {} {} to {} — unsupported conversion. Try: g, kg, ml, l, tbsp, tsp.", value, from, to),
            ChatLang::Pl => format!("Nie mogę przeliczyć {} {} na {} — taka konwersja nie jest obsługiwana. Spróbuj: g, kg, ml, l, łyżka, łyżeczka.", value, from, to),
            ChatLang::Uk => format!("Не можу перевести {} {} в {} — така конвертація не підтримується.", value, from, to),
        };
    }
    match lang {
        ChatLang::Ru => format!("✅ {} {} = **{} {}**", value, from, result, to),
        ChatLang::En => format!("✅ {} {} = **{} {}**", value, from, result, to),
        ChatLang::Pl => format!("✅ {} {} = **{} {}**", value, from, result, to),
        ChatLang::Uk => format!("✅ {} {} = **{} {}**", value, from, result, to),
    }
}

fn format_season_text(product: &str, lang: ChatLang) -> String {
    // Static season data for common products
    let season_info = match product {
        "salmon" | "лосось" => ("salmon", "June–September", "Июнь–Сентябрь", "Czerwiec–Wrzesień"),
        "strawberry" | "клубника" | "truskawka" => ("strawberry", "May–July", "Май–Июль", "Maj–Lipiec"),
        "herring" | "сельдь" | "śledź" => ("herring", "October–April", "Октябрь–Апрель", "Październik–Kwiecień"),
        "mushrooms" | "грибы" | "grzyby" => ("mushrooms", "August–October", "Август–Октябрь", "Sierpień–Październik"),
        _ => return match lang {
            ChatLang::Ru => "Используй инструмент «Сезонный календарь» для точных данных по сезонам продуктов 📅".to_string(),
            ChatLang::En => "Use the \"Seasonal Calendar\" tool for accurate product season data 📅".to_string(),
            ChatLang::Pl => "Użyj narzędzia «Kalendarz sezonowy» dla dokładnych danych o sezonie produktów 📅".to_string(),
            ChatLang::Uk => "Використай інструмент «Сезонний календар» для точних даних по сезонах продуктів 📅".to_string(),
        },
    };

    match lang {
        ChatLang::Ru => format!("📅 **{}**: сезон {}", season_info.0, season_info.2),
        ChatLang::En => format!("📅 **{}**: season {}", season_info.0, season_info.1),
        ChatLang::Pl => format!("📅 **{}**: sezon {}", season_info.0, season_info.3),
        ChatLang::Uk => format!("📅 **{}**: сезон {}", season_info.0, season_info.2),
    }
}

fn format_recipe_hint(dish: &str, lang: ChatLang) -> String {
    match lang {
        ChatLang::Ru => format!("🍳 Ищешь рецепт: **{}**? Перейди в раздел «Рецепты» — там подробные шаги, КБЖУ и стоимость ингредиентов.", dish),
        ChatLang::En => format!("🍳 Looking for a **{}** recipe? Check the \"Recipes\" section — detailed steps, macros and ingredient costs.", dish),
        ChatLang::Pl => format!("🍳 Szukasz przepisu na **{}**? Przejdź do sekcji «Przepisy» — szczegółowe kroki, makroskładniki i ceny.", dish),
        ChatLang::Uk => format!("🍳 Шукаєш рецепт: **{}**? Перейди до розділу «Рецепти» — там покрокові інструкції, КБЖУ та вартість.", dish),
    }
}

// ── Simple NLP helpers ────────────────────────────────────────────────────────

/// Try to extract (value, from_unit, to_unit) from text like "200 грамм в ложках".
fn extract_conversion(input: &str) -> Option<(f64, String, String)> {
    let text = input.to_lowercase();

    // Find a number
    let value = text
        .split_whitespace()
        .find_map(|w| w.parse::<f64>().ok())?;

    // Map Russian/Polish keywords to standard unit codes
    let unit_map: &[(&str, &str)] = &[
        ("грамм", "g"),  ("граммов", "g"), ("грам", "g"), ("г ", "g"),
        ("килограмм", "kg"), ("кг", "kg"), ("кило", "kg"),
        ("миллилитр", "ml"), ("мл", "ml"),
        ("литр", "l"), ("л ", "l"),
        ("унци", "oz"), ("унц", "oz"), ("oz", "oz"),
        ("ложк", "tbsp"), ("tbsp", "tbsp"), ("ст.л", "tbsp"),
        ("чайн", "tsp"), ("tsp", "tsp"), ("ч.л", "tsp"),
        ("стакан", "cup"), ("cup", "cup"),
        // Polish
        ("gram", "g"), ("kilogram", "kg"), ("mililitr", "ml"),
        ("litr", "l"), ("łyżk", "tbsp"), ("szklank", "cup"),
        // English
        ("gram", "g"), ("kilogram", "kg"), ("milliliter", "ml"),
        ("liter", "l"), ("tablespoon", "tbsp"), ("teaspoon", "tsp"),
    ];

    let from_unit = unit_map.iter()
        .find(|(kw, _)| text.contains(kw) && text.find(kw) < text.rfind(|c: char| c.is_ascii_digit()).map(|p| p + 10).or(Some(1000)))
        .map(|(_, u)| u.to_string())?;

    // Find "to" unit — look after "в ", "in ", "na "
    let after_markers = ["в ", "in ", "na ", "to ", "на "];
    let search_start = after_markers.iter()
        .find_map(|m| text.find(m).map(|p| p + m.len()))
        .unwrap_or(0);
    let rest = &text[search_start..];

    let to_unit = unit_map.iter()
        .find(|(kw, u)| rest.contains(kw) && *u != from_unit)
        .map(|(_, u)| u.to_string())?;

    Some((value, from_unit, to_unit))
}

fn detect_season_product(text: &str) -> Option<&'static str> {
    let products = [
        ("salmon", &["лосось", "salmon", "łosoś"][..]),
        ("strawberry", &["клубника", "strawberry", "truskawka"][..]),
        ("herring", &["сельдь", "селёдка", "herring", "śledź"][..]),
        ("mushrooms", &["гриб", "mushroom", "grzyb"][..]),
    ];
    for (slug, keywords) in &products {
        if keywords.iter().any(|kw| text.contains(kw)) {
            return Some(slug);
        }
    }
    None
}

fn detect_dish_keyword(text: &str) -> Option<&'static str> {
    let dishes = [
        ("борщ", "borscht"), ("pasta", "pasta"), ("паст", "pasta"),
        ("суп", "soup"), ("soup", "soup"), ("zup", "soup"),
        ("салат", "salad"), ("salad", "salad"), ("sałatk", "salad"),
        ("омлет", "omelette"), ("omelette", "omelette"),
        ("пицца", "pizza"), ("pizza", "pizza"),
        ("котлет", "cutlet"), ("стейк", "steak"), ("steak", "steak"),
    ];
    for (keyword, dish_name) in &dishes {
        if text.contains(keyword) {
            return Some(dish_name);
        }
    }
    None
}

// ── Suggestion & chef-tip builders ─────────────────────────────────────────────

fn build_healthy_suggestions(lang: ChatLang, goal: HealthGoal, top_name: &str) -> Vec<Suggestion> {
    // "Plan" button has a hook with numbers — user sees the value BEFORE clicking
    let plan_label = match lang {
        ChatLang::Ru => match goal {
            HealthGoal::LowCalorie  => "~1600 ккал · 100г белка → Собрать день".into(),
            HealthGoal::HighProtein => "~2200 ккал · 160г белка → Собрать день".into(),
            HealthGoal::Balanced    => "~1800 ккал · 120г белка → Собрать день".into(),
        },
        ChatLang::En => match goal {
            HealthGoal::LowCalorie  => "~1600 kcal · 100g protein → Build my day".into(),
            HealthGoal::HighProtein => "~2200 kcal · 160g protein → Build my day".into(),
            HealthGoal::Balanced    => "~1800 kcal · 120g protein → Build my day".into(),
        },
        ChatLang::Pl => match goal {
            HealthGoal::LowCalorie  => "~1600 kcal · 100g białka → Ułóż dzień".into(),
            HealthGoal::HighProtein => "~2200 kcal · 160g białka → Ułóż dzień".into(),
            HealthGoal::Balanced    => "~1800 kcal · 120g białka → Ułóż dzień".into(),
        },
        ChatLang::Uk => match goal {
            HealthGoal::LowCalorie  => "~1600 ккал · 100г білка → Скласти день".into(),
            HealthGoal::HighProtein => "~2200 ккал · 160г білка → Скласти день".into(),
            HealthGoal::Balanced    => "~1800 ккал · 120г білка → Скласти день".into(),
        },
    };

    match lang {
        ChatLang::Ru => vec![
            Suggestion { label: format!("Рецепты с {}", top_name), query: format!("рецепт с {}", top_name), emoji: Some("📖") },
            Suggestion { label: plan_label, query: "план питания на день".into(), emoji: Some("📋") },
            Suggestion { label: "Ещё варианты".into(), query: match goal {
                HealthGoal::HighProtein => "ещё высокобелковые продукты".into(),
                HealthGoal::LowCalorie  => "ещё низкокалорийные продукты".into(),
                HealthGoal::Balanced    => "ещё полезные продукты".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::En => vec![
            Suggestion { label: format!("Recipes with {}", top_name), query: format!("recipe with {}", top_name), emoji: Some("📖") },
            Suggestion { label: plan_label, query: "meal plan for the day".into(), emoji: Some("📋") },
            Suggestion { label: "More options".into(), query: match goal {
                HealthGoal::HighProtein => "more high protein foods".into(),
                HealthGoal::LowCalorie  => "more low calorie foods".into(),
                HealthGoal::Balanced    => "more healthy food ideas".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: format!("Przepisy z {}", top_name), query: format!("przepis z {}", top_name), emoji: Some("📖") },
            Suggestion { label: plan_label, query: "plan posiłków na dzień".into(), emoji: Some("📋") },
            Suggestion { label: "Więcej opcji".into(), query: match goal {
                HealthGoal::HighProtein => "więcej produktów wysokobiałkowych".into(),
                HealthGoal::LowCalorie  => "więcej niskokalorycznych produktów".into(),
                HealthGoal::Balanced    => "więcej zdrowych produktów".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: format!("Рецепти з {}", top_name), query: format!("рецепт з {}", top_name), emoji: Some("📖") },
            Suggestion { label: plan_label, query: "план харчування на день".into(), emoji: Some("📋") },
            Suggestion { label: "Ще варіанти".into(), query: match goal {
                HealthGoal::HighProtein => "ще високобілкові продукти".into(),
                HealthGoal::LowCalorie  => "ще низькокалорійні продукти".into(),
                HealthGoal::Balanced    => "ще корисні продукти".into(),
            }, emoji: Some("🔄") },
        ],
    }
}

fn build_meal_suggestions(lang: ChatLang, slug: &str) -> Vec<Suggestion> {
    match lang {
        ChatLang::Ru => vec![
            Suggestion { label: "Покажи рецепт".into(), query: format!("рецепт с {}", slug), emoji: Some("🍳") },
            Suggestion { label: "Другая идея".into(), query: "что ещё приготовить".into(), emoji: Some("🔄") },
            Suggestion { label: "Калории продукта".into(), query: format!("калории {}", slug), emoji: Some("📊") },
        ],
        ChatLang::En => vec![
            Suggestion { label: "Show recipe".into(), query: format!("recipe with {}", slug), emoji: Some("🍳") },
            Suggestion { label: "Another idea".into(), query: "another meal idea".into(), emoji: Some("🔄") },
            Suggestion { label: "Product calories".into(), query: format!("calories in {}", slug), emoji: Some("📊") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: "Pokaż przepis".into(), query: format!("przepis z {}", slug), emoji: Some("🍳") },
            Suggestion { label: "Inny pomysł".into(), query: "inny pomysł na posiłek".into(), emoji: Some("🔄") },
            Suggestion { label: "Kalorie produktu".into(), query: format!("kalorie {}", slug), emoji: Some("📊") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: "Покажи рецепт".into(), query: format!("рецепт з {}", slug), emoji: Some("🍳") },
            Suggestion { label: "Інша ідея".into(), query: "що ще приготувати".into(), emoji: Some("🔄") },
            Suggestion { label: "Калорії продукту".into(), query: format!("калорії {}", slug), emoji: Some("📊") },
        ],
    }
}

fn pick_chef_tip(
    p: &crate::infrastructure::ingredient_cache::IngredientData,
    lang: ChatLang,
    goal: HealthGoal,
) -> String {
    // Product-specific tips based on slug — always relevant to the product shown
    let slug = p.slug.to_lowercase();

    // Tip index rotation for variety within a category
    let tip_seed = {
        use std::time::{SystemTime, UNIX_EPOCH};
        (SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() / 10) as usize
    };

    // ── Product-specific tip tables (4 langs) ──
    // Each returns Option<&str> — if None, fall through to macro-based tips
    let product_tip: Option<(&str, &str, &str, &str)> = if slug.contains("chicken") {
        Some((
            "Запекай курицу без кожи — минус ~120 ккал на порцию. Мясо останется сочным, если накрыть фольгой.",
            "Bake chicken without skin — minus ~120 kcal per serving. Cover with foil to keep it juicy.",
            "Piecz kurczaka bez skóry — minus ~120 kcal na porcję. Przykryj folią, żeby był soczysty.",
            "Запікай курку без шкіри — мінус ~120 ккал на порцію. Накрий фольгою для соковитості.",
        ))
    } else if slug.contains("salmon") {
        Some((
            "Лосось на пару за 12 минут — сохраняет омега-3 и текстуру. Жарка разрушает до 30% жирных кислот.",
            "Steam salmon for 12 min — preserves omega-3 and texture. Frying destroys up to 30% of fatty acids.",
            "Łosoś na parze 12 min — zachowuje omega-3 i teksturę. Smażenie niszczy do 30% kwasów tłuszczowych.",
            "Лосось на парі 12 хв — зберігає омега-3. Смаження руйнує до 30% жирних кислот.",
        ))
    } else if slug.contains("egg") {
        Some((
            "Варёное яйцо — 78 ккал. Жареное — 120+. Разница в масле, а не в яйце.",
            "Boiled egg: 78 kcal. Fried: 120+. The difference is the oil, not the egg.",
            "Jajko gotowane: 78 kcal. Smażone: 120+. Różnica w oleju, nie w jajku.",
            "Варене яйце — 78 ккал. Смажене — 120+. Різниця в олії, а не в яйці.",
        ))
    } else if slug.contains("spinach") {
        Some((
            "Шпинат теряет 50% объёма при готовке — клади в 2 раза больше, чем кажется нужным.",
            "Spinach loses 50% volume when cooked — use 2x more than you think you need.",
            "Szpinak traci 50% objętości przy gotowaniu — daj 2x więcej niż ci się wydaje.",
            "Шпинат втрачає 50% об'єму при готуванні — клади вдвічі більше.",
        ))
    } else if slug.contains("broccoli") {
        Some((
            "Брокколи на пару 5 мин — максимум витамина C. Если варить дольше — теряешь до 60%.",
            "Steam broccoli for 5 min — maximum vitamin C. Boiling longer loses up to 60%.",
            "Brokuły na parze 5 min — maks. witaminy C. Dłuższe gotowanie traci do 60%.",
            "Броколі на парі 5 хв — максимум вітаміну C. Довше варити — мінус 60%.",
        ))
    } else if slug.contains("tuna") {
        Some((
            "Тунец из банки в собственном соку — 100 ккал. В масле — 200+. Всегда выбирай «в соку».",
            "Canned tuna in water: 100 kcal. In oil: 200+. Always pick water-packed.",
            "Tuńczyk w wodzie: 100 kcal. W oleju: 200+. Zawsze wybieraj w sosie własnym.",
            "Тунець у власному соці — 100 ккал. В олії — 200+. Завжди обирай «у соці».",
        ))
    } else if slug.contains("almond") {
        Some((
            "Миндаль — 30г (горсть) = ~170 ккал. Легко переесть. Отмеряй порцию заранее.",
            "Almonds — 30g (handful) = ~170 kcal. Easy to overeat. Pre-measure your portion.",
            "Migdały — 30g (garść) = ~170 kcal. Łatwo zjeść za dużo. Odmierz porcję z góry.",
            "Мигдаль — 30г (жменька) = ~170 ккал. Легко переїсти. Відміряй порцію заздалегідь.",
        ))
    } else if slug.contains("rice") {
        Some((
            "Охлаждённый рис содержит резистентный крахмал — меньше калорий усваивается. Приготовь заранее.",
            "Cooled rice contains resistant starch — fewer calories absorbed. Cook it ahead.",
            "Schłodzony ryż zawiera skrobię oporną — mniej kalorii się wchłania. Ugotuj wcześniej.",
            "Охолоджений рис містить резистентний крохмаль — менше калорій засвоюється.",
        ))
    } else if slug.contains("beef") {
        Some((
            "Говядина: дай стейку отдохнуть 5 мин — соки перераспределятся, мясо будет нежнее на 40%.",
            "Beef: let the steak rest 5 min — juices redistribute, 40% more tender.",
            "Wołowina: daj stekowi odpocząć 5 min — soki się rozprowadzą, mięso o 40% delikatniejsze.",
            "Яловичина: дай стейку відпочити 5 хв — соки розподіляться, м'ясо ніжніше на 40%.",
        ))
    } else if slug.contains("blueberr") {
        Some((
            "Замороженная черника сохраняет 95% антиоксидантов — не хуже свежей, а дешевле в 3 раза.",
            "Frozen blueberries retain 95% of antioxidants — as good as fresh, 3x cheaper.",
            "Mrożone jagody zachowują 95% antyoksydantów — tak dobre jak świeże, 3x tańsze.",
            "Заморожена чорниця зберігає 95% антиоксидантів — не гірша за свіжу, а дешевша в 3 рази.",
        ))
    } else {
        None
    };

    if let Some((ru, en, pl, uk)) = product_tip {
        return match lang {
            ChatLang::Ru => format!("💡 Шеф-совет: {}", ru),
            ChatLang::En => format!("💡 Chef tip: {}", en),
            ChatLang::Pl => format!("💡 Rada szefa: {}", pl),
            ChatLang::Uk => format!("💡 Порада шефа: {}", uk),
        };
    }

    // ── Fallback: macro-based tips when slug not recognized ──
    let high_protein = p.protein_per_100g >= 20.0;
    let low_cal = p.calories_per_100g < 80.0;
    let high_fat = p.fat_per_100g >= 15.0;
    let is_meat = p.protein_per_100g >= 18.0 && p.fat_per_100g >= 3.0;
    let is_veggie = p.calories_per_100g < 50.0 && p.protein_per_100g < 5.0;

    match lang {
        ChatLang::Ru => {
            let tips: Vec<&str> = if is_meat {
                vec![
                    "Готовь мясо на решётке или в духовке — жир стечёт, минус ~100 ккал.",
                    "Маринуй в лимонном соке + травы — вкуснее и мягче без масла.",
                    "Дай мясу «отдохнуть» 5 мин после готовки — соки распределятся.",
                ]
            } else if is_veggie {
                vec![
                    "Овощи аль-денте сохраняют витамины. Не переваривай — 3-5 мин на пару достаточно.",
                    "Заправляй лимонным соком вместо масла — минус 100 ккал, плюс витамин C.",
                    "Запекай вместо варки — карамелизация даёт вкус без калорий.",
                ]
            } else if high_protein && matches!(goal, HealthGoal::HighProtein) {
                vec![
                    "Готовь на пару — сохраняет до 95% белка, в отличие от жарки.",
                    "Сочетай с бобовыми — получишь полный аминокислотный профиль.",
                ]
            } else if high_fat {
                vec![
                    "Калорийный продукт — используй как усилитель вкуса, не как основу.",
                    "Отмеряй порцию заранее — легко переесть на 200+ ккал.",
                ]
            } else if low_cal && matches!(goal, HealthGoal::LowCalorie) {
                vec![
                    "Запекай вместо жарки — экономишь ~80 ккал на порцию.",
                    "Ешь медленнее — насыщение приходит через 20 минут.",
                ]
            } else {
                vec![
                    "Свежие специи (базилик, кинза) добавляй в конце — так ярче вкус.",
                    "Пробуй новые способы готовки: пар, гриль, запекание — каждый раскрывает продукт по-разному.",
                ]
            };
            format!("💡 Шеф-совет: {}", tips[tip_seed % tips.len()])
        }
        ChatLang::En => {
            let tips: Vec<&str> = if is_meat {
                vec![
                    "Cook meat on a rack or in the oven — fat drips off, minus ~100 kcal.",
                    "Marinate in lemon juice + herbs — tastier and tender without oil.",
                    "Let meat rest 5 min after cooking — juices redistribute evenly.",
                ]
            } else if is_veggie {
                vec![
                    "Al dente veggies keep their vitamins. Don't overcook — 3-5 min steaming is enough.",
                    "Use lemon juice instead of oil — minus 100 kcal, plus vitamin C.",
                    "Roast instead of boiling — caramelization adds flavor without calories.",
                ]
            } else if high_protein && matches!(goal, HealthGoal::HighProtein) {
                vec![
                    "Steam instead of frying — preserves up to 95% of protein.",
                    "Pair with legumes for a complete amino acid profile.",
                ]
            } else if high_fat {
                vec![
                    "Calorie-dense — use as a flavor booster, not the main course.",
                    "Pre-measure your portion — easy to overeat by 200+ kcal.",
                ]
            } else if low_cal && matches!(goal, HealthGoal::LowCalorie) {
                vec![
                    "Bake instead of frying — saves ~80 kcal per serving.",
                    "Eat slowly — fullness takes 20 minutes to kick in.",
                ]
            } else {
                vec![
                    "Add fresh herbs (basil, cilantro) at the end for brighter flavor.",
                    "Try different cooking methods: steam, grill, roast — each reveals the product differently.",
                ]
            };
            format!("💡 Chef tip: {}", tips[tip_seed % tips.len()])
        }
        ChatLang::Pl => {
            let tips: Vec<&str> = if is_meat {
                vec![
                    "Piecz mięso na ruszcie — tłuszcz ścieka, minus ~100 kcal.",
                    "Marynuj w soku z cytryny + zioła — smaczniej i delikatniej bez oleju.",
                    "Daj mięsu odpocząć 5 min — soki się rozprowadzą.",
                ]
            } else if is_veggie {
                vec![
                    "Warzywa al dente zachowują witaminy. Nie rozgotowuj — 3-5 min na parze wystarczy.",
                    "Zamiast oleju — sok z cytryny — minus 100 kcal, plus witamina C.",
                    "Piecz zamiast gotować — karmelizacja daje smak bez kalorii.",
                ]
            } else if high_protein && matches!(goal, HealthGoal::HighProtein) {
                vec![
                    "Gotuj na parze — zachowuje do 95% białka.",
                    "Połącz z roślinami strączkowymi — pełny profil aminokwasów.",
                ]
            } else if high_fat {
                vec![
                    "Kaloryczny produkt — używaj jako wzmacniacz smaku, nie podstawę.",
                    "Odmierz porcję z góry — łatwo zjeść za dużo.",
                ]
            } else if low_cal && matches!(goal, HealthGoal::LowCalorie) {
                vec![
                    "Piecz zamiast smażyć — oszczędzasz ~80 kcal na porcję.",
                    "Jedz wolniej — sytość przychodzi po 20 minutach.",
                ]
            } else {
                vec![
                    "Świeże zioła (bazylia, kolendra) dodawaj na końcu — smak będzie żywszy.",
                    "Wypróbuj różne metody: para, grill, pieczenie — każda wydobywa inny smak.",
                ]
            };
            format!("💡 Rada szefa: {}", tips[tip_seed % tips.len()])
        }
        ChatLang::Uk => {
            let tips: Vec<&str> = if is_meat {
                vec![
                    "Готуй м'ясо на решітці — жир стече, мінус ~100 ккал.",
                    "Маринуй в лимонному соці + трави — смачніше без олії.",
                    "Дай м'ясу «відпочити» 5 хв — соки розподіляться.",
                ]
            } else if is_veggie {
                vec![
                    "Овочі аль-денте зберігають вітаміни. Не перевар — 3-5 хв на парі достатньо.",
                    "Замість олії — лимонний сік — мінус 100 ккал, плюс вітамін C.",
                    "Запікай замість варки — карамелізація дає смак без калорій.",
                ]
            } else if high_protein && matches!(goal, HealthGoal::HighProtein) {
                vec![
                    "Готуй на парі — зберігає до 95% білка.",
                    "Поєднуй з бобовими — повний амінокислотний профіль.",
                ]
            } else if high_fat {
                vec![
                    "Калорійний продукт — використовуй як підсилювач смаку.",
                    "Відміряй порцію заздалегідь — легко переїсти на 200+ ккал.",
                ]
            } else if low_cal && matches!(goal, HealthGoal::LowCalorie) {
                vec![
                    "Запікай замість смаження — економиш ~80 ккал на порцію.",
                    "Їж повільніше — ситість приходить через 20 хвилин.",
                ]
            } else {
                vec![
                    "Свіжі спеції додавай в кінці — так яскравіший смак.",
                    "Спробуй різні способи: пара, гриль, запікання — кожен розкриває продукт інакше.",
                ]
            };
            format!("💡 Порада шефа: {}", tips[tip_seed % tips.len()])
        }
    }
}

// needed for hour/day helpers
use chrono::Timelike;
