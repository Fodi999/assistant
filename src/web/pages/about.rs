pub fn render() -> String {
    r#"
<section class="about-hero">
  <div class="chef-avatar-wrap"><i class="bi bi-person-fill" style="font-size:3.5rem;color:var(--accent)"></i></div>
  <h1 class="chef-name">Алексей Северов</h1>
  <p class="chef-title"><i class="bi bi-award"></i> Шеф-повар &bull; 15 лет практики</p>
</section>

<div class="about-body reveal">
  <p>Кулинарное путешествие началось во Флоренции, прошло через парижские бистро
  и вернулось домой — в Россию. Моя кухня — это честный продукт,
  минимум лишнего и максимум вкуса.</p>
  <p>Каждое блюдо — это диалог с сезоном и уважение к продукту. Никакой химии,
  никаких полуфабрикатов — только то, что выросло сегодня и будет приготовлено сегодня.</p>
</div>

<div class="stats-section reveal" style="margin:2.5rem -1.5rem">
  <div class="stats">
    <div class="stat">
      <span class="stat-num counter" data-target="15" data-suffix="">0</span>
      <span class="stat-label">лет опыта</span>
    </div>
    <div class="stat">
      <span class="stat-num counter" data-target="3" data-suffix="">0</span>
      <span class="stat-label">страны работы</span>
    </div>
    <div class="stat">
      <span class="stat-num counter" data-target="200" data-suffix="+">0</span>
      <span class="stat-label">авторских блюд</span>
    </div>
    <div class="stat">
      <span class="stat-num counter" data-target="8" data-suffix="">0</span>
      <span class="stat-label">лет мишлен</span>
    </div>
  </div>
</div>

<div class="section-title reveal" style="margin-top:3rem">
  <span class="section-eyebrow">Путь</span>
  <h2>Кулинарная история</h2>
  <div class="accent-line"></div>
</div>

<div class="timeline reveal" style="margin-bottom:2rem">
  <div class="timeline-item">
    <div class="timeline-icon"><i class="bi bi-geo-alt"></i></div>
    <div class="timeline-body">
      <span class="timeline-year">2008 — 2012</span>
      <h4>Флоренция, Италия</h4>
      <p>Кулинарная академия Cordon Bleu, стажировка в ресторане 1&nbsp;★&nbsp;Michelin</p>
    </div>
  </div>
  <div class="timeline-item">
    <div class="timeline-icon"><i class="bi bi-geo-alt"></i></div>
    <div class="timeline-body">
      <span class="timeline-year">2012 — 2017</span>
      <h4>Париж, Франция</h4>
      <p>Су-шеф в bistrot gastronomique, разработка сезонных меню</p>
    </div>
  </div>
  <div class="timeline-item">
    <div class="timeline-icon"><i class="bi bi-geo-alt"></i></div>
    <div class="timeline-body">
      <span class="timeline-year">2017 — сейчас</span>
      <h4>Москва, Россия</h4>
      <p>Авторский ресторан, гастрономические ужины, мастер-классы</p>
    </div>
  </div>
</div>
"#.to_string()
}
