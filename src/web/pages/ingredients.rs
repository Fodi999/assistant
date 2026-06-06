use crate::web::language;

pub fn render(lang: language::Lang) -> String {
    let (eyebrow, title, subtitle, note, loading, empty, error, count_label, back, search, all) =
        match lang {
            language::Lang::Pl => (
                "Katalog składników",
                "Katalog składników",
        "Produkty z wartością odżywczą i zdjęciami.",
        "Baza składników kuchni: zdjęcia, makro, energia, sezonowość i krótkie notatki technologiczne.",
                "Ładujemy składniki z bazy...",
                "Nie znaleziono składników.",
                "Nie udało się załadować katalogu.",
                "produktów",
                "O szefie",
                "Szukaj składnika",
                "Wszystkie kategorie",
            ),
            language::Lang::Ru => (
                "Каталог ингредиентов",
                "Каталог ингредиентов",
        "Продукты с пищевой ценностью и фото.",
        "База ингредиентов кухни: фото, макро, энергия, сезонность и короткие технологические заметки.",
                "Загружаем ингредиенты из базы...",
                "Ингредиенты не найдены.",
                "Не удалось загрузить каталог.",
                "продуктов",
                "О шефе",
                "Поиск ингредиента",
                "Все категории",
            ),
            language::Lang::En => (
                "Ingredient catalog",
                "Ingredient catalog",
        "Products with nutrition values and photos.",
        "Kitchen ingredient database: photos, macros, energy, seasonality and short technology notes.",
                "Loading ingredients from the database...",
                "No ingredients found.",
                "Could not load the catalog.",
                "products",
                "About",
                "Search ingredient",
                "All categories",
            ),
        };

    format!(
        r#"<div class="container ingredient-page" data-ingredient-page data-loading="{loading}" data-empty="{empty}" data-error="{error}" data-count-label="{count_label}" data-all-label="{all}">
  <section class="page-header ingredient-hero">
    <span class="section-eyebrow"><i class="bi bi-journal-richtext"></i> {eyebrow}</span>
    <h1>{title}</h1>
    <p class="page-header-sub">{subtitle}</p>
  </section>

  <section class="ingredient-toolbar reveal">
    <div><strong data-ingredient-count>0 {count_label}</strong><span>{note}</span></div>
    <a href="/about" class="btn btn-ghost btn-sm"><i class="bi bi-person-badge"></i> {back}</a>
  </section>

  <section class="ingredient-controls reveal">
    <label class="ingredient-search">
      <i class="bi bi-search"></i>
      <input type="search" data-ingredient-search placeholder="{search}" autocomplete="off">
    </label>
    <select data-ingredient-category aria-label="{all}">
      <option value="">{all}</option>
    </select>
  </section>

  <section class="ingredient-grid reveal" data-ingredient-grid>
    <div class="ingredient-state">{loading}</div>
  </section>
</div>"#
    )
}
