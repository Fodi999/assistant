use crate::web::language;

pub fn render(lang: language::Lang, slug: &str, state: Option<&str>) -> String {
    let (
        eyebrow,
        title,
        subtitle,
        back,
        loading,
        error,
        no_data,
        states_title,
        nutrition_title,
        minerals_title,
        vitamins_title,
        culinary_title,
        properties_title,
        health_title,
        sugar_title,
        processing_title,
        behavior_title,
        measures_title,
        calculators_title,
        nutrition_calc_title,
        measure_calc_title,
        amount_label,
        unit_label,
        convert_label,
        result_label,
        season_title,
        pairings_title,
    ) = match lang {
        language::Lang::Pl => (
            "Baza składników",
            "Profil składnika",
            "Pełne dane z bazy: wartości odżywcze, profile zdrowotne i zastosowanie kulinarne.",
            "Powrót do katalogu",
            "Ładujemy pełny profil składnika...",
            "Nie udało się załadować danych składnika.",
            "Brak danych dla tego składnika.",
            "Stany obróbki",
            "Wartości odżywcze na 100 g",
            "Minerały na 100 g",
            "Witaminy na 100 g",
            "Profil kulinarny",
            "Właściwości produktu",
            "Profil zdrowotny",
            "Profil cukrowy",
            "Wpływ obróbki",
            "Zachowanie w kuchni",
            "Miary kuchenne",
            "Kalkulatory",
            "Przelicznik wartości odżywczych",
            "Konwerter miar",
            "Ilość",
            "Jednostka",
            "Przelicz",
            "Wynik",
            "Sezon",
            "Najlepsze połączenia",
        ),
        language::Lang::Ru => (
            "База ингредиентов",
            "Профиль ингредиента",
            "Полные данные из базы: пищевая ценность, профили здоровья и кулинарное применение.",
            "Назад в каталог",
            "Загружаем полный профиль ингредиента...",
            "Не удалось загрузить данные ингредиента.",
            "Нет данных по этому ингредиенту.",
            "Состояния обработки",
            "Пищевая ценность на 100 г",
            "Минералы на 100 г",
            "Витамины на 100 г",
            "Кулинарный профиль",
            "Свойства продукта",
            "Профиль здоровья",
            "Сахарный профиль",
            "Влияние обработки",
            "Поведение в кулинарии",
            "Кухонные меры",
            "Калькуляторы",
            "Калькулятор нутриентов",
            "Конвертер мер",
            "Количество",
            "Единица",
            "Посчитать",
            "Результат",
            "Сезон",
            "Лучшие сочетания",
        ),
        language::Lang::En => (
            "Ingredient database",
            "Ingredient profile",
            "Full database facts: nutrition, health profile and culinary behavior.",
            "Back to catalog",
            "Loading full ingredient profile...",
            "Could not load ingredient data.",
            "No data available for this ingredient.",
            "Processing states",
            "Nutrition per 100 g",
            "Minerals per 100 g",
            "Vitamins per 100 g",
            "Culinary profile",
            "Product properties",
            "Health profile",
            "Sugar profile",
            "Processing effects",
            "Culinary behavior",
            "Kitchen measures",
            "Calculators",
            "Nutrition calculator",
            "Measure converter",
            "Amount",
            "Unit",
            "Calculate",
            "Result",
            "Season",
            "Best pairings",
        ),
    };

    format!(
        r#"<div class="container ingredient-detail-page" data-ingredient-detail-page data-slug="{slug}" data-state="{state}" data-loading="{loading}" data-error="{error}" data-empty="{no_data}">
  <section class="page-header ingredient-detail-hero">
    <span class="section-eyebrow"><i class="bi bi-journal-medical"></i> {eyebrow}</span>
    <h1 data-ingredient-title>{title}</h1>
    <p class="page-header-sub" data-ingredient-subtitle>{subtitle}</p>
  </section>

  <section class="ingredient-detail-actions reveal">
    <a href="/ingredient-catalog" class="btn btn-ghost btn-sm"><i class="bi bi-arrow-left"></i> {back}</a>
  </section>

  <section class="ingredient-detail-content reveal" data-ingredient-detail-content>
    <div class="ingredient-state">{loading}</div>
  </section>

  <template data-ingredient-detail-template>
    <article class="ingredient-detail-top">
      <div class="ingredient-detail-photo" data-detail-image></div>
      <div class="ingredient-detail-summary">
        <h2 data-detail-name></h2>
        <p class="ingredient-detail-lead" data-detail-description></p>
        <div class="ingredient-flag-list" data-detail-flags></div>
        <div class="ingredient-season-list"><strong>{season_title}:</strong> <span data-detail-seasons></span></div>
      </div>
    </article>

    <section class="ingredient-detail-section">
      <h3>{states_title}</h3>
      <div class="ingredient-state-chips" data-detail-states></div>
      <div class="ingredient-state-card" data-detail-state-card></div>
    </section>

    <section class="ingredient-detail-grid">
      <article class="ingredient-detail-section"><h3>{nutrition_title}</h3><div data-detail-macros></div></article>
      <article class="ingredient-detail-section"><h3>{minerals_title}</h3><div data-detail-minerals></div></article>
      <article class="ingredient-detail-section"><h3>{vitamins_title}</h3><div data-detail-vitamins></div></article>
      <article class="ingredient-detail-section"><h3>{culinary_title}</h3><div data-detail-culinary></div></article>
      <article class="ingredient-detail-section"><h3>{properties_title}</h3><div data-detail-properties></div></article>
      <article class="ingredient-detail-section"><h3>{health_title}</h3><div data-detail-health></div></article>
      <article class="ingredient-detail-section"><h3>{sugar_title}</h3><div data-detail-sugar></div></article>
      <article class="ingredient-detail-section"><h3>{processing_title}</h3><div data-detail-processing></div></article>
      <article class="ingredient-detail-section"><h3>{behavior_title}</h3><div data-detail-behavior></div></article>
      <article class="ingredient-detail-section"><h3>{measures_title}</h3><div data-detail-measures></div></article>
    </section>

    <section class="ingredient-detail-section ingredient-calculators-section">
      <h3>{calculators_title}</h3>
      <div class="ingredient-calculator-grid">
        <article class="ingredient-calculator-card" data-nutrition-calculator>
          <h4>{nutrition_calc_title}</h4>
          <div class="ingredient-calculator-form">
            <label>
              <span>{amount_label}</span>
              <input type="number" step="0.1" min="0" value="100" data-nutrition-amount>
            </label>
            <label>
              <span>{unit_label}</span>
              <select data-nutrition-unit>
                <option value="g">г — граммы</option>
                <option value="kg">кг — килограммы</option>
                <option value="ml">мл — миллилитры</option>
                <option value="cup">стакан</option>
                <option value="tbsp">столовая ложка</option>
                <option value="tsp">чайная ложка</option>
              </select>
            </label>
            <button type="button" class="btn btn-primary btn-sm" data-nutrition-run>{convert_label}</button>
          </div>
          <div class="ingredient-calculator-result" data-nutrition-result>{result_label}</div>
        </article>

        <article class="ingredient-calculator-card" data-measure-calculator>
          <h4>{measure_calc_title}</h4>
          <div class="ingredient-calculator-form ingredient-measure-form">
            <label>
              <span>{amount_label}</span>
              <input type="number" step="0.1" min="0" value="1" data-measure-amount>
            </label>
            <label>
              <span>Из единицы</span>
              <select data-measure-unit>
                <option value="cup">стакан</option>
                <option value="tbsp">столовая ложка</option>
                <option value="tsp">чайная ложка</option>
                <option value="ml">миллилитры</option>
                <option value="g">граммы</option>
                <option value="kg">килограммы</option>
              </select>
            </label>
            <label>
              <span>В единицу</span>
              <select data-measure-to-unit>
                <option value="g">граммы</option>
                <option value="kg">килограммы</option>
                <option value="ml">миллилитры</option>
                <option value="cup">стаканы</option>
                <option value="tbsp">столовые ложки</option>
                <option value="tsp">чайные ложки</option>
              </select>
            </label>
            <button type="button" class="btn btn-primary btn-sm" data-measure-run>{convert_label}</button>
          </div>
          <div class="ingredient-calculator-result" data-measure-result>{result_label}</div>
        </article>
      </div>
    </section>

    <section class="ingredient-detail-section">
      <h3>{pairings_title}</h3>
      <div class="ingredient-pairings" data-detail-pairings></div>
    </section>
  </template>
</div>"#,
        slug = slug,
        state = state.unwrap_or(""),
    )
}
