//! Chef Coach — motivational sous-chef messages.
//!
//! The coach tracks the user's journey through the session and injects
//! motivational, educational, and supportive messages as a sous-chef persona.
//!
//! Activation rules:
//!   - Every 2nd–3rd turn (not every turn — avoid annoying)
//!   - Contextual: adapts to goal (weight loss, muscle, balanced)
//!   - Progressive: messages evolve with session depth
//!   - Multilingual: RU / EN / PL / UK
//!
//! Usage in chat_engine.rs:
//! ```ignore
//!   response.coach_message = chef_coach::pick_message(ctx, goal, lang);
//! ```

use super::intent_router::ChatLang;
use super::response_builder::HealthGoal;
use super::session_context::SessionContext;

/// Pick a motivational coach message based on session context.
/// Returns `None` if this turn should be silent (to avoid being annoying).
pub fn pick_message(
    ctx: &SessionContext,
    goal: HealthGoal,
    lang: ChatLang,
) -> Option<String> {
    let turn = ctx.turn_count;
    let explored = ctx.shown_slugs.len();

    // ── Frequency control ─────────────────────────────────────────────────
    // Show coach message on turns: 1, 3, 5, 8, 12, 17, ...
    // i.e. first turn always, then every 2-3 turns, spacing out over time
    let should_show = match turn {
        0 | 1 => true,                      // Welcome motivation
        2 => false,                          // Skip — let them explore
        3 | 4 => turn == 3,                  // Show at turn 3
        5..=7 => turn == 5,                  // Show at turn 5
        8..=11 => turn == 8,                 // Show at turn 8
        12..=16 => turn == 12,               // Show at turn 12
        _ => turn % 5 == 0,                  // Every 5th turn after
    };

    if !should_show {
        return None;
    }

    // ── Message selection ─────────────────────────────────────────────────
    let msg = match (turn, goal) {
        // ═══ FIRST INTERACTION — warm welcome ═══
        (0..=1, HealthGoal::LowCalorie) => match lang {
            ChatLang::Ru => "🔥 Шеф-коуч: Отлично, что ты решил(а) заняться своим питанием! Похудение — это не голодовка, а умный выбор продуктов. Давай разберёмся вместе!",
            ChatLang::En => "🔥 Chef Coach: Great that you're taking charge of your nutrition! Weight loss isn't about starving — it's about smart food choices. Let's figure it out together!",
            ChatLang::Pl => "🔥 Szef-coach: Świetnie, że dbasz o swoje odżywianie! Odchudzanie to nie głodówka — to mądre wybory. Zróbmy to razem!",
            ChatLang::Uk => "🔥 Шеф-коуч: Чудово, що ти вирішив(ла) зайнятися харчуванням! Схуднення — це не голодування, а розумний вибір продуктів. Давай розберемося разом!",
        },
        (0..=1, HealthGoal::HighProtein) => match lang {
            ChatLang::Ru => "💪 Шеф-коуч: Набор массы начинается на кухне! 70% результата — это питание. Я подберу тебе продукты с максимумом белка и правильным балансом.",
            ChatLang::En => "💪 Chef Coach: Muscle building starts in the kitchen! 70% of results come from nutrition. I'll find you the best protein-packed foods.",
            ChatLang::Pl => "💪 Szef-coach: Budowanie masy zaczyna się w kuchni! 70% wyników to odżywianie. Znajdę ci najlepsze produkty bogate w białko.",
            ChatLang::Uk => "💪 Шеф-коуч: Набір маси починається на кухні! 70% результату — це харчування. Я підберу продукти з максимумом білка.",
        },
        (0..=1, HealthGoal::Balanced) => match lang {
            ChatLang::Ru => "👨‍🍳 Шеф-коуч: Рад видеть тебя! Я помогу разобраться в продуктах — калории, белки, витамины. Спрашивай что угодно!",
            ChatLang::En => "👨‍🍳 Chef Coach: Welcome! I'll help you understand food — calories, protein, vitamins. Ask me anything!",
            ChatLang::Pl => "👨‍🍳 Szef-coach: Witaj! Pomogę ci zrozumieć jedzenie — kalorie, białko, witaminy. Pytaj o cokolwiek!",
            ChatLang::Uk => "👨‍🍳 Шеф-коуч: Радий бачити! Я допоможу розібратися в продуктах — калорії, білки, вітаміни. Питай що завгодно!",
        },

        // ═══ TURN 3 — first progress acknowledgment ═══
        (3, HealthGoal::LowCalorie) => match lang {
            ChatLang::Ru => "📊 Шеф-коуч: Ты уже изучил(а) несколько продуктов — это отличный старт! Помни: дефицит 300–500 ккал в день = минус 0.5 кг в неделю. Без стресса, без срывов.",
            ChatLang::En => "📊 Chef Coach: You've already explored several products — great start! Remember: a 300–500 kcal deficit per day = minus 0.5 kg per week. No stress, no crashes.",
            ChatLang::Pl => "📊 Szef-coach: Już poznałeś kilka produktów — świetny start! Pamiętaj: deficyt 300–500 kcal dziennie = minus 0.5 kg tygodniowo.",
            ChatLang::Uk => "📊 Шеф-коуч: Ти вже вивчив(ла) кілька продуктів — чудовий старт! Пам'ятай: дефіцит 300–500 ккал на день = мінус 0.5 кг на тиждень.",
        },
        (3, HealthGoal::HighProtein) => match lang {
            ChatLang::Ru => "🎯 Шеф-коуч: Уже разбираешься! Для роста мышц нужно 1.6–2.2г белка на кг веса. Распредели его на 4–5 приёмов — усвоение будет максимальным.",
            ChatLang::En => "🎯 Chef Coach: You're getting the hang of it! For muscle growth, aim for 1.6–2.2g protein per kg body weight. Spread it across 4–5 meals for best absorption.",
            ChatLang::Pl => "🎯 Szef-coach: Już się orientujesz! Na masę potrzebujesz 1.6–2.2g białka na kg masy ciała. Rozłóż na 4–5 posiłków.",
            ChatLang::Uk => "🎯 Шеф-коуч: Вже розбираєшся! Для росту м'язів потрібно 1.6–2.2г білка на кг ваги. Розподіли на 4–5 прийомів.",
        },
        (3, _) => match lang {
            ChatLang::Ru => "💡 Шеф-коуч: Ты на верном пути! Чем больше ты знаешь о продуктах, тем лучше твои решения. Продолжай исследовать!",
            ChatLang::En => "💡 Chef Coach: You're on the right track! The more you know about food, the better your choices. Keep exploring!",
            ChatLang::Pl => "💡 Szef-coach: Jesteś na dobrej drodze! Im więcej wiesz o jedzeniu, tym lepsze twoje wybory.",
            ChatLang::Uk => "💡 Шеф-коуч: Ти на правильному шляху! Чим більше знаєш про продукти, тим кращі рішення.",
        },

        // ═══ TURN 5 — building habits ═══
        (5, HealthGoal::LowCalorie) => match lang {
            ChatLang::Ru => "🌱 Шеф-коуч: 5 шагов пройдено! Лайфхак: заполняй тарелку по правилу 50/25/25 — половина овощи, четверть белок, четверть углеводы. Объём есть, а калорий мало!",
            ChatLang::En => "🌱 Chef Coach: 5 steps done! Pro tip: fill your plate 50/25/25 — half veggies, quarter protein, quarter carbs. Volume eating with fewer calories!",
            ChatLang::Pl => "🌱 Szef-coach: 5 kroków za tobą! Wskazówka: talerz 50/25/25 — połowa warzywa, ćwierć białko, ćwierć węglowodany.",
            ChatLang::Uk => "🌱 Шеф-коуч: 5 кроків пройдено! Лайфхак: тарілка 50/25/25 — половина овочі, чверть білок, чверть вуглеводи.",
        },
        (5, HealthGoal::HighProtein) => match lang {
            ChatLang::Ru => "🏋️ Шеф-коуч: Уже 5 запросов — ты серьёзно настроен! Совет: готовь batch-cooking на 3 дня — курица, рис, овощи в контейнерах. Экономит время и калории.",
            ChatLang::En => "🏋️ Chef Coach: 5 queries — you're serious! Tip: try batch cooking for 3 days — chicken, rice, veggies in containers. Saves time and controls portions.",
            ChatLang::Pl => "🏋️ Szef-coach: 5 zapytań — jesteś na poważnie! Spróbuj gotowania na 3 dni — kurczak, ryż, warzywa w pojemnikach.",
            ChatLang::Uk => "🏋️ Шеф-коуч: 5 запитів — ти серйозно налаштований! Порада: готуй batch-cooking на 3 дні — курка, рис, овочі в контейнерах.",
        },
        (5, _) => match lang {
            ChatLang::Ru => "✨ Шеф-коуч: Ты уже профи! Знал(а), что разнообразие в еде — ключ к здоровью? Разные цвета продуктов = разные витамины.",
            ChatLang::En => "✨ Chef Coach: You're becoming a pro! Did you know food diversity is key to health? Different colors = different vitamins.",
            ChatLang::Pl => "✨ Szef-coach: Stajesz się pro! Różnorodność w jedzeniu to klucz do zdrowia. Różne kolory = różne witaminy.",
            ChatLang::Uk => "✨ Шеф-коуч: Ти вже профі! Знав(ла), що різноманіття в їжі — ключ до здоров'я? Різні кольори продуктів = різні вітаміни.",
        },

        // ═══ TURN 8 — deep session, reward persistence ═══
        (8, HealthGoal::LowCalorie) => {
            let explored_msg = if explored >= 6 { "более 6" } else { "несколько" };
            return Some(match lang {
                ChatLang::Ru => format!("🏆 Шеф-коуч: Ты изучил(а) {} продуктов! Это не просто запросы — это инвестиция в здоровье. Каждый раз, выбирая шпинат вместо чипсов, ты побеждаешь.", explored_msg),
                ChatLang::En => format!("🏆 Chef Coach: You've explored {} products! Every smart food choice is a win. You're building habits that last.", if explored >= 6 { "6+" } else { "several" }),
                ChatLang::Pl => format!("🏆 Szef-coach: Poznałeś {} produktów! Każdy mądry wybór to krok do celu.", if explored >= 6 { "ponad 6" } else { "kilka" }),
                ChatLang::Uk => format!("🏆 Шеф-коуч: Ти вивчив(ла) {} продуктів! Кожен розумний вибір — це крок до мети.", if explored >= 6 { "більше 6" } else { "кілька" }),
            });
        },
        (8, _) => match lang {
            ChatLang::Ru => "🏆 Шеф-коуч: Уже 8 вопросов — ты настоящий исследователь! Совет: попробуй сочетать продукты — железо из шпината усваивается лучше с витамином C (лимон).",
            ChatLang::En => "🏆 Chef Coach: 8 questions — you're a true explorer! Tip: try food combos — iron from spinach absorbs better with vitamin C (lemon).",
            ChatLang::Pl => "🏆 Szef-coach: 8 pytań — jesteś prawdziwym odkrywcą! Żelazo ze szpinaku wchłania się lepiej z witaminą C (cytryna).",
            ChatLang::Uk => "🏆 Шеф-коуч: 8 питань — ти справжній дослідник! Залізо зі шпинату засвоюється краще з вітаміном C (лимон).",
        },

        // ═══ TURN 12 — loyal user, advanced tips ═══
        (12, HealthGoal::LowCalorie) => match lang {
            ChatLang::Ru => "🌟 Шеф-коуч: 12 шагов — ты уже знаешь больше о питании, чем 90% людей! Секрет: не считай каждую калорию — научись чувствовать порции. Ладонь = порция белка, кулак = порция углеводов.",
            ChatLang::En => "🌟 Chef Coach: 12 steps — you know more about nutrition than 90% of people! Secret: don't count every calorie — learn to feel portions. Palm = protein portion, fist = carb portion.",
            ChatLang::Pl => "🌟 Szef-coach: 12 kroków — wiesz o odżywianiu więcej niż 90% ludzi! Dłoń = porcja białka, pięść = porcja węglowodanów.",
            ChatLang::Uk => "🌟 Шеф-коуч: 12 кроків — ти знаєш про харчування більше, ніж 90% людей! Долоня = порція білка, кулак = порція вуглеводів.",
        },
        (12, HealthGoal::HighProtein) => match lang {
            ChatLang::Ru => "🌟 Шеф-коуч: 12 запросов — ты строишь тело через знания! Помни: сон 7-8 часов = рост мышц. Белок усваивается именно во сне. Ешь казеин на ночь (творог).",
            ChatLang::En => "🌟 Chef Coach: 12 queries — building your body through knowledge! Remember: 7-8 hours of sleep = muscle growth. Protein is absorbed during sleep. Eat casein at night (cottage cheese).",
            ChatLang::Pl => "🌟 Szef-coach: 12 zapytań — budujesz ciało przez wiedzę! Sen 7-8 godzin = wzrost mięśni. Zjedz kazeinę na noc (twaróg).",
            ChatLang::Uk => "🌟 Шеф-коуч: 12 запитів — ти будуєш тіло через знання! Сон 7-8 годин = ріст м'язів. Їж казеїн на ніч (сирок).",
        },
        (12, _) => match lang {
            ChatLang::Ru => "🌟 Шеф-коуч: 12 вопросов — ты эксперт! Попробуй спланировать меню на неделю. Напиши «план питания» — я помогу.",
            ChatLang::En => "🌟 Chef Coach: 12 questions — you're an expert! Try planning a weekly menu. Type 'meal plan' and I'll help.",
            ChatLang::Pl => "🌟 Szef-coach: 12 pytań — jesteś ekspertem! Napisz 'plan posiłków' — pomogę.",
            ChatLang::Uk => "🌟 Шеф-коуч: 12 питань — ти експерт! Напиши «план харчування» — я допоможу.",
        },

        // ═══ EVERY 5th TURN after 12 — rotating wisdom ═══
        (n, goal) if n > 12 && n % 5 == 0 => {
            let idx = ((n / 5) as usize) % 6;
            let wisdom = match (goal, idx) {
                (HealthGoal::LowCalorie, 0) => match lang {
                    ChatLang::Ru => "💧 Шеф-коуч: Выпей стакан воды перед едой — съешь на 20% меньше. Мозг часто путает жажду с голодом.",
                    ChatLang::En => "💧 Chef Coach: Drink a glass of water before eating — you'll eat 20% less. The brain often confuses thirst with hunger.",
                    ChatLang::Pl => "💧 Szef-coach: Wypij szklankę wody przed jedzeniem — zjesz o 20% mniej.",
                    ChatLang::Uk => "💧 Шеф-коуч: Випий склянку води перед їжею — з'їси на 20% менше.",
                },
                (HealthGoal::LowCalorie, 1) => match lang {
                    ChatLang::Ru => "🍽️ Шеф-коуч: Ешь медленно — 20 минут нужно мозгу, чтобы почувствовать сытость. Жуй каждый кусок 15-20 раз.",
                    ChatLang::En => "🍽️ Chef Coach: Eat slowly — your brain needs 20 minutes to feel full. Chew each bite 15-20 times.",
                    ChatLang::Pl => "🍽️ Szef-coach: Jedz powoli — mózg potrzebuje 20 minut, żeby poczuć sytość.",
                    ChatLang::Uk => "🍽️ Шеф-коуч: Їж повільно — мозку потрібно 20 хвилин, щоб відчути ситість.",
                },
                (HealthGoal::LowCalorie, 2) => match lang {
                    ChatLang::Ru => "🌙 Шеф-коуч: Последний приём пищи — за 3 часа до сна. Не потому что «после 6 нельзя», а чтобы сон был крепким, а пищеварение — лёгким.",
                    ChatLang::En => "🌙 Chef Coach: Last meal — 3 hours before bed. Not because 'no food after 6pm' myth, but for better sleep and digestion.",
                    ChatLang::Pl => "🌙 Szef-coach: Ostatni posiłek — 3 godziny przed snem. Dla lepszego snu i trawienia.",
                    ChatLang::Uk => "🌙 Шеф-коуч: Останній прийом їжі — за 3 години до сну. Для кращого сну та травлення.",
                },
                (HealthGoal::HighProtein, 0) => match lang {
                    ChatLang::Ru => "⏰ Шеф-коуч: Белковое окно после тренировки — 30-60 минут. Но не паникуй: общий белок за день важнее точного тайминга.",
                    ChatLang::En => "⏰ Chef Coach: Protein window after workout: 30-60 min. But don't panic — total daily protein matters more than exact timing.",
                    ChatLang::Pl => "⏰ Szef-coach: Okno białkowe po treningu: 30-60 min. Ale spokojnie — dzienna suma białka jest ważniejsza.",
                    ChatLang::Uk => "⏰ Шеф-коуч: Білкове вікно після тренування — 30-60 хв. Але не панікуй: загальний білок за день важливіший.",
                },
                (HealthGoal::HighProtein, 1) => match lang {
                    ChatLang::Ru => "🥚 Шеф-коуч: Яйца — идеальный белок (биологическая ценность 100%). 3 яйца утром = 20г белка + витамин D + холин для мозга.",
                    ChatLang::En => "🥚 Chef Coach: Eggs — perfect protein (biological value 100%). 3 eggs = 20g protein + vitamin D + choline for brain.",
                    ChatLang::Pl => "🥚 Szef-coach: Jajka — idealne białko (wartość biologiczna 100%). 3 jajka = 20g białka + witamina D + cholina.",
                    ChatLang::Uk => "🥚 Шеф-коуч: Яйця — ідеальний білок (біологічна цінність 100%). 3 яйця = 20г білка + вітамін D + холін.",
                },
                (_, _) => match lang {
                    ChatLang::Ru => "🧠 Шеф-коуч: Знание — сила. Ты не просто ешь — ты принимаешь осознанные решения. Это и есть настоящий путь к здоровью.",
                    ChatLang::En => "🧠 Chef Coach: Knowledge is power. You're not just eating — you're making conscious decisions. That's the real path to health.",
                    ChatLang::Pl => "🧠 Szef-coach: Wiedza to siła. Nie tylko jesz — podejmujesz świadome decyzje. To prawdziwa droga do zdrowia.",
                    ChatLang::Uk => "🧠 Шеф-коуч: Знання — сила. Ти не просто їси — ти приймаєш усвідомлені рішення. Це і є шлях до здоров'я.",
                },
            };
            return Some(wisdom.to_string());
        },

        // Default — no message
        _ => return None,
    };

    Some(msg.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_turn_always_shows() {
        let ctx = SessionContext::new();
        let msg = pick_message(&ctx, HealthGoal::LowCalorie, ChatLang::Ru);
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("Шеф-коуч"));
    }

    #[test]
    fn turn_2_is_silent() {
        let mut ctx = SessionContext::new();
        ctx.turn_count = 2;
        let msg = pick_message(&ctx, HealthGoal::LowCalorie, ChatLang::Ru);
        assert!(msg.is_none());
    }

    #[test]
    fn turn_3_shows() {
        let mut ctx = SessionContext::new();
        ctx.turn_count = 3;
        let msg = pick_message(&ctx, HealthGoal::HighProtein, ChatLang::En);
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("Chef Coach"));
    }

    #[test]
    fn turn_20_shows() {
        let mut ctx = SessionContext::new();
        ctx.turn_count = 20;
        let msg = pick_message(&ctx, HealthGoal::LowCalorie, ChatLang::Ru);
        assert!(msg.is_some());
    }

    #[test]
    fn turn_14_silent() {
        let mut ctx = SessionContext::new();
        ctx.turn_count = 14;
        let msg = pick_message(&ctx, HealthGoal::LowCalorie, ChatLang::Ru);
        assert!(msg.is_none());
    }

    #[test]
    fn turn_15_silent() {
        let mut ctx = SessionContext::new();
        ctx.turn_count = 15;
        let msg = pick_message(&ctx, HealthGoal::LowCalorie, ChatLang::Ru);
        assert!(msg.is_none());
    }

    #[test]
    fn multilingual() {
        let ctx = SessionContext::new();
        let ru = pick_message(&ctx, HealthGoal::Balanced, ChatLang::Ru).unwrap();
        let en = pick_message(&ctx, HealthGoal::Balanced, ChatLang::En).unwrap();
        assert!(ru.contains("Шеф-коуч"));
        assert!(en.contains("Chef Coach"));
    }

    #[test]
    fn explored_count_in_turn_8() {
        let mut ctx = SessionContext::new();
        ctx.turn_count = 8;
        ctx.shown_slugs = vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into(), "f".into(), "g".into()];
        let msg = pick_message(&ctx, HealthGoal::LowCalorie, ChatLang::Ru).unwrap();
        assert!(msg.contains("более 6") || msg.contains("продуктов"));
    }
}
