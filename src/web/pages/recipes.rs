pub struct Recipe {
    pub id:          &'static str,
    pub name:        &'static str,
    pub icon:        &'static str,
    pub time:        &'static str,
    pub difficulty:  &'static str,
    pub ingredients: &'static [(&'static str, &'static str)],
    pub steps:       &'static [&'static str],
}

pub static RECIPES: &[Recipe] = &[
    Recipe {
        id: "borsch",
        name: "Борщ по-киевски",
        icon: "bi-cup-hot",
        time: "90 мин",
        difficulty: "Средний",
        ingredients: &[
            ("Говяжья грудинка", "500 г"),
            ("Свёкла", "2 шт"),
            ("Капуста белокочанная", "300 г"),
            ("Картофель", "3 шт"),
            ("Морковь", "1 шт"),
            ("Томатная паста", "2 ст.л."),
        ],
        steps: &[
            "Сварить бульон из грудинки на медленном огне — 1 час.",
            "Свёклу натереть на крупной тёрке, потушить с уксусом 10 мин.",
            "Добавить нарезанный картофель и капусту в бульон.",
            "Ввести зажарку (лук, морковь, томатная паста) и свёклу, варить 15 мин.",
            "Дать настояться 20 мин, подавать со сметаной и зеленью.",
        ],
    },
    Recipe {
        id: "tiramisu",
        name: "Тирамису классический",
        icon: "bi-cake2",
        time: "30 мин + 4 ч охлаждения",
        difficulty: "Лёгкий",
        ingredients: &[
            ("Маскарпоне", "500 г"),
            ("Яйца", "4 шт"),
            ("Сахар", "100 г"),
            ("Печенье савоярди", "300 г"),
            ("Эспрессо крепкий", "200 мл"),
            ("Какао для посыпки", "2 ст.л."),
        ],
        steps: &[
            "Желтки взбить с сахаром до пышной светлой массы.",
            "Ввести маскарпоне, аккуратно перемешать лопаткой.",
            "Взбить белки до устойчивых пиков, вмешать в крем.",
            "Печенье обмакнуть в остывший эспрессо — по 2 секунды каждое.",
            "Выложить слоями: печенье → крем → печенье → крем.",
            "Убрать в холодильник минимум на 4 часа, перед подачей посыпать какао.",
        ],
    },
    Recipe {
        id: "risotto",
        name: "Ризотто с белыми грибами",
        icon: "bi-fire",
        time: "35 мин",
        difficulty: "Средний",
        ingredients: &[
            ("Рис Карнероли", "300 г"),
            ("Белые грибы", "200 г"),
            ("Маскарпоне", "100 г"),
            ("Пармезан тёртый", "80 г"),
            ("Белое сухое вино", "150 мл"),
            ("Горячий куриный бульон", "1 л"),
        ],
        steps: &[
            "Обжарить грибы на сливочном масле до золотистого цвета, отложить.",
            "Пассеровать лук-шалот, добавить рис, перемешивать 2 мин.",
            "Влить вино, выпарить почти полностью.",
            "Добавлять бульон по одному половнику, постоянно помешивая.",
            "На последнем бульоне ввести грибы, маскарпоне, пармезан.",
            "Снять с огня, накрыть на 2 мин, подавать немедленно.",
        ],
    },
];

pub fn list() -> String {
    let cards: String = RECIPES.iter().map(|r| format!(
        r#"<a href="/recipes/{}" class="recipe-card reveal">
  <div class="recipe-card-icon"><i class="bi {}"></i></div>
  <h3>{}</h3>
  <p><i class="bi bi-clock"></i> {} &bull; <i class="bi bi-bar-chart"></i> {}</p>
</a>"#,
        r.id, r.icon, r.name, r.time, r.difficulty
    )).collect();
    format!(r#"<section class="page-header"><h1>Рецепты</h1><p>Проверенные техники и авторские секреты</p></section><div class="recipes-grid">{}</div>"#, cards)
}

pub fn detail(id: &str) -> String {
    let r = RECIPES.iter().find(|r| r.id == id);
    match r {
        None => r#"<div class="not-found"><h1>404</h1><p>Рецепт не найден</p><a href="/recipes" class="btn"><i class="bi bi-arrow-left"></i> Назад</a></div>"#.to_string(),
        Some(r) => {
            let ings: String = r.ingredients.iter().map(|(n,q)| format!(
                "<li><i class=\"bi bi-dot\"></i><strong>{}</strong>&ensp;{}</li>", n, q
            )).collect();
            let steps: String = r.steps.iter().enumerate().map(|(i,s)| format!(
                "<li><span class=\"step-num\">{}</span>{}</li>", i+1, s
            )).collect();
            format!(r#"<a href="/recipes" class="back-link"><i class="bi bi-arrow-left"></i> Все рецепты</a>
<section class="page-header">
  <div style="font-size:2.5rem;color:var(--accent);margin-bottom:.5rem"><i class="bi {}"></i></div>
  <h1>{}</h1>
  <p><i class="bi bi-clock"></i> {} &bull; <i class="bi bi-bar-chart"></i> {}</p>
</section>
<section class="recipe-detail">
  <div class="ingredients"><h2><i class="bi bi-list-check"></i> Ингредиенты</h2><ul>{}</ul></div>
  <div class="steps"><h2><i class="bi bi-card-list"></i> Приготовление</h2><ol>{}</ol></div>
</section>"#, r.icon, r.name, r.time, r.difficulty, ings, steps)
        }
    }
}
